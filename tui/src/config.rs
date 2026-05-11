use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    // Respect XDG_CONFIG_HOME on all platforms.
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("binocular");
    }

    // On macOS, prefer ~/.config over ~/Library/Application Support.
    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("binocular");
    }

    #[cfg(target_os = "windows")]
    if let Ok(app_data) = std::env::var("APPDATA") {
        return PathBuf::from(app_data).join("binocular");
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("binocular");
    }

    PathBuf::from(".").join("binocular")
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    fn matches(&self, key: &KeyEvent) -> bool {
        self.code == key.code && self.modifiers == key.modifiers
    }
}

pub fn format_keybinding(binding: &KeyBinding) -> String {
    let mut parts = Vec::new();
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    parts.push(match binding.code {
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "Shift+Tab".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(ch) => ch.to_ascii_uppercase().to_string(),
        KeyCode::F(n) => format!("F{n}"),
        _ => format!("{:?}", binding.code),
    });
    parts.join("+")
}

pub fn format_keybindings(bindings: &[KeyBinding]) -> String {
    bindings
        .iter()
        .map(format_keybinding)
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn kb_matches(bindings: &[KeyBinding], key: &KeyEvent) -> bool {
    bindings.iter().any(|b| b.matches(key))
}

pub fn parse_key(s: &str) -> Result<KeyBinding, String> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.is_empty() {
        return Err("empty key string".into());
    }

    let mut modifiers = KeyModifiers::empty();
    for &part in &parts[..parts.len() - 1] {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" | "meta" => modifiers |= KeyModifiers::ALT,
            other => return Err(format!("unknown modifier '{other}'")),
        }
    }

    let key_str = parts[parts.len() - 1].to_ascii_lowercase();
    let code = match key_str.as_str() {
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "pageup" | "page_up" => KeyCode::PageUp,
        "pagedown" | "page_down" => KeyCode::PageDown,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "insert" | "ins" => KeyCode::Insert,
        "space" => KeyCode::Char(' '),
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        c if c.chars().count() == 1 => KeyCode::Char(c.chars().next().unwrap()),
        other => return Err(format!("unknown key '{other}'")),
    };

    Ok(KeyBinding { code, modifiers })
}

fn parse_key_list(strings: Vec<String>, action: &str) -> Vec<KeyBinding> {
    strings
        .into_iter()
        .filter_map(|s| {
            parse_key(&s)
                .map_err(|e| eprintln!("binocular: keybinding '{action}': {e}"))
                .ok()
        })
        .collect()
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(s) => vec![s],
            Self::Many(v) => v,
        }
    }
}

// ── Raw config struct (serde) ─────────────────────────────────────────────────

#[derive(Deserialize, Default)]
#[serde(default)]
struct KeybindingsConfig {
    quit: Option<OneOrMany>,
    toggle_help: Option<OneOrMany>,
    toggle_preview_focus: Option<OneOrMany>,
    toggle_preview_fullscreen: Option<OneOrMany>,
    swap_panes: Option<OneOrMany>,
    preview_wider: Option<OneOrMany>,
    preview_narrower: Option<OneOrMany>,
    toggle_search_bar_position: Option<OneOrMany>,
    toggle_preview_visibility: Option<OneOrMany>,
    toggle_exact: Option<OneOrMany>,
    mode_path: Option<OneOrMany>,
    mode_files: Option<OneOrMany>,
    mode_grep: Option<OneOrMany>,
    mode_dirs: Option<OneOrMany>,
    scroll_preview_up: Option<OneOrMany>,
    scroll_preview_down: Option<OneOrMany>,
    mark_result: Option<OneOrMany>,
    mark_diff_result: Option<OneOrMany>,
    select_from_preview: Option<OneOrMany>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawAppConfig {
    keybindings: KeybindingsConfig,
    log: LogConfig,
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    /// Maximum number of log entries kept in memory (initial load + streaming).
    pub max_entries: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
        }
    }
}

#[derive(Clone)]
pub struct Keybindings {
    pub quit: Vec<KeyBinding>,
    pub toggle_help: Vec<KeyBinding>,
    pub toggle_preview_focus: Vec<KeyBinding>,
    pub toggle_preview_fullscreen: Vec<KeyBinding>,
    pub swap_panes: Vec<KeyBinding>,
    pub preview_wider: Vec<KeyBinding>,
    pub preview_narrower: Vec<KeyBinding>,
    pub toggle_search_bar_position: Vec<KeyBinding>,
    pub toggle_preview_visibility: Vec<KeyBinding>,
    pub toggle_exact: Vec<KeyBinding>,
    pub mode_path: Vec<KeyBinding>,
    pub mode_files: Vec<KeyBinding>,
    pub mode_grep: Vec<KeyBinding>,
    pub mode_dirs: Vec<KeyBinding>,
    pub scroll_preview_up: Vec<KeyBinding>,
    pub scroll_preview_down: Vec<KeyBinding>,
    pub mark_result: Vec<KeyBinding>,
    pub mark_diff_result: Vec<KeyBinding>,
    pub select_from_preview: Vec<KeyBinding>,
}

fn single(code: KeyCode, modifiers: KeyModifiers) -> Vec<KeyBinding> {
    vec![KeyBinding { code, modifiers }]
}

