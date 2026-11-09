
use gtk4::CssProvider;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn default_colors() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("background", "130c0e");
    m.insert("surface", "130c0e");
    m.insert("surfaceContainerLowest", "000000");
    m.insert("surfaceContainerLow", "1a1113");
    m.insert("surfaceContainer", "211719");
    m.insert("surfaceContainerHigh", "281d1f");
    m.insert("surfaceContainerHighest", "2f2225");
    m.insert("surfaceBright", "36292b");
    m.insert("onSurface", "f7e0e3");
    m.insert("onSurfaceVariant", "baa6a9");
    m.insert("outline", "837174");
    m.insert("outlineVariant", "534447");
    m.insert("primary", "f5b6c3");
    m.insert("onPrimary", "5f333e");
    m.insert("primaryContainer", "744550");
    m.insert("onPrimaryContainer", "ffdae0");
    m.insert("secondary", "e4bdc4");
    m.insert("onSecondary", "54393e");
    m.insert("secondaryContainer", "4f343a");
    m.insert("onSecondaryContainer", "dcb6bc");
    m.insert("error", "fa746f");
    m.insert("onError", "490006");
    m.insert("errorContainer", "871f21");
    m.insert("onErrorContainer", "ff9993");
    m
}

pub fn load_caelestia_colors() -> HashMap<String, String> {
    let mut colors: HashMap<String, String> = default_colors()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let scheme_json = PathBuf::from(&home).join(".local/state/caelestia/scheme.json");
    let scheme_lua = PathBuf::from(&home).join(".config/hypr/scheme/current.lua");

    // 1. Try ~/.local/state/caelestia/scheme.json
    if scheme_json.exists() {
        if let Ok(content) = fs::read_to_string(&scheme_json) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                if let Some(obj) = val.get("colours").and_then(|v| v.as_object()) {
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            let clean = s.trim_start_matches('#');
                            colors.insert(k.clone(), clean.to_string());
                        }
                    }
                    return colors;
                }
            }
        }
    }

    // 2. Try ~/.config/hypr/scheme/current.lua
    if scheme_lua.exists() {
        if let Ok(content) = fs::read_to_string(&scheme_lua) {
            if let Ok(re) = Regex::new(r#"(\w+)\s*=\s*["']#?([a-fA-F0-9]+)["']"#) {
                for caps in re.captures_iter(&content) {
                    if let (Some(k), Some(v)) = (caps.get(1), caps.get(2)) {
                        colors.insert(k.as_str().to_string(), v.as_str().to_string());
                    }
                }
            }
        }
    }

    colors
}

pub fn get_caelestia_css() -> String {
    let colors = load_caelestia_colors();

    let get_color = |k: &str| -> String {
        colors.get(k).cloned().unwrap_or_else(|| "f5b6c3".to_string())
    };

    let surface_container = get_color("surfaceContainer");
    let surface_container_high = get_color("surfaceContainerHigh");
    let surface_container_highest = get_color("surfaceContainerHighest");
    let primary = get_color("primary");
    let primary_container = get_color("primaryContainer");
    let on_primary = get_color("onPrimary");
    let on_primary_container = get_color("onPrimaryContainer");
    let on_surface = get_color("onSurface");
    let on_surface_variant = get_color("onSurfaceVariant");
    let outline_variant = get_color("outlineVariant");

    format!(
r#"
window.switcher-window {{
    background-color: transparent;
    font-family: "Rubik", "GoogleSansFlex", "Google Sans", "Noto Sans", sans-serif;
}}

.main-container {{
    background-color: alpha(#{surface_container}, 0.72);
    border: 1px solid alpha(#{outline_variant}, 0.28);
    border-radius: 32px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.65);
    padding: 24px 30px;
    margin: 20px;
}}

.title-label {{
    font-size: 19px;
    font-weight: 600;
    color: #{on_surface};
    letter-spacing: 0.2px;
    margin-bottom: 8px;
}}

.status-pill {{
    background-color: alpha(#{surface_container_highest}, 0.6);
    border: 1px solid alpha(#{outline_variant}, 0.2);
    border-radius: 9999px;
    padding: 5px 18px;
    margin-bottom: 20px;
}}

.status-pill-text {{
    font-size: 12px;
    font-weight: 500;
    color: #{on_surface_variant};
}}

.cards-box {{
    margin-bottom: 20px;
}}

button.mode-card {{
    background-color: alpha(#{surface_container_high}, 0.65);
    background-image: none;
    border: 1.5px solid alpha(#{outline_variant}, 0.2);
    border-radius: 24px;
    padding: 16px 16px 18px 16px;
    min-width: 164px;
    min-height: 156px;
    transition: all 180ms ease-in-out;
    outline: none;
    box-shadow: none;
}}

button.mode-card:hover {{
    background-color: alpha(#{surface_container_highest}, 0.85);
    border-color: alpha(#{primary}, 0.45);
}}

button.mode-card.selected {{
    background-color: alpha(#{primary_container}, 0.65);
    border: 2px solid #{primary};
    box-shadow: 0 0 26px alpha(#{primary}, 0.5), 0 0 8px alpha(#{primary}, 0.75);
}}

.mode-badge {{
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    background-color: alpha(#{primary}, 0.18);
    color: #{primary};
    border: 1px solid alpha(#{primary}, 0.35);
    border-radius: 9999px;
    padding: 2px 8px;
    margin-bottom: 6px;
}}

button.mode-card.selected .mode-badge {{
    background-color: #{primary};
    color: #{on_primary};
    border-color: #{primary};
}}

.mode-icon {{
    color: alpha(#{on_surface}, 0.75);
    margin-bottom: 12px;
    -gtk-icon-size: 44px;
    transition: color 180ms ease-in-out;
}}

button.mode-card:hover .mode-icon {{
    color: #{primary};
}}

button.mode-card.selected .mode-icon {{
    color: #{on_primary_container};
}}

.mode-name {{
    font-size: 15px;
    font-weight: 600;
    color: #{on_surface};
    margin-bottom: 4px;
}}

button.mode-card.selected .mode-name {{
    color: #{on_primary_container};
    font-weight: 700;
}}

.mode-desc {{
    font-size: 11px;
    color: #{on_surface_variant};
}}

button.mode-card.selected .mode-desc {{
    color: alpha(#{on_primary_container}, 0.95);
}}

.footer-pill {{
    background-color: alpha(#{surface_container_high}, 0.55);
    border: 1px solid alpha(#{outline_variant}, 0.2);
    border-radius: 9999px;
    padding: 6px 22px;
}}

.footer-label {{
    font-size: 12px;
    font-weight: 500;
    color: #{on_surface_variant};
}}
"#
    )
}

pub fn apply_theme() {
    let provider = CssProvider::new();
    let css = get_caelestia_css();
    provider.load_from_data(&css);

    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 10,
        );
    }
}
