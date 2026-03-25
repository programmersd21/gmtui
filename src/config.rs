use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use directories::BaseDirs;
use ratatui::style::Color;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: Theme,
    pub keybindings: Keybindings,
    pub page_size: usize,
    pub cache_ttl_secs: u64,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub primary_read: Color,
    pub primary_unread: Color,
    pub social: Color,
    pub promotions: Color,
    pub updates: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub status_bar_bg: Color,
    pub header_fg: Color,
}

#[derive(Debug, Clone)]
pub struct Keybindings {
    pub down: KeyCode,
    pub up: KeyCode,
    pub open: KeyCode,
    pub compose: KeyCode,
    pub reply: KeyCode,
    pub delete: KeyCode,
    pub search: KeyCode,
    pub refresh: KeyCode,
    pub quit: KeyCode,
    pub next_tab: KeyCode,
    pub prev_tab: KeyCode,
    pub load_more: KeyCode,
    pub help: KeyCode,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub token_cache_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Down,
    Up,
    Open,
    Compose,
    Reply,
    Delete,
    Search,
    Refresh,
    Quit,
    NextTab,
    PrevTab,
    LoadMore,
    Help,
}

impl Theme {
    pub const fn dark() -> Theme {
        Theme {
            name: String::new(),
            primary_read: Color::Rgb(160, 166, 176),
            primary_unread: Color::Rgb(248, 250, 252),
            social: Color::Rgb(96, 165, 250),
            promotions: Color::Rgb(244, 114, 182),
            updates: Color::Rgb(251, 191, 36),
            selected_bg: Color::Rgb(56, 189, 248),
            selected_fg: Color::Rgb(15, 23, 42),
            status_bar_bg: Color::Rgb(30, 41, 59),
            header_fg: Color::Rgb(148, 163, 184),
        }
    }

    pub const fn light() -> Theme {
        Theme {
            name: String::new(),
            primary_read: Color::Rgb(71, 85, 105),
            primary_unread: Color::Rgb(15, 23, 42),
            social: Color::Rgb(2, 132, 199),
            promotions: Color::Rgb(219, 39, 119),
            updates: Color::Rgb(180, 83, 9),
            selected_bg: Color::Rgb(59, 130, 246),
            selected_fg: Color::Rgb(255, 255, 255),
            status_bar_bg: Color::Rgb(226, 232, 240),
            header_fg: Color::Rgb(100, 116, 139),
        }
    }
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            down: KeyCode::Char('j'),
            up: KeyCode::Char('k'),
            open: KeyCode::Enter,
            compose: KeyCode::Char('c'),
            reply: KeyCode::Char('r'),
            delete: KeyCode::Char('d'),
            search: KeyCode::Char('/'),
            refresh: KeyCode::Char('R'),
            quit: KeyCode::Char('q'),
            next_tab: KeyCode::Tab,
            prev_tab: KeyCode::BackTab,
            load_more: KeyCode::Char('L'),
            help: KeyCode::Char('?'),
        }
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        let mut path = config_base_dir();
        path.push("gmtui");
        path.push("config.toml");
        path
    }

    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let parsed: ConfigFile = toml::from_str(&content)?;
        let theme = match parsed.theme.as_deref() {
            Some("light") => Theme::light().named("light"),
            Some("dark") => Theme::dark().named("dark"),
            Some(_) => Theme::dark().named("dark"),
            None => Theme::dark().named("dark"),
        };
        let keybindings = Keybindings::from_file(parsed.keybindings);
        Ok(Self {
            theme,
            keybindings,
            page_size: parsed.page_size.unwrap_or(20),
            cache_ttl_secs: parsed.cache_ttl_secs.unwrap_or(300),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::dark().named("dark"),
            keybindings: Keybindings::default(),
            page_size: 20,
            cache_ttl_secs: 300,
        }
    }
}

impl Theme {
    fn named(mut self, name: &str) -> Theme {
        self.name = name.to_string();
        self
    }
}

impl AuthConfig {
    pub fn load() -> Result<Self> {
        let path = Config::default_path();
        let mut file_cfg = None;
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let parsed: ConfigFile = toml::from_str(&content)?;
            file_cfg = Some(parsed);
        }
        let (client_id, client_secret, token_cache_path) = if let Some(cfg) = file_cfg {
            let client_id = cfg.client_id.or_else(|| env::var("GMTUI_CLIENT_ID").ok());
            let client_secret = cfg
                .client_secret
                .or_else(|| env::var("GMTUI_CLIENT_SECRET").ok());
            let token_cache_path = cfg
                .token_cache_path
                .or_else(|| env::var("GMTUI_TOKEN_CACHE_PATH").ok());
            (client_id, client_secret, token_cache_path)
        } else {
            (
                env::var("GMTUI_CLIENT_ID").ok(),
                env::var("GMTUI_CLIENT_SECRET").ok(),
                env::var("GMTUI_TOKEN_CACHE_PATH").ok(),
            )
        };

