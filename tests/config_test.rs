use ccmux::config::{Config, ActionMode};
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.default_strategy(), "auto-safe");
}

#[test]
fn test_load_config_from_toml() {
    let toml_content = r#"
[default]
strategy = "manual"

[strategy.auto-safe]
file_read = "auto"
file_write = "pause"
command_exec = "pause"
tool_use = "auto"
"#;
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();

    let config = Config::load_from(file.path()).unwrap();
    assert_eq!(config.default_strategy(), "manual");
    println!("Strategies: {:?}", config.strategy);
    assert!(config.strategy.contains_key("auto-safe"));
}

#[test]
fn test_strategy_action_mode() {
    assert_eq!(ActionMode::Auto.to_string(), "auto");
    assert_eq!(ActionMode::Pause.to_string(), "pause");
}
