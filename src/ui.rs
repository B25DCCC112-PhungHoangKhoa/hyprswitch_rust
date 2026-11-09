use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box, Button, Image, Justification, Label, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::rc::Rc;
use crate::monitors::{DisplayPreset, MonitorInfo};

pub struct ModeCardWidget {
    pub button: Button,
    pub preset: DisplayPreset,
}

impl ModeCardWidget {
    pub fn new<F: Fn(DisplayPreset) + 'static>(
        preset: DisplayPreset,
        title: &str,
        subtitle: &str,
        icon_name: &str,
        on_clicked: F,
    ) -> Self {
        let button = Button::new();
        button.add_css_class("mode-card");

        let vbox = Box::new(Orientation::Vertical, 4);
        button.set_child(Some(&vbox));

        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(44);
        icon.add_css_class("mode-icon");
        vbox.append(&icon);

        let title_label = Label::new(Some(title));
        title_label.add_css_class("mode-name");
        title_label.set_justify(Justification::Center);
        vbox.append(&title_label);

        let desc_label = Label::new(Some(subtitle));
        desc_label.add_css_class("mode-desc");
        desc_label.set_wrap(true);
        desc_label.set_max_width_chars(18);
        desc_label.set_justify(Justification::Center);
        vbox.append(&desc_label);

        button.connect_clicked(move |_| {
            on_clicked(preset);
        });

        Self { button, preset }
    }

    pub fn set_selected(&self, selected: bool) {
        if selected {
            self.button.add_css_class("selected");
        } else {
            self.button.remove_css_class("selected");
        }
    }
}

pub struct SwitcherWindow {
    pub window: ApplicationWindow,
    pub cards: Vec<ModeCardWidget>,
    pub footer_label: Label,
    #[allow(dead_code)]
    pub status_label: Label,
}

impl SwitcherWindow {
    pub fn new<F: Fn(DisplayPreset) + 'static>(
        app: &Application,
        internal_mon: &Option<MonitorInfo>,
        external_mon: &Option<MonitorInfo>,
        initial_mode: DisplayPreset,
        on_mode_selected: F,
    ) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Display Projection")
            .build();

        window.add_css_class("switcher-window");

        // Setup Layer Shell
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace(Some("hyprswitch"));
        window.set_keyboard_mode(KeyboardMode::Exclusive);

        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, false);
        }

        let main_box = Box::new(Orientation::Vertical, 0);
        main_box.add_css_class("main-container");
        window.set_child(Some(&main_box));

        // Header Title
        let title = Label::new(Some("Display Projection"));
        title.add_css_class("title-label");
        title.set_halign(Align::Center);
        main_box.append(&title);

        // Status Pill
        let status_box = Box::new(Orientation::Horizontal, 0);
        status_box.add_css_class("status-pill");
        status_box.set_halign(Align::Center);

        let status_text = Self::format_status_text(internal_mon, external_mon);
        let status_label = Label::new(Some(&status_text));
        status_label.add_css_class("status-pill-text");
        status_box.append(&status_label);
        main_box.append(&status_box);

        // Cards horizontal box
        let cards_box = Box::new(Orientation::Horizontal, 14);
        cards_box.add_css_class("cards-box");
        cards_box.set_halign(Align::Center);
        main_box.append(&cards_box);

        let on_mode_selected = Rc::new(on_mode_selected);
        let mut cards = Vec::new();

        for &preset in &DisplayPreset::ALL {
            let sub = Self::get_preset_subtext(preset, internal_mon, external_mon);
            let cb = on_mode_selected.clone();
            let card = ModeCardWidget::new(
                preset,
                preset.name(),
                &sub,
                preset.icon_name(),
                move |p| cb(p),
            );
            cards_box.append(&card.button);
            cards.push(card);
        }

        // Footer Pill
        let footer_box = Box::new(Orientation::Horizontal, 0);
        footer_box.add_css_class("footer-pill");
        footer_box.set_halign(Align::Center);

        let footer_label = Label::new(Some("Enter to select   •   Win+P to cycle   •   Esc to cancel"));
        footer_label.add_css_class("footer-label");
        footer_box.append(&footer_label);
        main_box.append(&footer_box);

        let switcher = Rc::new(Self {
            window,
            cards,
            footer_label,
            status_label,
        });

        switcher.set_highlighted_mode(initial_mode);
        switcher
    }

    fn format_status_text(internal_mon: &Option<MonitorInfo>, external_mon: &Option<MonitorInfo>) -> String {
        let int_desc = match internal_mon {
            Some(m) => format!("Internal: {} ({}x{}@{:.0}Hz)", m.name, m.width, m.height, m.refresh_rate),
            None => "Internal: None".to_string(),
        };

        let ext_desc = match external_mon {
            Some(m) => {
                let ext_mode = match m.best_mode() {
                    Some(best) => format!("{}x{}@{:.0}Hz", best.width, best.height, best.refresh_rate),
                    None => "Auto".to_string(),
                };
                format!("External: {} ({})", m.name, ext_mode)
            }
            None => "External: No external display detected".to_string(),
        };

        format!("{}   |   {}", int_desc, ext_desc)
    }

    fn get_preset_subtext(preset: DisplayPreset, internal: &Option<MonitorInfo>, external: &Option<MonitorInfo>) -> String {
        let int_name = internal.as_ref().map(|m| m.name.as_str()).unwrap_or("Built-in");
        let ext_name = external.as_ref().map(|m| m.name.as_str()).unwrap_or("External");

        match preset {
            DisplayPreset::PcOnly => format!("Built-in ({}) only", int_name),
            DisplayPreset::Duplicate => format!("Mirror {} to {}", int_name, ext_name),
            DisplayPreset::Extend => {
                if let Some(ext) = external {
                    if let Some(best) = ext.best_mode() {
                        return format!("Dual Screen ({}x{}@{:.0}Hz)", best.width, best.height, best.refresh_rate);
                    }
                }
                "Extend side-by-side".to_string()
            }
            DisplayPreset::SecondOnly => format!("{} only", ext_name),
        }
    }

    pub fn set_highlighted_mode(&self, preset: DisplayPreset) {
        for card in &self.cards {
            let is_sel = card.preset == preset;
            card.set_selected(is_sel);
            if is_sel {
                card.button.grab_focus();
            }
        }
        self.footer_label.set_text(&format!(
            "Enter to select '{}'   •   Win+P to cycle   •   Esc to cancel",
            preset.name()
        ));
    }
}
