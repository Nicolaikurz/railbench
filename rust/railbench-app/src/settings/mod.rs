use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Dark,
    Light,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "system" | "auto" => Self::System,
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    pub fn label(self, is_de: bool) -> &'static str {
        match (self, is_de) {
            (Self::System, true) => "System (Auto)",
            (Self::System, false) => "System (Auto)",
            (Self::Dark, true) => "Dunkel",
            (Self::Dark, false) => "Dark",
            (Self::Light, true) => "Hell",
            (Self::Light, false) => "Light",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliLaunchOptions {
    pub file_path: Option<String>,
    pub server: bool,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub room: Option<String>,
    pub connect: Option<String>,
}

/// Parses CLI arguments including layout file paths and Electron-compatible collab flags.
pub fn parse_cli_args(args: &[String]) -> CliLaunchOptions {
    let mut out = CliLaunchOptions::default();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--server" | "--collab-server" | "--start-server" | "--autostart-server" | "--serve" => {
                out.server = true;
            }
            "--port" | "--server-port" | "--collab-port" if i + 1 < args.len() => {
                out.port = args[i + 1].parse::<u16>().ok();
                i += 1;
            }
            "--password" | "--server-password" | "--collab-password" if i + 1 < args.len() => {
                out.password = Some(args[i + 1].clone());
                i += 1;
            }
            "--room" | "--server-room" | "--collab-room" if i + 1 < args.len() => {
                out.room = Some(args[i + 1].clone());
                i += 1;
            }
            "--connect" | "--auto-connect" | "--client" if i + 1 < args.len() => {
                out.connect = Some(args[i + 1].clone());
                i += 1;
            }
            other if !other.starts_with('-') => {
                if out.file_path.is_none() {
                    out.file_path = Some(other.to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    Ask,
    Recent,
    New,
    Demo,
    Open,
}

impl StartupAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Recent => "recent",
            Self::New => "new",
            Self::Demo => "demo",
            Self::Open => "open",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "recent" => Self::Recent,
            "new" => Self::New,
            "demo" => Self::Demo,
            "open" => Self::Open,
            _ => Self::Ask,
        }
    }

    pub fn label(self, is_de: bool) -> &'static str {
        match (self, is_de) {
            (Self::Ask, true) => "Beim Start fragen (Willkommen)",
            (Self::Ask, false) => "Ask on Startup (Welcome)",
            (Self::Recent, true) => "Letzten Plan fortsetzen",
            (Self::Recent, false) => "Continue Last Plan",
            (Self::New, true) => "Leeren Plan starten",
            (Self::New, false) => "Start Empty Plan",
            (Self::Demo, true) => "Demo-Anlage laden",
            (Self::Demo, false) => "Load Demo Layout",
            (Self::Open, true) => "Datei-Auswahl beim Start oeffnen",
            (Self::Open, false) => "Open File Dialog on Startup",
        }
    }
}


