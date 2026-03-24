//! Configuration management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Action mode for strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ActionMode {
    Auto,
    #[default]
    Pause,
}

impl std::fmt::Display for ActionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Pause => write!(f, "pause"),
        }
    }
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyConfig {
    #[serde(default)]
    pub file_read: ActionMode,
    #[serde(default)]
    pub file_write: ActionMode,
    #[serde(default)]
    pub command_exec: ActionMode,
    #[serde(default)]
    pub tool_use: ActionMode,
}

/// Default configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            strategy: default_strategy(),
        }
    }
}

fn default_strategy() -> String {
    "auto-safe".to_string()
}

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default: DefaultConfig,
    #[serde(default)]
    pub strategy: HashMap<String, StrategyConfig>,
}

impl Config {
    /// Get the default strategy name
    pub fn default_strategy(&self) -> &str {
        &self.default.strategy
    }

    /// Get all strategies
    pub fn strategies(&self) -> &HashMap<String, StrategyConfig> {
        &self.strategy
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut strategy = HashMap::new();

        strategy.insert(
            "auto-safe".to_string(),
            StrategyConfig {
                file_read: ActionMode::Auto,
                file_write: ActionMode::Pause,
                command_exec: ActionMode::Pause,
                tool_use: ActionMode::Auto,
            },
        );

        strategy.insert(
            "auto-all".to_string(),
            StrategyConfig {
                file_read: ActionMode::Auto,
                file_write: ActionMode::Auto,
                command_exec: ActionMode::Auto,
                tool_use: ActionMode::Auto,
            },
        );

        strategy.insert(
            "manual".to_string(),
            StrategyConfig {
                file_read: ActionMode::Pause,
                file_write: ActionMode::Pause,
                command_exec: ActionMode::Pause,
                tool_use: ActionMode::Pause,
            },
        );

        Self {
            default: DefaultConfig {
                strategy: default_strategy(),
            },
            strategy,
        }
    }
}

impl Config {
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn get_strategy(&self, name: &str) -> Option<&StrategyConfig> {
        self.strategy.get(name)
    }

    pub fn config_dir() -> anyhow::Result<std::path::PathBuf> {
        let base = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;
        Ok(base.join("ccmux"))
    }

    pub fn config_path() -> anyhow::Result<std::path::PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            Self::load_from(path)
        } else {
            Ok(Self::default())
        }
    }
}
