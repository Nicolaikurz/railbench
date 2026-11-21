use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    Ask,
    Recent,
    New,
    Demo,
}

impl StartupAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Recent => "recent",
            Self::New => "new",
            Self::Demo => "demo",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "recent" => Self::Recent,
            "new" => Self::New,
            "demo" => Self::Demo,
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    View,
    Edit,
    Tools,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Undo,
    Redo,
    Save,
    Open,
    NewPlan,
    ExportCsv,
    PrintPlan,
    Copy,
    Paste,
    Duplicate,
    SelectAll,
    FitView,
    RotateCw,
    RotateCcw,
    DeleteSelected,
    Toggle3D,
    ToggleRulers,
    ToggleLabels,
    ToggleSnapping,
    TogglePowerSim,
    ToggleLayersSidebar,
    TogglePerfMonitor,
    ToggleFeeder,
    ToggleCatenaryFeeder,
    ToggleIsolator,
    ToggleCatenaryIsolator,
    ToggleCatenary,
    ToggleBridge,
    ToggleTunnel,
    ToggleLantern,
    ToggleDecoder,
    ToggleSignal,
    OpenHelp,
}

pub struct KeyActionDef {
    pub action: KeyAction,
    pub id: &'static str,
    pub category: KeyCategory,
    pub label_de: &'static str,
    pub label_en: &'static str,
    pub desc_de: &'static str,
    pub desc_en: &'static str,
    pub default_key: &'static str,
}