mod keymap;
pub use keymap::*;

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub dark_mode: bool,
    pub theme_mode: ThemeMode,
    pub lang: String,
    pub snapping_enabled: bool,
    pub layer_protection: bool,
    pub color_mode: String,
    pub show_labels: bool,
    pub show_connectors: bool,
    pub show_rulers: bool,
    pub show_wood_plates: bool,
    pub show_right_sidebar: bool,
    pub rotation_step: f64,
    pub rotation_snap_locked: bool,
    pub power_sim_enabled: bool,
    pub selected_catalog_id: Option<String>,
    pub active_sidebar_tab: String,
    pub camera_offset: Option<[f32; 2]>,
    pub camera_zoom: Option<f32>,
    pub bars_locked: bool,
    pub low_gpu_mode: bool,
    pub use_discrete_gpu: bool,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
    pub window_maximized: bool,
    pub left_sidebar_width: Option<f32>,
    pub right_sidebar_width: Option<f32>,
    pub collab_user_name: Option<String>,
    pub collab_user_color: Option<String>,
    pub collab_password: Option<String>,
    pub viewer3d_env: String,
    pub startup_action: StartupAction,
    pub last_opened_path: Option<String>,
    pub recent_files: Vec<String>,
    pub perf_monitor_visible: bool,
    pub custom_keymap: BTreeMap<String, String>,
    pub favorite_tracks: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            theme_mode: ThemeMode::Dark,
            lang: "de".into(),
            snapping_enabled: true,
            layer_protection: true,
            color_mode: "types".into(),
            show_labels: true,
            show_connectors: true,
            show_rulers: true,
            show_wood_plates: true,
            show_right_sidebar: true,
            rotation_step: 30.0,
            rotation_snap_locked: false,
            power_sim_enabled: false,
            selected_catalog_id: None,
            active_sidebar_tab: "tracks".into(),
            camera_offset: None,
            camera_zoom: None,
            bars_locked: false,
            low_gpu_mode: false,
            use_discrete_gpu: false,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            window_maximized: false,
            left_sidebar_width: None,
            right_sidebar_width: None,
            collab_user_name: None,
            collab_user_color: None,
            collab_password: None,
            viewer3d_env: "hall-landscape".into(),
            startup_action: StartupAction::Ask,
            last_opened_path: None,
            recent_files: Vec::new(),
            perf_monitor_visible: false,
            custom_keymap: BTreeMap::new(),
            favorite_tracks: Vec::new(),
        }
    }
}

pub fn detect_os_lang_de() -> bool {
    for var in ["LC_ALL", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let low = val.trim().to_ascii_lowercase();
            if !low.is_empty() && low != "c" && low != "posix" {
                return low.starts_with("de");
            }
        }
    }
    true
}

impl AppSettings {
    pub fn config_dir() -> Option<PathBuf> {
        Self::config_path()?.parent().map(|p| p.to_path_buf())
    }

