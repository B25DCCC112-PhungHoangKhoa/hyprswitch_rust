mod app;
mod monitors;
mod theme;
mod ui;

use clap::Parser;
use gtk4::glib::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "hyprswitch_rust",
    about = "Display projection switcher for Hyprland with Caelestia aesthetic (Rust Edition)",
    version
)]
struct Cli {
    /// Print connected monitor detection & current mode
    #[arg(short, long)]
    status: bool,

    /// Apply mode directly without UI (pc, duplicate, extend, second)
    #[arg(short, long)]
    apply: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    // Fast-path for CLI queries (instant execution without GTK loop)
    if args.iter().any(|a| a == "--status" || a == "-s") {
        let mm = monitors::MonitorManager::default();
        let (internal, external) = mm.get_displays();
        let current_mode = mm.detect_current_mode();

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
        return ExitCode::SUCCESS;
    }

    for (i, arg) in args.iter().enumerate() {
        if (arg == "--apply" || arg == "-a") && i + 1 < args.len() {
            let mode_str = &args[i + 1];
            if let Some(preset) = monitors::DisplayPreset::from_str_name(mode_str) {
                println!("Applying mode: {}", preset.name());
                let mm = monitors::MonitorManager::default();
                if let Err(e) = mm.apply_mode(preset) {
                    eprintln!("Error applying mode: {}", e);
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            } else {
                eprintln!("Unknown mode '{}'. Valid: pc, duplicate, extend, second", mode_str);
                return ExitCode::FAILURE;
            }
        }
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        let _ = Cli::parse();
        return ExitCode::SUCCESS;
    }

    let app = app::HyprswitchApp::new();
    app.run()
}