impl Default for Keybindings {
    fn default() -> Self {
        use KeyCode::*;
        use KeyModifiers as M;
        Self {
            quit: single(Char('c'), M::CONTROL),
            toggle_help: single(Char('h'), M::CONTROL),
            toggle_preview_focus: single(Char('w'), M::CONTROL),
            toggle_preview_fullscreen: single(Char('f'), M::CONTROL),
            swap_panes: single(Char('e'), M::CONTROL),
            preview_wider: single(Char('p'), M::CONTROL),
            preview_narrower: single(Char('n'), M::CONTROL),
            toggle_search_bar_position: single(Char('t'), M::CONTROL),
            toggle_preview_visibility: single(Char('b'), M::CONTROL),
            toggle_exact: single(Char('x'), M::CONTROL),
            mode_path: single(F(1), M::NONE),
            mode_files: single(F(2), M::NONE),
            mode_grep: single(F(3), M::NONE),
            mode_dirs: single(F(4), M::NONE),
            scroll_preview_up: vec![
                KeyBinding {
                    code: PageUp,
                    modifiers: M::NONE,
                },
                KeyBinding {
                    code: Char('u'),
                    modifiers: M::CONTROL,
                },
            ],
            scroll_preview_down: vec![
                KeyBinding {
                    code: PageDown,
                    modifiers: M::NONE,
                },
                KeyBinding {
                    code: Char('d'),
                    modifiers: M::CONTROL,
                },
            ],
            mark_result: single(Tab, M::NONE),
            mark_diff_result: single(F(5), M::NONE),
            select_from_preview: single(Enter, M::NONE),
        }
    }
}

impl Keybindings {
    fn from_config(cfg: KeybindingsConfig) -> Self {
        let d = Self::default();

        macro_rules! resolve {
            ($field:ident) => {
                match cfg.$field {
                    None => d.$field,
                    Some(raw) => {
                        let parsed = parse_key_list(raw.into_vec(), stringify!($field));
                        if parsed.is_empty() {
                            d.$field
                        } else {
                            parsed
                        }
                    }
                }
            };
        }

        Self {
            quit: resolve!(quit),
            toggle_help: resolve!(toggle_help),
            toggle_preview_focus: resolve!(toggle_preview_focus),
            toggle_preview_fullscreen: resolve!(toggle_preview_fullscreen),
            swap_panes: resolve!(swap_panes),
            preview_wider: resolve!(preview_wider),
            preview_narrower: resolve!(preview_narrower),
            toggle_search_bar_position: resolve!(toggle_search_bar_position),
            toggle_preview_visibility: resolve!(toggle_preview_visibility),
            toggle_exact: resolve!(toggle_exact),
            mode_path: resolve!(mode_path),
            mode_files: resolve!(mode_files),
            mode_grep: resolve!(mode_grep),
            mode_dirs: resolve!(mode_dirs),
            scroll_preview_up: resolve!(scroll_preview_up),
            scroll_preview_down: resolve!(scroll_preview_down),
            mark_result: resolve!(mark_result),
            mark_diff_result: resolve!(mark_diff_result),
            select_from_preview: resolve!(select_from_preview),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct PersistedLayout {
    pub panes_swapped: bool,
    pub preview_percent: u16,
    pub search_bar_at_bottom: bool,
    pub preview_hidden: bool,
}

impl Default for PersistedLayout {
    fn default() -> Self {
        Self {
            panes_swapped: false,
            preview_percent: 50,
            search_bar_at_bottom: false,
            preview_hidden: false,
        }
    }
}

pub fn load_layout() -> PersistedLayout {
    let path = config_dir().join("layout.toml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return PersistedLayout::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}

pub fn save_layout(layout: &PersistedLayout) {
    let dir = config_dir();
    let path = dir.join("layout.toml");
    if let Ok(content) = toml::to_string(layout) {
        let _ = std::fs::write(path, content);
    }
}

const DEFAULT_CONFIG: &str = include_str!("../config/default.toml");

#[derive(Clone, Default)]
pub struct LoadedAppConfig {
    pub keybindings: Keybindings,
    pub log: LogConfig,
}

fn ensure_config_file() -> PathBuf {
    let dir = config_dir();
    let path = dir.join("config.toml");

    if !path.exists() {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!(
                "binocular: could not create config directory {}: {e}",
                dir.display()
            );
        } else if let Err(e) = std::fs::write(&path, DEFAULT_CONFIG) {
            eprintln!(
                "binocular: could not write default config {}: {e}",
                path.display()
            );
        }
    }

    path
}

pub fn load_app_config() -> LoadedAppConfig {
    let path = ensure_config_file();
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            return LoadedAppConfig {
                keybindings: Keybindings::default(),
                log: LogConfig::default(),
            };
        }
    };
    let cfg: RawAppConfig = toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("binocular: error reading config {}: {e}", path.display());
        RawAppConfig::default()
    });

    LoadedAppConfig {
        keybindings: Keybindings::from_config(cfg.keybindings),
        log: cfg.log,
    }
}

pub fn load_keybindings() -> Keybindings {
    load_app_config().keybindings
}

pub fn load_log_max_entries() -> usize {
    load_app_config().log.max_entries
}