    pub fn config_path() -> Option<PathBuf> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("railbench").join("settings.json"));
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                return Some(PathBuf::from(appdata).join("railbench").join("settings.json"));
            }
            return std::env::var_os("USERPROFILE")
                .map(|p| PathBuf::from(p).join(".config").join("railbench").join("settings.json"));
        }
        #[cfg(target_os = "macos")]
        {
            return std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join("Library")
                    .join("Application Support")
                    .join("railbench")
                    .join("settings.json")
            });
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config").join("railbench").join("settings.json"))
        }
    }

    pub fn effective_key(&self, action: KeyAction) -> String {
        let Some(def) = KEY_ACTION_DEFS.iter().find(|d| d.action == action) else {
            return String::new();
        };
        self.custom_keymap
            .get(def.id)
            .cloned()
            .unwrap_or_else(|| def.default_key.to_string())
    }

    pub fn matches_action(&self, action: KeyAction, i: &egui::InputState) -> bool {
        let binding = self.effective_key(action);
        if matches_input(&binding, i) {
            return true;
        }
        // Keep secondary ergonomic bindings aligned with TS defaults when uncustomized
        let is_custom = KEY_ACTION_DEFS
            .iter()
            .find(|d| d.action == action)
            .map_or(false, |d| self.custom_keymap.contains_key(d.id));
        if !is_custom {
            match action {
                KeyAction::Redo => return matches_input("Ctrl+Shift+Z", i),
                KeyAction::DeleteSelected => return matches_input("Backspace", i),
                KeyAction::Toggle3D => return matches_input("V", i),
                KeyAction::ToggleDecoder => return matches_input("Shift+D", i),
                KeyAction::ZoomIn => {
                    return !i.modifiers.command
                        && !i.modifiers.alt
                        && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
                }
                KeyAction::ZoomOut => {
                    return !i.modifiers.command
                        && !i.modifiers.alt
                        && i.key_pressed(egui::Key::Minus);
                }
                KeyAction::PanUp => return matches_input("Shift+Up", i),
                KeyAction::PanDown => return matches_input("Shift+Down", i),
                KeyAction::PanLeft => return matches_input("Shift+Left", i),
                KeyAction::PanRight => return matches_input("Shift+Right", i),
                KeyAction::OpenHelp => {
                    return !i.modifiers.command
                        && !i.modifiers.alt
                        && (i.key_pressed(egui::Key::Questionmark)
                            || i.events.iter().any(|e| matches!(e, egui::Event::Text(t) if t == "?")));
                }
                _ => {}
            }
        }
        false
    }

    pub fn set_key(&mut self, action: KeyAction, binding: String) {
        if let Some(def) = KEY_ACTION_DEFS.iter().find(|d| d.action == action) {
            if binding == def.default_key {
                self.custom_keymap.remove(def.id);
            } else {
                self.custom_keymap.insert(def.id.to_string(), binding);
            }
        }
    }

    pub fn find_conflicting_action(&self, target: KeyAction, binding: &str) -> Option<&'static KeyActionDef> {
        let needle = parse_shortcut(binding);
        KEY_ACTION_DEFS.iter().find(|def| {
            if def.action == target {
                return false;
            }
            let eff = self.effective_key(def.action);
            if let (Some(a), Some(b)) = (needle, parse_shortcut(&eff)) {
                a == b
            } else {
                eff.eq_ignore_ascii_case(binding)
            }
        })
    }

    pub fn reset_key(&mut self, action: KeyAction) {
        if let Some(def) = KEY_ACTION_DEFS.iter().find(|d| d.action == action) {
            self.custom_keymap.remove(def.id);
        }
    }

    pub fn reset_all_keys(&mut self) {
        self.custom_keymap.clear();
    }

    pub fn push_recent_file(&mut self, path: &str) {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return;
        }
        self.recent_files.retain(|p| p != trimmed);
        self.recent_files.insert(0, trimmed.to_string());
        self.recent_files.truncate(8);
        self.last_opened_path = Some(trimmed.to_string());
    }

    pub fn reset_personal_defaults(&mut self) {
        let preserved = self.last_opened_path.clone();
        let recents = self.recent_files.clone();
        let favs = self.favorite_tracks.clone();
        let wx = self.window_x;
        let wy = self.window_y;
        let ww = self.window_width;
        let wh = self.window_height;
        let wm = self.window_maximized;
        *self = Self::default();
        self.last_opened_path = preserved;
        self.recent_files = recents;
        self.favorite_tracks = favs;
        self.window_x = wx;
        self.window_y = wy;
        self.window_width = ww;
        self.window_height = wh;
        self.window_maximized = wm;
    }

    pub fn to_json_string(&self) -> String {
        let mut root = Map::new();
        root.insert("dark_mode".into(), Value::Bool(self.dark_mode));
        root.insert("theme_mode".into(), Value::String(self.theme_mode.as_str().into()));
        root.insert("lang".into(), Value::String(self.lang.clone()));
        root.insert("snapping_enabled".into(), Value::Bool(self.snapping_enabled));
        root.insert("layer_protection".into(), Value::Bool(self.layer_protection));
        root.insert("color_mode".into(), Value::String(self.color_mode.clone()));
        root.insert("show_labels".into(), Value::Bool(self.show_labels));
        root.insert("show_connectors".into(), Value::Bool(self.show_connectors));
        root.insert("show_rulers".into(), Value::Bool(self.show_rulers));
        root.insert("show_wood_plates".into(), Value::Bool(self.show_wood_plates));
        root.insert("show_right_sidebar".into(), Value::Bool(self.show_right_sidebar));
        root.insert("rotation_step".into(), Value::from(self.rotation_step));
        root.insert("rotation_snap_locked".into(), Value::Bool(self.rotation_snap_locked));
        root.insert("power_sim_enabled".into(), Value::Bool(self.power_sim_enabled));
        if let Some(ref cid) = self.selected_catalog_id {
            root.insert("selected_catalog_id".into(), Value::String(cid.clone()));
        }
        root.insert("active_sidebar_tab".into(), Value::String(self.active_sidebar_tab.clone()));
        if let Some(off) = self.camera_offset {
            root.insert("camera_offset".into(), Value::Array(vec![Value::from(off[0]), Value::from(off[1])]));
        }
        if let Some(z) = self.camera_zoom {
            root.insert("camera_zoom".into(), Value::from(z));
        }
        root.insert("bars_locked".into(), Value::Bool(self.bars_locked));
        root.insert("low_gpu_mode".into(), Value::Bool(self.low_gpu_mode));
        root.insert("use_discrete_gpu".into(), Value::Bool(self.use_discrete_gpu));
        if let Some(x) = self.window_x {
            root.insert("window_x".into(), Value::from(x));
        }
        if let Some(y) = self.window_y {
            root.insert("window_y".into(), Value::from(y));
        }
        if let Some(w) = self.window_width {
            root.insert("window_width".into(), Value::from(w));
        }
        if let Some(h) = self.window_height {
            root.insert("window_height".into(), Value::from(h));
        }
        root.insert("window_maximized".into(), Value::Bool(self.window_maximized));
        if let Some(w) = self.left_sidebar_width {
            root.insert("left_sidebar_width".into(), Value::from(w));
        }
        if let Some(w) = self.right_sidebar_width {
            root.insert("right_sidebar_width".into(), Value::from(w));
        }
        if let Some(ref n) = self.collab_user_name {
            root.insert("collab_user_name".into(), Value::String(n.clone()));
        }
        if let Some(ref c) = self.collab_user_color {
            root.insert("collab_user_color".into(), Value::String(c.clone()));
        }
        if let Some(ref pw) = self.collab_password {
            root.insert("collab_password".into(), Value::String(pw.clone()));
        }
        root.insert("viewer3d_env".into(), Value::String(self.viewer3d_env.clone()));
        root.insert("startup_action".into(), Value::String(self.startup_action.as_str().into()));
        root.insert("perf_monitor_visible".into(), Value::Bool(self.perf_monitor_visible));
        if let Some(ref p) = self.last_opened_path {
            root.insert("last_opened_path".into(), Value::String(p.clone()));
        }
        root.insert(
            "recent_files".into(),
            Value::Array(
                self.recent_files
                    .iter()
                    .map(|p| Value::String(p.clone()))
                    .collect(),
            ),
        );
        let mut km = Map::new();
        for (k, v) in &self.custom_keymap {
            km.insert(k.clone(), Value::String(v.clone()));
        }
        root.insert("custom_keymap".into(), Value::Object(km));
        root.insert(
            "favorite_tracks".into(),
            Value::Array(
                self.favorite_tracks
                    .iter()
                    .map(|id| Value::String(id.clone()))
                    .collect(),
            ),
        );
        serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_else(|_| "{}".into())
    }

    pub fn from_json_str(raw: &str) -> Self {
        let mut s = Self::default();
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) else {
            return s;
        };
        if let Some(v) = map.get("theme_mode").and_then(|v| v.as_str()) {
            s.theme_mode = ThemeMode::from_str(v);
            match s.theme_mode {
                ThemeMode::Dark => s.dark_mode = true,
                ThemeMode::Light => s.dark_mode = false,
                ThemeMode::System => {}
            }
        }
        if let Some(v) = map.get("dark_mode").and_then(|v| v.as_bool()) {
            s.dark_mode = v;
            if s.theme_mode != ThemeMode::System {
                s.theme_mode = if v { ThemeMode::Dark } else { ThemeMode::Light };
            }
        }
        if let Some(v) = map.get("lang").and_then(|v| v.as_str()) {
            s.lang = v.into();
        }
        if let Some(v) = map.get("snapping_enabled").and_then(|v| v.as_bool()) {
            s.snapping_enabled = v;
        }
        if let Some(v) = map.get("layer_protection").and_then(|v| v.as_bool()) {
            s.layer_protection = v;
        }
        if let Some(v) = map.get("color_mode").and_then(|v| v.as_str()) {
            s.color_mode = v.into();
        }
        if let Some(v) = map.get("show_labels").and_then(|v| v.as_bool()) {
            s.show_labels = v;
        }
        if let Some(v) = map.get("show_connectors").and_then(|v| v.as_bool()) {
            s.show_connectors = v;
        }
        if let Some(v) = map.get("show_rulers").and_then(|v| v.as_bool()) {
            s.show_rulers = v;
        }
        if let Some(v) = map.get("show_wood_plates").and_then(|v| v.as_bool()) {
            s.show_wood_plates = v;
        }
        if let Some(v) = map.get("show_right_sidebar").and_then(|v| v.as_bool()) {
            s.show_right_sidebar = v;
        }
        if let Some(v) = map.get("rotation_step").and_then(|v| v.as_f64()) {
            if v > 0.0 && v <= 180.0 {
                s.rotation_step = v;
            }
        }
        if let Some(v) = map.get("viewer3d_env").and_then(|v| v.as_str()) {
            s.viewer3d_env = v.into();
        }
        if let Some(v) = map.get("startup_action").and_then(|v| v.as_str()) {
            s.startup_action = StartupAction::from_str(v);
        }
        if let Some(v) = map.get("perf_monitor_visible").and_then(|v| v.as_bool()) {
            s.perf_monitor_visible = v;
        }
        if let Some(v) = map.get("last_opened_path").and_then(|v| v.as_str()) {
            s.last_opened_path = Some(v.into());
        }
        if let Some(v) = map.get("rotation_snap_locked").and_then(|v| v.as_bool()) {
            s.rotation_snap_locked = v;
        }
        if let Some(v) = map.get("power_sim_enabled").and_then(|v| v.as_bool()) {
            s.power_sim_enabled = v;
        }
        if let Some(v) = map.get("selected_catalog_id").and_then(|v| v.as_str()) {
            s.selected_catalog_id = Some(v.into());
        }
        if let Some(v) = map.get("active_sidebar_tab").and_then(|v| v.as_str()) {
            s.active_sidebar_tab = v.into();
        }
        if let Some(arr) = map.get("camera_offset").and_then(|v| v.as_array()) {
            if arr.len() >= 2 {
                if let (Some(x), Some(y)) = (arr[0].as_f64(), arr[1].as_f64()) {
                    s.camera_offset = Some([x as f32, y as f32]);
                }
            }
        }
        if let Some(v) = map.get("camera_zoom").and_then(|v| v.as_f64()) {
            if v > 0.02 && v <= 20.0 {
                s.camera_zoom = Some(v as f32);
            }
        }
        if let Some(v) = map.get("bars_locked").and_then(|v| v.as_bool()) {
            s.bars_locked = v;
        }
        if let Some(v) = map.get("low_gpu_mode").and_then(|v| v.as_bool()) {
            s.low_gpu_mode = v;
        }
        if let Some(v) = map.get("use_discrete_gpu").and_then(|v| v.as_bool()) {
            s.use_discrete_gpu = v;
        }
        if let Some(v) = map.get("window_x").and_then(|v| v.as_f64()) {
            if v.is_finite() && (-2000.0..=10000.0).contains(&v) {
                s.window_x = Some(v as f32);
            }
        }
        if let Some(v) = map.get("window_y").and_then(|v| v.as_f64()) {
            if v.is_finite() && (-2000.0..=10000.0).contains(&v) {
                s.window_y = Some(v as f32);
            }
        }
        if let Some(v) = map.get("window_width").and_then(|v| v.as_f64()) {
            if v >= 400.0 {
                s.window_width = Some(v as f32);
            }
        }
        if let Some(v) = map.get("window_height").and_then(|v| v.as_f64()) {
            if v >= 300.0 {
                s.window_height = Some(v as f32);
            }
        }
        if let Some(v) = map.get("window_maximized").and_then(|v| v.as_bool()) {
            s.window_maximized = v;
        }
        if let Some(v) = map.get("left_sidebar_width").and_then(|v| v.as_f64()) {
            if (220.0..=520.0).contains(&v) {
                s.left_sidebar_width = Some(v as f32);
            }
        }
        if let Some(v) = map.get("right_sidebar_width").and_then(|v| v.as_f64()) {
            if (215.0..=480.0).contains(&v) {
                s.right_sidebar_width = Some(v as f32);
            }
        }
        if let Some(arr) = map.get("recent_files").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(p) = item.as_str() {
                    let t = p.trim();
                    if !t.is_empty() && !s.recent_files.iter().any(|x| x == t) {
                        s.recent_files.push(t.to_string());
                    }
                }
            }
            s.recent_files.truncate(8);
        } else if let Some(ref lp) = s.last_opened_path {
            s.recent_files.push(lp.clone());
        }
        if let Some(v) = map.get("collab_user_name").and_then(|v| v.as_str()) {
            s.collab_user_name = Some(v.into());
        }
        if let Some(v) = map.get("collab_user_color").and_then(|v| v.as_str()) {
            s.collab_user_color = Some(v.into());
        }
        if let Some(v) = map.get("collab_password").and_then(|v| v.as_str()) {
            s.collab_password = Some(v.into());
        }
        if let Some(Value::Object(km)) = map.get("custom_keymap") {
            for (k, v) in km {
                if let Some(bind) = v.as_str() {
                    if parse_shortcut(bind).is_some() {
                        s.custom_keymap.insert(k.clone(), bind.into());
                    }
                }
            }
        }
        if let Some(Value::Array(favs)) = map.get("favorite_tracks") {
            s.favorite_tracks = favs
                .iter()
                .filter_map(|v| v.as_str().map(|id| id.to_string()))
                .collect();
        }
        s
    }

    pub fn load_from_disk() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .map(|raw| Self::from_json_str(&raw))
            .unwrap_or_default()
    }

    pub fn save_to_disk(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, self.to_json_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_json_roundtrip_and_defaults() {
        let mut cfg = AppSettings::default();
        cfg.dark_mode = false;
        cfg.lang = "en".into();
        cfg.rotation_step = 15.0;
        cfg.startup_action = StartupAction::Demo;
        cfg.last_opened_path = Some("/tmp/demo.railbench".into());
        cfg.set_key(KeyAction::Toggle3D, "Ctrl+Shift+3".into());
        cfg.favorite_tracks = vec!["24188".into(), "24611".into()];

        let json = cfg.to_json_string();
        let parsed = AppSettings::from_json_str(&json);
        assert!(!parsed.dark_mode);
        assert_eq!(parsed.lang, "en");
        assert!((parsed.rotation_step - 15.0).abs() < 1e-6);
        assert_eq!(parsed.startup_action, StartupAction::Demo);
        assert_eq!(parsed.effective_key(KeyAction::Toggle3D), "Ctrl+Shift+3");
        assert_eq!(parsed.favorite_tracks, vec!["24188".to_string(), "24611".to_string()]);

        let mut reset = parsed.clone();
        reset.reset_personal_defaults();
        assert!(reset.dark_mode);
        assert_eq!(reset.effective_key(KeyAction::Toggle3D), "3");
        assert_eq!(reset.last_opened_path.as_deref(), Some("/tmp/demo.railbench"));
    }

    #[test]
    fn shortcut_parse_format_and_override() {
        let spec = parse_shortcut("Ctrl+Shift+R").unwrap();
        assert!(spec.ctrl && spec.shift && !spec.alt);
        assert_eq!(spec.key, egui::Key::R);
        assert_eq!(format_shortcut(spec), "Ctrl+Shift+R");

        let f3 = parse_shortcut("F3").unwrap();
        assert_eq!(f3.key, egui::Key::F3);
        assert_eq!(format_shortcut(f3), "F3");
        assert!(parse_shortcut("Ctrl+").is_none());

        assert_eq!(parse_shortcut("+").unwrap().key, egui::Key::Plus);
        assert_eq!(parse_shortcut("-").unwrap().key, egui::Key::Minus);
        assert_eq!(parse_shortcut("Escape").unwrap().key, egui::Key::Escape);
        let alt_l = parse_shortcut("Alt+L").unwrap();
        assert!(alt_l.alt && !alt_l.ctrl && !alt_l.shift && alt_l.key == egui::Key::L);
        let save_as = parse_shortcut("Ctrl+Shift+S").unwrap();
        assert!(save_as.ctrl && save_as.shift && !save_as.alt && save_as.key == egui::Key::S);
        assert_eq!(parse_shortcut("?").unwrap().key, egui::Key::Questionmark);
    }

    #[test]
    fn default_keymap_matches_electron_bindings() {
        let cfg = AppSettings::default();
        assert_eq!(cfg.effective_key(KeyAction::ToggleRulers), "Shift+L");
        assert_eq!(cfg.effective_key(KeyAction::ToggleLantern), "W");
        assert_eq!(cfg.effective_key(KeyAction::ToggleDecoder), "Shift+W");
        assert_eq!(cfg.effective_key(KeyAction::ToggleSelectionLock), "Alt+L");
        assert_eq!(cfg.effective_key(KeyAction::ToggleMeasure), "M");
        assert_eq!(cfg.effective_key(KeyAction::CycleColorMode), "C");
        assert_eq!(cfg.effective_key(KeyAction::SavePlanAs), "Ctrl+Shift+S");
        assert_eq!(cfg.effective_key(KeyAction::DeselectAll), "Escape");
        assert_eq!(cfg.effective_key(KeyAction::ZoomIn), "Plus");
        assert_eq!(cfg.effective_key(KeyAction::ZoomOut), "Minus");
        assert_eq!(cfg.effective_key(KeyAction::PanUp), "Up");
        assert_eq!(cfg.effective_key(KeyAction::PanDown), "Down");
        assert_eq!(cfg.effective_key(KeyAction::PanLeft), "Left");
        assert_eq!(cfg.effective_key(KeyAction::PanRight), "Right");
        assert_eq!(cfg.effective_key(KeyAction::TogglePerfMonitor), "F3");
        assert_eq!(cfg.effective_key(KeyAction::OpenHelp), "F1");
        assert_eq!(cfg.effective_key(KeyAction::FlipSelected), "Alt+F");
        assert_eq!(cfg.effective_key(KeyAction::ToggleGroup), "G");
        assert_eq!(cfg.effective_key(KeyAction::AutoRamp), "Shift+M");
    }

    #[test]
    fn extended_settings_conflicts_and_startup_open() {
        let mut s = AppSettings::default();
        s.startup_action = StartupAction::Open;
        s.rotation_snap_locked = true;
        s.power_sim_enabled = true;
        s.viewer3d_env = "minimal".into();
        s.window_width = Some(1440.0);
        s.window_height = Some(900.0);
        let json = s.to_json_string();
        let de = AppSettings::from_json_str(&json);
        assert_eq!(de.startup_action, StartupAction::Open);
        assert!(de.rotation_snap_locked);
        assert!(de.power_sim_enabled);
        assert_eq!(de.viewer3d_env, "minimal");
        assert_eq!(de.window_width, Some(1440.0));

        // Conflict lookup detects `S` belongs to ToggleSnapping when assigning to Undo
        let conflict = de.find_conflicting_action(KeyAction::Undo, "S").unwrap();
        assert_eq!(conflict.action, KeyAction::ToggleSnapping);
        assert!(de.find_conflicting_action(KeyAction::Undo, "Ctrl+Z").is_none());
    }
}
