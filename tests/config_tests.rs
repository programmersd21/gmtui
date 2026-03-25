use gmtui::config::Config;
use std::env;
use std::fs;

fn set_env(key: &str, value: &str) -> Option<String> {
    let prev = env::var(key).ok();
    env::set_var(key, value);
    prev
}

fn restore_env(key: &str, prev: Option<String>) {
    if let Some(val) = prev {
        env::set_var(key, val);
    } else {
        env::remove_var(key);
    }
}

#[test]
fn default_path_resolves() {
    let path = Config::default_path();
    let normalized = path.to_string_lossy().replace('\\', "/");
    assert!(normalized.ends_with("gmtui/config.toml"));
}

#[test]
fn load_returns_default_on_missing_file() {
    let temp = env::temp_dir().join(format!("gmtui_cfg_missing_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp);

    let prev_appdata = set_env("APPDATA", temp.to_string_lossy().as_ref());
    let prev_local = set_env("LOCALAPPDATA", temp.to_string_lossy().as_ref());
    let prev_xdg = set_env("XDG_CONFIG_HOME", temp.to_string_lossy().as_ref());
    let prev_home = set_env("HOME", temp.to_string_lossy().as_ref());

    let result = Config::load();

    restore_env("APPDATA", prev_appdata);
    restore_env("LOCALAPPDATA", prev_local);
    restore_env("XDG_CONFIG_HOME", prev_xdg);
    restore_env("HOME", prev_home);

    let cfg = result.expect("config should load defaults");
    assert_eq!(cfg.page_size, 20);
    assert_eq!(cfg.cache_ttl_secs, 300);
}

#[test]
fn load_overrides_values() {
    let temp = env::temp_dir().join(format!("gmtui_cfg_min_{}", std::process::id()));
    let config_dir = temp.join("gmtui");
    let _ = fs::create_dir_all(&config_dir);
    let config_path = config_dir.join("config.toml");

    fs::write(
        &config_path,
        "theme = \"light\"\npage_size = 10\ncache_ttl_secs = 42\n",
    )
    .unwrap();

    let prev_appdata = set_env("APPDATA", temp.to_string_lossy().as_ref());
    let prev_local = set_env("LOCALAPPDATA", temp.to_string_lossy().as_ref());
    let prev_xdg = set_env("XDG_CONFIG_HOME", temp.to_string_lossy().as_ref());
    let prev_home = set_env("HOME", temp.to_string_lossy().as_ref());

    let cfg = Config::load().expect("config should load");

    restore_env("APPDATA", prev_appdata);
    restore_env("LOCALAPPDATA", prev_local);
    restore_env("XDG_CONFIG_HOME", prev_xdg);
    restore_env("HOME", prev_home);

    assert_eq!(cfg.page_size, 10);
    assert_eq!(cfg.cache_ttl_secs, 42);
    assert_eq!(cfg.theme.name, "light");
}
