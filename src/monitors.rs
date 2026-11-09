use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub mode_str: String,
}

impl DisplayMode {
    pub fn pixels(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MonitorInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub disabled: bool,
    pub mirror_of: String,
    pub available_modes: Vec<DisplayMode>,
    pub is_internal: bool,
}

impl MonitorInfo {
    pub fn best_mode(&self) -> Option<DisplayMode> {
        if self.available_modes.is_empty() {
            if self.width > 0 && self.height > 0 {
                return Some(DisplayMode {
                    width: self.width,
                    height: self.height,
                    refresh_rate: self.refresh_rate,
                    mode_str: format!("{}x{}@{:.2}Hz", self.width, self.height, self.refresh_rate),
                });
            }
            return None;
        }

        let mut modes = self.available_modes.clone();
        modes.sort_by(|a, b| {
            b.pixels()
                .cmp(&a.pixels())
                .then_with(|| {
                    b.refresh_rate
                        .partial_cmp(&a.refresh_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        modes.into_iter().next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayPreset {
    PcOnly = 0,
    Duplicate = 1,
    Extend = 2,
    SecondOnly = 3,
}

impl DisplayPreset {
    pub const ALL: [DisplayPreset; 4] = [
        DisplayPreset::PcOnly,
        DisplayPreset::Duplicate,
        DisplayPreset::Extend,
        DisplayPreset::SecondOnly,
    ];

    pub fn index(&self) -> usize {
        *self as usize
    }

    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => DisplayPreset::PcOnly,
            1 => DisplayPreset::Duplicate,
            2 => DisplayPreset::Extend,
            _ => DisplayPreset::SecondOnly,
        }
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pc" | "internal" | "pc screen only" => Some(DisplayPreset::PcOnly),
            "duplicate" | "mirror" => Some(DisplayPreset::Duplicate),
            "extend" => Some(DisplayPreset::Extend),
            "second" | "external" | "second screen only" => Some(DisplayPreset::SecondOnly),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DisplayPreset::PcOnly => "PC Screen Only",
            DisplayPreset::Duplicate => "Duplicate",
            DisplayPreset::Extend => "Extend",
            DisplayPreset::SecondOnly => "Second Screen Only",
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            DisplayPreset::PcOnly => "Use built-in display only and disable external monitor",
            DisplayPreset::Duplicate => "Mirror the built-in display onto external monitor",
            DisplayPreset::Extend => "Extend desktop across both displays side-by-side",
            DisplayPreset::SecondOnly => "Use external monitor only and disable built-in display",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            DisplayPreset::PcOnly => "video-single-display-symbolic",
            DisplayPreset::Duplicate => "video-joined-displays-symbolic",
            DisplayPreset::Extend => "view-paged-symbolic",
            DisplayPreset::SecondOnly => "preferences-desktop-display-symbolic",
        }
    }
}

pub struct MonitorManager {
    #[allow(dead_code)]
    config_dir: PathBuf,
    monitors_lua: PathBuf,
}

impl Default for MonitorManager {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MonitorManager {
    pub fn new(custom_dir: Option<PathBuf>) -> Self {
        let config_dir = custom_dir.unwrap_or_else(|| {
            dirs_home().join(".config").join("hypr")
        });
        let monitors_lua = config_dir.join("monitors.lua");
        Self {
            config_dir,
            monitors_lua,
        }
    }

    pub fn parse_mode_string(mode_str: &str) -> Option<DisplayMode> {
        let clean = mode_str.trim();
        let re = Regex::new(r"^(\d+)x(\d+)(?:@([\d.]+))?(?:Hz)?$").ok()?;
        let caps = re.captures(clean)?;

        let w: u32 = caps.get(1)?.as_str().parse().ok()?;
        let h: u32 = caps.get(2)?.as_str().parse().ok()?;
        let rr: f64 = match caps.get(3) {
            Some(m) => m.as_str().parse().unwrap_or(60.0),
            None => 60.0,
        };

        Some(DisplayMode {
            width: w,
            height: h,
            refresh_rate: rr,
            mode_str: clean.to_string(),
        })
    }

    pub fn is_internal_name(name: &str) -> bool {
        let upper = name.to_uppercase();
        upper.starts_with("EDP") || upper.starts_with("LVDS") || upper.starts_with("DSI")
    }

    pub fn get_monitors(&self) -> Vec<MonitorInfo> {
        let output = match Command::new("hyprctl")
            .args(["monitors", "all", "-j"])
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                eprintln!("Failed to execute hyprctl monitors all -j: {}", e);
                return Vec::new();
            }
        };

        if !output.status.success() {
            return Vec::new();
        }

        let Ok(text) = String::from_utf8(output.stdout) else {
            return Vec::new();
        };

        let Ok(items): Result<Vec<Value>, _> = serde_json::from_str(&text) else {
            return Vec::new();
        };

        let mut monitors = Vec::new();
        for item in items {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let is_internal = Self::is_internal_name(&name);

            let mut modes = Vec::new();
            if let Some(arr) = item.get("availableModes").and_then(|v| v.as_array()) {
                for m in arr {
                    if let Some(s) = m.as_str() {
                        if let Some(parsed) = Self::parse_mode_string(s) {
                            modes.push(parsed);
                        }
                    }
                }
            }

            let mut mirror_of = item.get("mirrorOf").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if mirror_of.eq_ignore_ascii_case("none") {
                mirror_of.clear();
            }

            let mon = MonitorInfo {
                id: item.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                name,
                description: item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                make: item.get("make").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                model: item.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                width: item.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                height: item.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                refresh_rate: item.get("refreshRate").and_then(|v| v.as_f64()).unwrap_or(0.0),
                x: item.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: item.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                scale: item.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0),
                disabled: item.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                mirror_of,
                available_modes: modes,
                is_internal,
            };
            monitors.push(mon);
        }

        if !monitors.is_empty() && !monitors.iter().any(|m| m.is_internal) {
            monitors[0].is_internal = true;
        }

        monitors
    }

    pub fn get_displays(&self) -> (Option<MonitorInfo>, Option<MonitorInfo>) {
        let monitors = self.get_monitors();
        let internal = monitors.iter().find(|m| m.is_internal).cloned();
        let external = monitors.iter().find(|m| !m.is_internal).cloned();
        (internal, external)
    }

    pub fn detect_current_mode(&self) -> DisplayPreset {
        // 1. Read saved configuration first
        if self.monitors_lua.exists() {
            if let Ok(content) = fs::read_to_string(&self.monitors_lua) {
                for line in content.lines() {
                    if let Some(mode_str) = line.strip_prefix("-- Mode:") {
                        let trimmed = mode_str.trim();
                        if let Some(preset) = DisplayPreset::from_str_name(trimmed) {
                            return preset;
                        }
                    }
                }
            }
        }

        // 2. Live detection fallback
        let (internal, external) = self.get_displays();
        let (Some(int_mon), Some(ext_mon)) = (internal, external) else {
            return DisplayPreset::PcOnly;
        };

        if ext_mon.disabled && !int_mon.disabled {
            DisplayPreset::PcOnly
        } else if int_mon.disabled && !ext_mon.disabled {
            DisplayPreset::SecondOnly
        } else if ext_mon.mirror_of == int_mon.name || int_mon.mirror_of == ext_mon.name {
            DisplayPreset::Duplicate
        } else {
            DisplayPreset::Extend
        }
    }

    pub fn generate_lua_config(&self, preset: DisplayPreset) -> (String, Vec<String>) {
        let (internal, external) = self.get_displays();
        let Some(int_mon) = internal else {
            return ("-- No display found\n".to_string(), Vec::new());
        };

        let int_name = &int_mon.name;
        let int_mode_str = "preferred";

        let mut eval_cmds = Vec::new();
        let mut lua_lines = vec![
            "-- Generated automatically by hyprswitch".to_string(),
            format!("-- Mode: {}", preset.name()),
            "".to_string(),
        ];

        let ext_name = match &external {
            Some(e) => e.name.as_str(),
            None => "",
        };

        let ext_mode_str = match &external {
            Some(e) => match e.best_mode() {
                Some(best) => format!("{}x{}@{:.2}", best.width, best.height, best.refresh_rate),
                None => "preferred".to_string(),
            },
            None => "preferred".to_string(),
        };

        match preset {
            DisplayPreset::PcOnly => {
                let cmd_int = format!(
                    r#"hl.monitor({{ output = "{}", mode = "{}", position = "0x0", scale = 1 }})"#,
                    int_name, int_mode_str
                );
                let cmd_ext = format!(
                    r#"hl.monitor({{ output = "{}", disabled = true }})"#,
                    ext_name
                );
                eval_cmds.push(cmd_int.clone());
                eval_cmds.push(cmd_ext.clone());
                lua_lines.push(cmd_int);
                lua_lines.push(cmd_ext);
            }
            DisplayPreset::Duplicate => {
                let cmd_int = format!(
                    r#"hl.monitor({{ output = "{}", mode = "{}", position = "0x0", scale = 1 }})"#,
                    int_name, int_mode_str
                );
                let cmd_ext = format!(
                    r#"hl.monitor({{ output = "{}", mode = "preferred", position = "0x0", scale = 1, mirror = "{}" }})"#,
                    ext_name, int_name
                );
                eval_cmds.push(cmd_int.clone());
                eval_cmds.push(cmd_ext.clone());
                lua_lines.push(cmd_int);
                lua_lines.push(cmd_ext);
            }
            DisplayPreset::Extend => {
                let cmd_int = format!(
                    r#"hl.monitor({{ output = "{}", mode = "{}", position = "0x0", scale = 1 }})"#,
                    int_name, int_mode_str
                );
                let cmd_ext = format!(
                    r#"hl.monitor({{ output = "{}", mode = "{}", position = "auto-right", scale = 1 }})"#,
                    ext_name, ext_mode_str
                );
                eval_cmds.push(cmd_int.clone());
                eval_cmds.push(cmd_ext.clone());
                lua_lines.push(cmd_int);
                lua_lines.push(cmd_ext);
            }
            DisplayPreset::SecondOnly => {
                let cmd_ext = format!(
                    r#"hl.monitor({{ output = "{}", mode = "{}", position = "0x0", scale = 1 }})"#,
                    ext_name, ext_mode_str
                );
                eval_cmds.push(cmd_ext.clone());
                lua_lines.push(cmd_ext);

                if external.is_some() {
                    let cmd_int = format!(
                        r#"hl.monitor({{ output = "{}", disabled = true }})"#,
                        int_name
                    );
                    eval_cmds.push(cmd_int.clone());
                    lua_lines.push(cmd_int);
                } else {
                    let cmd_int = format!(
                        r#"hl.monitor({{ output = "{}", mode = "{}", position = "0x0", scale = 1 }})"#,
                        int_name, int_mode_str
                    );
                    eval_cmds.push(cmd_int.clone());
                    lua_lines.push(cmd_int);
                }
            }
        }

        let full_lua = lua_lines.join("\n") + "\n";
        (full_lua, eval_cmds)
    }

    pub fn apply_mode(&self, preset: DisplayPreset) -> Result<(), String> {
        let (lua_content, eval_cmds) = self.generate_lua_config(preset);
        if eval_cmds.is_empty() {
            return Ok(());
        }

        let combined_cmd = eval_cmds.join(" ");
        let status = Command::new("hyprctl")
            .args(["eval", &combined_cmd])
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => eprintln!("hyprctl eval exited with code: {:?}", s.code()),
            Err(e) => eprintln!("Failed to execute hyprctl eval: {}", e),
        }

        if let Some(parent) = self.monitors_lua.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::write(&self.monitors_lua, lua_content)
            .map_err(|e| format!("Failed to write to {}: {}", self.monitors_lua.display(), e))?;

        Ok(())
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mode_string() {
        let m1 = MonitorManager::parse_mode_string("1920x1080@60Hz").unwrap();
        assert_eq!(m1.width, 1920);
        assert_eq!(m1.height, 1080);
        assert_eq!(m1.refresh_rate, 60.0);

        let m2 = MonitorManager::parse_mode_string("2560x1440@143.91").unwrap();
        assert_eq!(m2.width, 2560);
        assert_eq!(m2.height, 1440);
        assert!((m2.refresh_rate - 143.91).abs() < 0.01);
    }

    #[test]
    fn test_best_mode_sorting() {
        let m1 = DisplayMode { width: 1920, height: 1080, refresh_rate: 60.0, mode_str: "1920x1080@60".into() };
        let m2 = DisplayMode { width: 1920, height: 1080, refresh_rate: 144.0, mode_str: "1920x1080@144".into() };
        let m3 = DisplayMode { width: 2560, height: 1440, refresh_rate: 60.0, mode_str: "2560x1440@60".into() };

        let mon = MonitorInfo {
            id: 1,
            name: "HDMI-A-1".into(),
            description: "".into(),
            make: "".into(),
            model: "".into(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            x: 0,
            y: 0,
            scale: 1.0,
            disabled: false,
            mirror_of: "".into(),
            available_modes: vec![m1, m2, m3],
            is_internal: false,
        };

        let best = mon.best_mode().unwrap();
        assert_eq!(best.width, 2560);
        assert_eq!(best.height, 1440);
    }

    #[test]
    fn test_is_internal_name() {
        assert!(MonitorManager::is_internal_name("eDP-1"));
        assert!(MonitorManager::is_internal_name("EDP-2"));
        assert!(MonitorManager::is_internal_name("LVDS-1"));
        assert!(!MonitorManager::is_internal_name("HDMI-A-1"));
        assert!(!MonitorManager::is_internal_name("DP-1"));
    }
}

    #[test]
    fn test_preset_names_and_conversion() {
        assert_eq!(DisplayPreset::from_str_name("pc"), Some(DisplayPreset::PcOnly));
        assert_eq!(DisplayPreset::from_str_name("duplicate"), Some(DisplayPreset::Duplicate));
        assert_eq!(DisplayPreset::from_str_name("extend"), Some(DisplayPreset::Extend));
        assert_eq!(DisplayPreset::from_str_name("second"), Some(DisplayPreset::SecondOnly));

        assert_eq!(DisplayPreset::from_index(0), DisplayPreset::PcOnly);
        assert_eq!(DisplayPreset::from_index(1), DisplayPreset::Duplicate);
        assert_eq!(DisplayPreset::from_index(2), DisplayPreset::Extend);
        assert_eq!(DisplayPreset::from_index(3), DisplayPreset::SecondOnly);
        assert_eq!(DisplayPreset::from_index(4), DisplayPreset::PcOnly);
    }

    #[test]
    fn test_detect_current_mode_from_file() {
        let tmp_dir = std::env::temp_dir().join("hyprswitch_test_detect");
        let _ = fs::create_dir_all(&tmp_dir);
        let lua_file = tmp_dir.join("monitors.lua");

        fs::write(&lua_file, "-- Generated automatically\n-- Mode: Extend\n\nhl.monitor(...)").unwrap();
        let mm = MonitorManager::new(Some(tmp_dir.clone()));
        assert_eq!(mm.detect_current_mode(), DisplayPreset::Extend);

        fs::write(&lua_file, "-- Generated automatically\n-- Mode: Duplicate\n\nhl.monitor(...)").unwrap();
        assert_eq!(mm.detect_current_mode(), DisplayPreset::Duplicate);

        let _ = fs::remove_dir_all(tmp_dir);
    }
