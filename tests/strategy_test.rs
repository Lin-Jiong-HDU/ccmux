use ccmux::config::{ActionMode, StrategyConfig};
use ccmux::server::{Strategy, StrategyEngine};

#[test]
fn test_strategy_auto_safe() {
    let config = StrategyConfig {
        file_read: ActionMode::Auto,
        file_write: ActionMode::Pause,
        command_exec: ActionMode::Pause,
        tool_use: ActionMode::Auto,
        bypass_permissions: false,
    };

    let strategy = Strategy::from_config(config);
    assert_eq!(strategy.should_pause_on_file_read(), false);
    assert_eq!(strategy.should_pause_on_file_write(), true);
    assert_eq!(strategy.should_pause_on_command_exec(), true);
    assert_eq!(strategy.should_pause_on_tool_use(), false);
}

#[test]
fn test_strategy_auto_all() {
    let config = StrategyConfig {
        file_read: ActionMode::Auto,
        file_write: ActionMode::Auto,
        command_exec: ActionMode::Auto,
        tool_use: ActionMode::Auto,
        bypass_permissions: false,
    };

    let strategy = Strategy::from_config(config);
    assert_eq!(strategy.should_pause_on_file_read(), false);
    assert_eq!(strategy.should_pause_on_file_write(), false);
    assert_eq!(strategy.should_pause_on_command_exec(), false);
    assert_eq!(strategy.should_pause_on_tool_use(), false);
}

#[test]
fn test_strategy_manual() {
    let config = StrategyConfig {
        file_read: ActionMode::Pause,
        file_write: ActionMode::Pause,
        command_exec: ActionMode::Pause,
        tool_use: ActionMode::Pause,
        bypass_permissions: false,
    };

    let strategy = Strategy::from_config(config);
    assert_eq!(strategy.should_pause_on_file_read(), true);
    assert_eq!(strategy.should_pause_on_file_write(), true);
    assert_eq!(strategy.should_pause_on_command_exec(), true);
    assert_eq!(strategy.should_pause_on_tool_use(), true);
}

#[test]
fn test_strategy_evaluate() {
    let config = StrategyConfig {
        file_read: ActionMode::Auto,
        file_write: ActionMode::Pause,
        command_exec: ActionMode::Pause,
        tool_use: ActionMode::Auto,
        bypass_permissions: false,
    };
    let strategy = Strategy::from_config(config);

    let result = strategy.evaluate("file_read");
    assert_eq!(result.should_pause, false);

    let result = strategy.evaluate("file_write");
    assert_eq!(result.should_pause, true);
}

#[test]
fn test_strategy_engine() {
    let mut engine = StrategyEngine::new();

    let config = StrategyConfig {
        file_read: ActionMode::Auto,
        file_write: ActionMode::Pause,
        command_exec: ActionMode::Pause,
        tool_use: ActionMode::Auto,
        bypass_permissions: false,
    };

    engine.add_strategy("auto-safe".to_string(), Strategy::from_config(config));

    let strategy = engine.get_strategy("auto-safe");
    assert!(strategy.is_some());
}