pub const KEY_ACTION_DEFS: &[KeyActionDef] = &[
    KeyActionDef {
        action: KeyAction::Undo,
        id: "undo",
        category: KeyCategory::Edit,
        label_de: "Rueckgaengig",
        label_en: "Undo",
        desc_de: "Letzten Bearbeitungsschritt rueckgaengig machen",
        desc_en: "Revert last editing step",
        default_key: "Ctrl+Z",
    },
    KeyActionDef {
        action: KeyAction::Redo,
        id: "redo",
        category: KeyCategory::Edit,
        label_de: "Wiederherstellen",
        label_en: "Redo",
        desc_de: "Rueckgaengig gemachten Schritt wiederherstellen",
        desc_en: "Restore undone action",
        default_key: "Ctrl+Y",
    },
    KeyActionDef {
        action: KeyAction::NewPlan,
        id: "newPlan",
        category: KeyCategory::File,
        label_de: "Neuer Plan",
        label_en: "New Plan",
        desc_de: "Erstellt eine leere Anlage",
        desc_en: "Create an empty layout",
        default_key: "Ctrl+N",
    },
    KeyActionDef {
        action: KeyAction::Open,
        id: "openPlan",
        category: KeyCategory::File,
        label_de: "Plan oeffnen",
        label_en: "Open Plan",
        desc_de: ".railbench Datei oeffnen",
        desc_en: "Open a .railbench layout file",
        default_key: "Ctrl+O",
    },
    KeyActionDef {
        action: KeyAction::Save,
        id: "savePlan",
        category: KeyCategory::File,
        label_de: "Plan speichern",
        label_en: "Save Plan",
        desc_de: "Aktuellen Gleisplan speichern",
        desc_en: "Save current layout",
        default_key: "Ctrl+S",
    },
    KeyActionDef {
        action: KeyAction::ExportCsv,
        id: "exportCsv",
        category: KeyCategory::File,
        label_de: "Stueckliste (CSV) exportieren",
        label_en: "Export Parts List (CSV)",
        desc_de: "Exportiert die Stueckliste als CSV-Datei",
        desc_en: "Export parts inventory to CSV",
        default_key: "Ctrl+E",
    },
    KeyActionDef {
        action: KeyAction::PrintPlan,
        id: "printPlan",
        category: KeyCategory::File,
        label_de: "Export / PDF Druckdialog",
        label_en: "Export / Print Dialog",
        desc_de: "Oeffnet den PDF/SVG/XML/CSV Exportdialog",
        desc_en: "Open blueprint PDF/SVG/XML/CSV export dialog",
        default_key: "Ctrl+P",
    },
    KeyActionDef {
        action: KeyAction::Copy,
        id: "copy",
        category: KeyCategory::Edit,
        label_de: "Kopieren",
        label_en: "Copy",
        desc_de: "Auswahl in die Zwischenablage kopieren",
        desc_en: "Copy selection to clipboard",
        default_key: "Ctrl+C",
    },
    KeyActionDef {
        action: KeyAction::Paste,
        id: "paste",
        category: KeyCategory::Edit,
        label_de: "Einfuegen",
        label_en: "Paste",
        desc_de: "Elemente aus Zwischenablage einfuegen",
        desc_en: "Paste clipboard items",
        default_key: "Ctrl+V",
    },
    KeyActionDef {
        action: KeyAction::Duplicate,
        id: "duplicate",
        category: KeyCategory::Edit,
        label_de: "Duplizieren",
        label_en: "Duplicate",
        desc_de: "Markierte Elemente duplizieren",
        desc_en: "Duplicate selected items",
        default_key: "Ctrl+D",
    },
    KeyActionDef {
        action: KeyAction::SelectAll,
        id: "selectAll",
        category: KeyCategory::Edit,
        label_de: "Alles auswaehlen",
        label_en: "Select All",
        desc_de: "Waehlt alle Gleise und Modelle aus",
        desc_en: "Select all tracks and models",
        default_key: "Ctrl+A",
    },
    KeyActionDef {
        action: KeyAction::RotateCw,
        id: "rotateCw",
        category: KeyCategory::Edit,
        label_de: "Im Uhrzeigersinn drehen",
        label_en: "Rotate Clockwise",
        desc_de: "Markierte Elemente drehen",
        desc_en: "Rotate selected elements clockwise",
        default_key: "R",
    },
    KeyActionDef {
        action: KeyAction::RotateCcw,
        id: "rotateCcw",
        category: KeyCategory::Edit,
        label_de: "Gegen Uhrzeigersinn drehen",
        label_en: "Rotate Counter-Clockwise",
        desc_de: "Markierte Elemente rueckwaerts drehen",
        desc_en: "Rotate selected elements counter-clockwise",
        default_key: "Shift+R",
    },
    KeyActionDef {
        action: KeyAction::DeleteSelected,
        id: "deleteSelected",
        category: KeyCategory::Edit,
        label_de: "Auswahl loeschen",
        label_en: "Delete Selected",
        desc_de: "Markierte Gleise und Modelle entfernen",
        desc_en: "Delete selected tracks and models",
        default_key: "Delete",
    },
    KeyActionDef {
        action: KeyAction::FitView,
        id: "resetView",
        category: KeyCategory::View,
        label_de: "Ansicht einpassen",
        label_en: "Fit View",
        desc_de: "Kamera auf die gesamte Anlage zentrieren",
        desc_en: "Fit camera onto layout",
        default_key: "Ctrl+0",
    },
    KeyActionDef {
        action: KeyAction::Toggle3D,
        id: "toggle3D",
        category: KeyCategory::View,
        label_de: "2D / 3D Ansicht wechseln",
        label_en: "Toggle 2D / 3D View",
        desc_de: "Zwischen 2D CAD und 3D Studio umschalten",
        desc_en: "Switch between 2D CAD and 3D viewer",
        default_key: "3",
    },
    KeyActionDef {
        action: KeyAction::ToggleRulers,
        id: "toggleRuler",
        category: KeyCategory::View,
        label_de: "Lineale ein-/ausblenden",
        label_en: "Toggle Rulers",
        desc_de: "Millimeter-Lineale am Rand umschalten",
        desc_en: "Toggle viewport edge rulers",
        default_key: "Shift+K",
    },
    KeyActionDef {
        action: KeyAction::ToggleLabels,
        id: "toggleTrackLabels",
        category: KeyCategory::View,
        label_de: "Gleisnummern anzeigen",
        label_en: "Toggle Track Labels",
        desc_de: "Artikelnummern ein- oder ausblenden",
        desc_en: "Show or hide catalog article labels",
        default_key: "N",
    },
    KeyActionDef {
        action: KeyAction::ToggleSnapping,
        id: "toggleSnapping",
        category: KeyCategory::Tools,
        label_de: "Magnet-Snap umschalten",
        label_en: "Toggle Snapping",
        desc_de: "Automatisches Verbinden von Gleisen",
        desc_en: "Enable or disable magnetic track snapping",
        default_key: "S",
    },
    KeyActionDef {
        action: KeyAction::TogglePowerSim,
        id: "togglePowerSim",
        category: KeyCategory::Tools,
        label_de: "Strom-Simulation",
        label_en: "Toggle Power Simulation",
        desc_de: "Spannungsabfall & Kurzschluss-Analyse",
        desc_en: "Voltage drop & short circuit simulation",
        default_key: "P",
    },
    KeyActionDef {
        action: KeyAction::ToggleLayersSidebar,
        id: "toggleLayersSidebar",
        category: KeyCategory::View,
        label_de: "Ebenen-Leiste umschalten",
        label_en: "Toggle Layers Sidebar",
        desc_de: "Rechte Ebenen-Seitenleiste ein-/ausblenden",
        desc_en: "Show or hide right layers sidebar",
        default_key: "Shift+B",
    },
    KeyActionDef {
        action: KeyAction::TogglePerfMonitor,
        id: "togglePerfMonitor",
        category: KeyCategory::View,
        label_de: "Hardware Monitor",
        label_en: "Hardware Monitor",
        desc_de: "FPS, CPU, RAM und GPU Auslastung",
        desc_en: "FPS, CPU, RAM and GPU telemetry overlay",
        default_key: "F3",
    },
    KeyActionDef {
        action: KeyAction::ToggleFeeder,
        id: "toggleFeeder",
        category: KeyCategory::Tools,
        label_de: "Stromeinspeisung",
        label_en: "Toggle Power Feeder",
        desc_de: "Einspeisung an Auswahl umschalten",
        desc_en: "Toggle rail feeder on selected tracks",
        default_key: "F",
    },
    KeyActionDef {
        action: KeyAction::ToggleCatenaryFeeder,
        id: "toggleCatenaryFeeder",
        category: KeyCategory::Tools,
        label_de: "Oberleitungs-Einspeisung",
        label_en: "Toggle Catenary Feeder",
        desc_de: "Oberleitungsspeisung an Auswahl umschalten",
        desc_en: "Toggle catenary feeder on selection",
        default_key: "Shift+F",
    },
    KeyActionDef {
        action: KeyAction::ToggleIsolator,
        id: "toggleIsolator",
        category: KeyCategory::Tools,
        label_de: "Isolierverbinder",
        label_en: "Toggle Rail Isolator",
        desc_de: "Schienen-Isolator an Auswahl umschalten",
        desc_en: "Toggle rail isolator on selection",
        default_key: "I",
    },
    KeyActionDef {
        action: KeyAction::ToggleCatenaryIsolator,
        id: "toggleCatenaryIsolator",
        category: KeyCategory::Tools,
        label_de: "Oberleitungs-Isolator",
        label_en: "Toggle Catenary Isolator",
        desc_de: "Streckentrenner in Oberleitung",
        desc_en: "Toggle catenary section isolator",
        default_key: "Shift+I",
    },
    KeyActionDef {
        action: KeyAction::ToggleCatenary,
        id: "toggleCatenary",
        category: KeyCategory::Tools,
        label_de: "Oberleitung",
        label_en: "Toggle Catenary",
        desc_de: "Oberleitungsmasten & Fahrdraht",
        desc_en: "Toggle overhead catenary masts and wire",
        default_key: "O",
    },
    KeyActionDef {
        action: KeyAction::ToggleBridge,
        id: "toggleBridge",
        category: KeyCategory::Tools,
        label_de: "Bruecke",
        label_en: "Toggle Bridge",
        desc_de: "Gleisbruecke fuer Auswahl umschalten",
        desc_en: "Toggle bridge structure on selection",
        default_key: "B",
    },
    KeyActionDef {
        action: KeyAction::ToggleTunnel,
        id: "toggleTunnel",
        category: KeyCategory::Tools,
        label_de: "Tunnel",
        label_en: "Toggle Tunnel",
        desc_de: "Tunnelabschnitt fuer Auswahl umschalten",
        desc_en: "Toggle tunnel section on selection",
        default_key: "T",
    },
    KeyActionDef {
        action: KeyAction::ToggleLantern,
        id: "toggleTurnoutLantern",
        category: KeyCategory::Tools,
        label_de: "Weichenlaterne",
        label_en: "Toggle Turnout Lantern",
        desc_de: "Beleuchtete Weichenlaterne",
        desc_en: "Toggle turnout lantern on switches",
        default_key: "Shift+L",
    },
    KeyActionDef {
        action: KeyAction::ToggleDecoder,
        id: "toggleTurnoutDecoder",
        category: KeyCategory::Tools,
        label_de: "Weichendecoder",
        label_en: "Toggle Turnout Decoder",
        desc_de: "Digital-Weichendecoder umschalten",
        desc_en: "Toggle digital turnout decoder",
        default_key: "Shift+D",
    },
    KeyActionDef {
        action: KeyAction::ToggleSignal,
        id: "toggleSignal",
        category: KeyCategory::Tools,
        label_de: "Lichtsignal am Gleis",
        label_en: "Toggle Trackside Signal",
        desc_de: "Signal an Auswahl umschalten",
        desc_en: "Toggle block signal on selection",
        default_key: "Shift+S",
    },
    KeyActionDef {
        action: KeyAction::OpenHelp,
        id: "openHelp",
        category: KeyCategory::View,
        label_de: "Hilfe & Anleitung",
        label_en: "Help & Guide",
        desc_de: "Oeffnet die Dokumentation",
        desc_en: "Open help & guide dialog",
        default_key: "F1",
    },
];

