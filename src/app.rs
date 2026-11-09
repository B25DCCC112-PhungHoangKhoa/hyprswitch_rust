use gtk4::gdk::Key;
use gtk4::gio::prelude::*;
use gtk4::gio::{ApplicationFlags, ApplicationHoldGuard};
use gtk4::glib::{self, Propagation};
use gtk4::prelude::*;
use gtk4::{Application, EventControllerKey, PropagationPhase};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::monitors::{DisplayPreset, MonitorManager};
use crate::theme::apply_theme;
use crate::ui::SwitcherWindow;

const AUTO_DISMISS_MS: u64 = 8000;

#[derive(Clone)]
pub struct HyprswitchApp {
    app: Application,
    monitor_manager: Rc<MonitorManager>,
    selected_mode: Rc<Cell<DisplayPreset>>,
    window: Rc<RefCell<Option<Rc<SwitcherWindow>>>>,
    timer_id: Rc<RefCell<Option<glib::SourceId>>>,
    hold_guard: Rc<RefCell<Option<ApplicationHoldGuard>>>,
}

impl HyprswitchApp {
    pub fn new() -> Self {
        let app = Application::builder()
            .application_id("org.hyprland.hyprswitch")
            .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();

        let this = Self {
            app,
            monitor_manager: Rc::new(MonitorManager::default()),
            selected_mode: Rc::new(Cell::new(DisplayPreset::PcOnly)),
            window: Rc::new(RefCell::new(None)),
            timer_id: Rc::new(RefCell::new(None)),
            hold_guard: Rc::new(RefCell::new(None)),
        };

        this.setup_signals();
        this
    }

    fn setup_signals(&self) {
        let this = self.clone();
        self.app.connect_startup(|_| {
            apply_theme();
        });

        let this_cmd = this.clone();
        self.app.connect_command_line(move |app, cmdline| {
            let args: Vec<String> = cmdline
                .arguments()
                .into_iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();

            // CLI: --status
            if args.iter().any(|a| a == "--status") {
                this_cmd.print_status();
                return 0.into();
            }

            // CLI: --apply <mode>
            for (i, arg) in args.iter().enumerate() {
                if (arg == "--apply" || arg == "-a") && i + 1 < args.len() {
                    let mode_str = &args[i + 1];
                    if let Some(preset) = DisplayPreset::from_str_name(mode_str) {
                        println!("Applying mode: {}", preset.name());
                        if let Err(e) = this_cmd.monitor_manager.apply_mode(preset) {
                            eprintln!("Error: {}", e);
                            return 1.into();
                        }
                        return 0.into();
                    } else {
                        eprintln!("Unknown mode '{}'. Valid: pc, duplicate, extend, second", mode_str);
                        return 1.into();
                    }
                }
            }

            // Interactive OSD: Super + P
            if this_cmd.window.borrow().is_none() {
                if this_cmd.hold_guard.borrow().is_none() {
                    let guard = app.hold();
                    *this_cmd.hold_guard.borrow_mut() = Some(guard);
                }

                let curr_mode = this_cmd.monitor_manager.detect_current_mode();
                this_cmd.selected_mode.set(curr_mode);
                this_cmd.open_window(curr_mode);
            } else {
                // Secondary instance cycling
                this_cmd.cycle_forward();
            }

            0.into()
        });
    }

    fn print_status(&self) {
        let (internal, external) = self.monitor_manager.get_displays();
        let current_mode = self.monitor_manager.detect_current_mode();

        println!("=== Hyprland Monitors ===");
        if let Some(int) = internal {
            let int_res = match int.best_mode() {
                Some(b) => format!("{}x{}@{:.2}Hz", b.width, b.height, b.refresh_rate),
                None => "Unknown".to_string(),
            };
            println!("Internal Display: {} (Active: {}, Best: {})", int.name, !int.disabled, int_res);
        } else {
            println!("Internal Display: None");
        }

        if let Some(ext) = external {
            let ext_res = match ext.best_mode() {
                Some(b) => format!("{}x{}@{:.2}Hz", b.width, b.height, b.refresh_rate),
                None => "Unknown".to_string(),
            };
            println!("External Display: {} (Active: {}, Best: {})", ext.name, !ext.disabled, ext_res);
            if !ext.available_modes.is_empty() {
                let modes_str: Vec<String> = ext.available_modes.iter().take(5).map(|m| m.mode_str.clone()).collect();
                println!("Available External Modes: {}...", modes_str.join(", "));
            }
        } else {
            println!("External Display: None detected");
        }

        println!("Current Mode: {}", current_mode.name());
    }

    fn open_window(&self, initial_mode: DisplayPreset) {
        let (internal, external) = self.monitor_manager.get_displays();

        let this_card = self.clone();
        let window = SwitcherWindow::new(
            &self.app,
            &internal,
            &external,
            initial_mode,
            move |preset| {
                this_card.selected_mode.set(preset);
                this_card.apply_and_quit();
            },
        );

        // Key Controller with CAPTURE phase
        let key_ctrl = EventControllerKey::new();
        key_ctrl.set_propagation_phase(PropagationPhase::Capture);

        let this_key = self.clone();
        key_ctrl.connect_key_pressed(move |_, key, _keycode, _state| {
            match key {
                Key::Return | Key::KP_Enter | Key::space => {
                    this_key.apply_and_quit();
                    Propagation::Stop
                }
                Key::Escape => {
                    this_key.close_and_quit();
                    Propagation::Stop
                }
                Key::Right | Key::Down | Key::Tab | Key::p | Key::P => {
                    this_key.cycle_forward();
                    Propagation::Stop
                }
                Key::Left | Key::Up | Key::ISO_Left_Tab => {
                    this_key.cycle_backward();
                    Propagation::Stop
                }
                _ => Propagation::Proceed,
            }
        });

        window.window.add_controller(key_ctrl);
        window.window.present();

        *self.window.borrow_mut() = Some(window);
        self.reset_dismiss_timer();
    }

    fn reset_dismiss_timer(&self) {
        if let Some(src) = self.timer_id.borrow_mut().take() {
            src.remove();
        }

        let this = self.clone();
        let src = glib::timeout_add_local(Duration::from_millis(AUTO_DISMISS_MS), move || {
            this.close_and_quit();
            glib::ControlFlow::Break
        });

        *self.timer_id.borrow_mut() = Some(src);
    }

    pub fn cycle_forward(&self) {
        let next = DisplayPreset::from_index(self.selected_mode.get().index() + 1);
        self.selected_mode.set(next);
        if let Some(w) = self.window.borrow().as_ref() {
            w.set_highlighted_mode(next);
        }
        self.reset_dismiss_timer();
    }

    pub fn cycle_backward(&self) {
        let prev = DisplayPreset::from_index(self.selected_mode.get().index() + 3);
        self.selected_mode.set(prev);
        if let Some(w) = self.window.borrow().as_ref() {
            w.set_highlighted_mode(prev);
        }
        self.reset_dismiss_timer();
    }

    pub fn apply_and_quit(&self) {
        let preset = self.selected_mode.get();
        if let Err(e) = self.monitor_manager.apply_mode(preset) {
            eprintln!("Error applying mode: {}", e);
        }
        self.close_and_quit();
    }

    pub fn close_and_quit(&self) {
        if let Some(src) = self.timer_id.borrow_mut().take() {
            src.remove();
        }

        if let Some(w) = self.window.borrow_mut().take() {
            w.window.destroy();
        }

        self.hold_guard.borrow_mut().take();
        self.app.quit();
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}