        let client_id = client_id.ok_or_else(|| anyhow!("missing client_id"))?;
        let client_secret = client_secret.ok_or_else(|| anyhow!("missing client_secret"))?;

        let mut config_dir = config_base_dir();
        config_dir.push("gmtui");
        let token_cache_path = token_cache_path
            .map(PathBuf::from)
            .unwrap_or_else(|| config_dir.join("token.json"));

        Ok(Self {
            client_id,
            client_secret,
            token_cache_path,
        })
    }
}

impl Keybindings {
    pub fn matches(&self, action: Action, key: KeyEvent) -> bool {
        match action {
            Action::Down => self.matches_with_fallback(key, self.down, KeyCode::Down),
            Action::Up => self.matches_with_fallback(key, self.up, KeyCode::Up),
            Action::Open => self.matches_code(key, self.open),
            Action::Compose => self.matches_code(key, self.compose),
            Action::Reply => self.matches_code(key, self.reply),
            Action::Delete => self.matches_code(key, self.delete),
            Action::Search => self.matches_code(key, self.search),
            Action::Refresh => self.matches_code(key, self.refresh),
            Action::Quit => self.matches_code(key, self.quit),
            Action::NextTab => self.matches_code(key, self.next_tab),
            Action::PrevTab => self.matches_code(key, self.prev_tab),
            Action::LoadMore => self.matches_code(key, self.load_more),
            Action::Help => self.matches_code(key, self.help),
        }
    }

    fn matches_code(&self, key: KeyEvent, binding: KeyCode) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        key.code == binding
    }

    fn matches_with_fallback(&self, key: KeyEvent, primary: KeyCode, fallback: KeyCode) -> bool {
        if self.matches_code(key, primary) {
            return true;
        }
        if primary == KeyCode::Char('j') && key.code == fallback {
            return true;
        }
        if primary == KeyCode::Char('k') && key.code == fallback {
            return true;
        }
        key.code == fallback && primary == fallback
    }

    fn from_file(file: Option<KeybindingsFile>) -> Self {
        let defaults = Keybindings::default();
        if let Some(file) = file {
            return Self {
                down: parse_keycode(file.down).unwrap_or(defaults.down),
                up: parse_keycode(file.up).unwrap_or(defaults.up),
                open: parse_keycode(file.open).unwrap_or(defaults.open),
                compose: parse_keycode(file.compose).unwrap_or(defaults.compose),
                reply: parse_keycode(file.reply).unwrap_or(defaults.reply),
                delete: parse_keycode(file.delete).unwrap_or(defaults.delete),
                search: parse_keycode(file.search).unwrap_or(defaults.search),
                refresh: parse_keycode(file.refresh).unwrap_or(defaults.refresh),
                quit: parse_keycode(file.quit).unwrap_or(defaults.quit),
                next_tab: parse_keycode(file.next_tab).unwrap_or(defaults.next_tab),
                prev_tab: parse_keycode(file.prev_tab).unwrap_or(defaults.prev_tab),
                load_more: parse_keycode(file.load_more).unwrap_or(defaults.load_more),
                help: parse_keycode(file.help).unwrap_or(defaults.help),
            };
        }
        defaults
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    theme: Option<String>,
    page_size: Option<usize>,
    cache_ttl_secs: Option<u64>,
    keybindings: Option<KeybindingsFile>,
    client_id: Option<String>,
    client_secret: Option<String>,
    token_cache_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeybindingsFile {
    down: Option<String>,
    up: Option<String>,
    open: Option<String>,
    compose: Option<String>,
    reply: Option<String>,
    delete: Option<String>,
    search: Option<String>,
    refresh: Option<String>,
    quit: Option<String>,
    next_tab: Option<String>,
    prev_tab: Option<String>,
    load_more: Option<String>,
    help: Option<String>,
}

fn config_base_dir() -> PathBuf {
    if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = env::var("APPDATA") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = env::var("LOCALAPPDATA") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = env::var("HOME") {
        return PathBuf::from(dir).join(".config");
    }
    if let Some(base) = BaseDirs::new() {
        return base.config_dir().to_path_buf();
    }
    PathBuf::from(".")
}

fn parse_keycode(input: Option<String>) -> Option<KeyCode> {
    let raw = input?;
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_lowercase();
    let key = match lower.as_str() {
        "down" => KeyCode::Down,
        "up" => KeyCode::Up,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "enter" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" | "shift+tab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        _ => {
            if value.len() == 1 {
                return value.chars().next().map(KeyCode::Char);
            }
            return None;
        }
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_colors_differ_from_light() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert_ne!(dark.primary_unread, light.primary_unread);
    }
}