const KEY_TOKENS: &[(&str, egui::Key)] = &[
    ("A", egui::Key::A),
    ("B", egui::Key::B),
    ("C", egui::Key::C),
    ("D", egui::Key::D),
    ("E", egui::Key::E),
    ("F", egui::Key::F),
    ("G", egui::Key::G),
    ("H", egui::Key::H),
    ("I", egui::Key::I),
    ("J", egui::Key::J),
    ("K", egui::Key::K),
    ("L", egui::Key::L),
    ("M", egui::Key::M),
    ("N", egui::Key::N),
    ("O", egui::Key::O),
    ("P", egui::Key::P),
    ("Q", egui::Key::Q),
    ("R", egui::Key::R),
    ("S", egui::Key::S),
    ("T", egui::Key::T),
    ("U", egui::Key::U),
    ("V", egui::Key::V),
    ("W", egui::Key::W),
    ("X", egui::Key::X),
    ("Y", egui::Key::Y),
    ("Z", egui::Key::Z),
    ("0", egui::Key::Num0),
    ("1", egui::Key::Num1),
    ("2", egui::Key::Num2),
    ("3", egui::Key::Num3),
    ("4", egui::Key::Num4),
    ("5", egui::Key::Num5),
    ("6", egui::Key::Num6),
    ("7", egui::Key::Num7),
    ("8", egui::Key::Num8),
    ("9", egui::Key::Num9),
    ("F1", egui::Key::F1),
    ("F2", egui::Key::F2),
    ("F3", egui::Key::F3),
    ("F4", egui::Key::F4),
    ("F5", egui::Key::F5),
    ("F6", egui::Key::F6),
    ("F7", egui::Key::F7),
    ("F8", egui::Key::F8),
    ("F9", egui::Key::F9),
    ("F10", egui::Key::F10),
    ("F11", egui::Key::F11),
    ("F12", egui::Key::F12),
    ("Delete", egui::Key::Delete),
    ("Backspace", egui::Key::Backspace),
    ("Space", egui::Key::Space),
    ("Tab", egui::Key::Tab),
    ("Enter", egui::Key::Enter),
    ("Home", egui::Key::Home),
    ("End", egui::Key::End),
    ("PageUp", egui::Key::PageUp),
    ("PageDown", egui::Key::PageDown),
    ("Insert", egui::Key::Insert),
    ("Plus", egui::Key::Plus),
    ("Minus", egui::Key::Minus),
    ("Equals", egui::Key::Equals),
    ("Up", egui::Key::ArrowUp),
    ("Down", egui::Key::ArrowDown),
    ("Left", egui::Key::ArrowLeft),
    ("Right", egui::Key::ArrowRight),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: egui::Key,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCapture {
    Pending,
    Cancelled,
    Captured(String),
}

pub fn parse_shortcut(s: &str) -> Option<ShortcutSpec> {
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut key = None;

    for raw_part in s.split('+') {
        let part = raw_part.trim();
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "command" => ctrl = true,
            "shift" => shift = true,
            "alt" | "option" => alt = true,
            "del" | "delete" => key = Some(egui::Key::Delete),
            "arrowup" => key = Some(egui::Key::ArrowUp),
            "arrowdown" => key = Some(egui::Key::ArrowDown),
            "arrowleft" => key = Some(egui::Key::ArrowLeft),
            "arrowright" => key = Some(egui::Key::ArrowRight),
            other => {
                if let Some((_, k)) = KEY_TOKENS.iter().find(|(tok, _)| tok.eq_ignore_ascii_case(other)) {
                    key = Some(*k);
                } else {
                    return None;
                }
            }
        }
    }
    Some(ShortcutSpec {
        ctrl,
        shift,
        alt,
        key: key?,
    })
}

