//! Strategy engine for auto/pause behavior

use crate::config::{ActionMode, StrategyConfig};

/// Evaluation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyResult {
    pub should_pause: bool,
    pub action: ActionMode,
}

/// Strategy evaluator
#[derive(Debug, Clone)]
pub struct Strategy {
    config: StrategyConfig,
}

impl Strategy {
    pub fn from_config(config: StrategyConfig) -> Self {
        Self { config }
    }

    pub fn should_pause_on_file_read(&self) -> bool {
        matches!(self.config.file_read, ActionMode::Pause)
    }

    pub fn should_pause_on_file_write(&self) -> bool {
        matches!(self.config.file_write, ActionMode::Pause)
    }

    pub fn should_pause_on_command_exec(&self) -> bool {
        matches!(self.config.command_exec, ActionMode::Pause)
    }

    pub fn should_pause_on_tool_use(&self) -> bool {
        matches!(self.config.tool_use, ActionMode::Pause)
    }

    pub fn evaluate(&self, event: &str) -> StrategyResult {
        let (should_pause, action) = match event {
            "file_read" => (self.should_pause_on_file_read(), self.config.file_read),
            "file_write" => (self.should_pause_on_file_write(), self.config.file_write),
            "command_exec" => (
                self.should_pause_on_command_exec(),
                self.config.command_exec,
            ),
            "tool_use" => (self.should_pause_on_tool_use(), self.config.tool_use),
            _ => (true, ActionMode::Pause), // Default to pause for unknown events
        };

        StrategyResult {
            should_pause,
            action,
        }
    }
}

/// Strategy engine with multiple strategies
#[derive(Debug, Clone)]
pub struct StrategyEngine {
    strategies: std::collections::HashMap<String, Strategy>,
}

impl StrategyEngine {
    pub fn new() -> Self {
        Self {
            strategies: std::collections::HashMap::new(),
        }
    }

    pub fn add_strategy(&mut self, name: String, strategy: Strategy) {
        self.strategies.insert(name, strategy);
    }

    pub fn get_strategy(&self, name: &str) -> Option<&Strategy> {
        self.strategies.get(name)
    }

    pub fn remove_strategy(&mut self, name: &str) -> Option<Strategy> {
        self.strategies.remove(name)
    }

    /// Load strategies from config
    pub fn from_config(config: &crate::config::Config) -> Self {
        let mut engine = Self::new();

        for (name, strategy_config) in config.strategies() {
            engine.add_strategy(name.clone(), Strategy::from_config(strategy_config.clone()));
        }

        engine
    }
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new()
    }
}