pub fn format_shortcut(spec: ShortcutSpec) -> String {
    let mut parts = Vec::new();
    if spec.ctrl {
        parts.push("Ctrl");
    }
    if spec.alt {
        parts.push("Alt");
    }
    if spec.shift {
        parts.push("Shift");
    }
    let key_tok = KEY_TOKENS
        .iter()
        .find(|(_, k)| *k == spec.key)
        .map(|(tok, _)| *tok)
        .unwrap_or("?");
    parts.push(key_tok);
    parts.join("+")
}

pub fn matches_input(binding: &str, i: &egui::InputState) -> bool {
    let Some(spec) = parse_shortcut(binding) else {
        return false;
    };
    i.modifiers.command == spec.ctrl
        && i.modifiers.shift == spec.shift
        && i.modifiers.alt == spec.alt
        && i.key_pressed(spec.key)
}

pub fn capture_pressed_shortcut(i: &egui::InputState) -> KeyCapture {
    if i.key_pressed(egui::Key::Escape) {
        return KeyCapture::Cancelled;
    }
    for (_, key) in KEY_TOKENS {
        if i.key_pressed(*key) {
            let formatted = format_shortcut(ShortcutSpec {
                ctrl: i.modifiers.command,
                shift: i.modifiers.shift,
                alt: i.modifiers.alt,
                key: *key,
            });
            return KeyCapture::Captured(formatted);
        }
    }
    KeyCapture::Pending
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub dark_mode: bool,
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
    pub viewer3d_env: String,
    pub startup_action: StartupAction,
    pub last_opened_path: Option<String>,
    pub perf_monitor_visible: bool,
    pub custom_keymap: BTreeMap<String, String>,
    pub favorite_tracks: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: true,
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
            viewer3d_env: "hall-landscape".into(),
            startup_action: StartupAction::Ask,
            last_opened_path: None,
            perf_monitor_visible: false,
            custom_keymap: BTreeMap::new(),
            favorite_tracks: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> Option<PathBuf> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("railbench").join("settings.json"));
        }
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".config").join("railbench").join("settings.json"))
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

    pub fn reset_key(&mut self, action: KeyAction) {
        if let Some(def) = KEY_ACTION_DEFS.iter().find(|d| d.action == action) {
            self.custom_keymap.remove(def.id);
        }
    }

    pub fn reset_all_keys(&mut self) {
        self.custom_keymap.clear();
    }

    pub fn reset_personal_defaults(&mut self) {
        let preserved = self.last_opened_path.clone();
        *self = Self::default();
        self.last_opened_path = preserved;
    }

    pub fn to_json_string(&self) -> String {
        let mut root = Map::new();
        root.insert("dark_mode".into(), Value::Bool(self.dark_mode));
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
        root.insert("viewer3d_env".into(), Value::String(self.viewer3d_env.clone()));
        root.insert("startup_action".into(), Value::String(self.startup_action.as_str().into()));
        root.insert("perf_monitor_visible".into(), Value::Bool(self.perf_monitor_visible));
        if let Some(ref p) = self.last_opened_path {
            root.insert("last_opened_path".into(), Value::String(p.clone()));
        }
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
        if let Some(v) = map.get("dark_mode").and_then(|v| v.as_bool()) {
            s.dark_mode = v;
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
    }
}
