//! Interaction mode detection from PTY output patterns
//!
//! This module analyzes PTY output to detect different interaction modes:
//! - Normal: standard command-line output
//! - Menu: interactive menus (often with reverse video ANSI)
//! - Editor: text editors (vim, nano, etc.)
//! - REPL: interactive shells and REPLs (python, node, etc.)

use crate::protocol::InteractionMode;
use once_cell::sync::Lazy;
use regex::Regex;

/// Detects interaction mode from PTY output patterns
#[derive(Debug, Clone)]
pub struct InteractionDetector {
    // No state needed for basic implementation
    // All patterns are compiled regexes
}

// ANSI escape sequences for reverse video (menu highlighting)
static REVERSE_VIDEO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\x1b\[[7m]").expect("Failed to compile reverse video regex")
});

// Editor signatures
static VIM_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new("(?i)VIM.*Vi.*IMproved").expect("Failed to compile vim regex")
});

static NANO_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"GNU nano \d+\.\d+").expect("Failed to compile nano regex")
});

static EMACS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"GNU Emacs").expect("Failed to compile emacs regex")
});

static EDITOR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(main\.py|\.rs|\.go|\.js|\.ts|README|TODO)\s+\d+,\s*\d+")
        .expect("Failed to compile editor line number regex")
});

// REPL signatures
static PYTHON_REPL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r">>> |In \[\d+\]: |   \.\.\. ").expect("Failed to compile python regex")
});

static NODE_REPL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"> ").expect("Failed to compile node regex")
});

static IPDB_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"ipdb> ").expect("Failed to compile ipdb regex")
});

static PDB_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\(Pdb\) ").expect("Failed to compile pdb regex")
});

static RUST_REPL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"dbg> ").expect("Failed to compile rust regex")
});

// Menu patterns beyond reverse video
static MENU_OPTIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*\d+\.\s+[A-Z]|\s*\[.\]\s+\w+").expect("Failed to compile menu options regex")
});

impl Default for InteractionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionDetector {
    /// Create a new interaction detector
    pub fn new() -> Self {
        Self {}
    }

    /// Detect interaction mode from PTY output
    ///
    /// # Arguments
    /// * `output` - The PTY output to analyze
    /// * `current_mode` - The current interaction mode (for persistence)
    ///
    /// # Returns
    /// The detected interaction mode
    pub fn detect(&self, output: &str, current_mode: InteractionMode) -> InteractionMode {
        // Strip ANSI codes for pattern matching (but we keep original for some patterns)
        let clean_output = strip_ansi::strip_ansi(output);

        // Mode persistence: once in a special mode, stay there unless clear exit signal
        match current_mode {
            InteractionMode::Menu => {
                // Check for menu exit signals
                if self.is_menu_exit(&clean_output) {
                    return InteractionMode::Normal;
                }
                // Otherwise stay in menu mode
                return InteractionMode::Menu;
            }
            InteractionMode::Editor => {
                // Check for editor exit signals
                if self.is_editor_exit(&clean_output) {
                    return InteractionMode::Normal;
                }
                // Otherwise stay in editor mode
                return InteractionMode::Editor;
            }
            InteractionMode::Repl => {
                // Check for REPL exit signals
                if self.is_repl_exit(&clean_output) {
                    return InteractionMode::Normal;
                }
                // Otherwise stay in REPL mode
                return InteractionMode::Repl;
            }
            InteractionMode::Normal => {
                // Detect new mode transitions
            }
        }

        // Check for editor patterns (highest priority)
        if self.is_editor_pattern(output, &clean_output) {
            return InteractionMode::Editor;
        }

        // Check for REPL patterns
        if self.is_repl_pattern(&clean_output) {
            return InteractionMode::Repl;
        }

        // Check for menu patterns
        if self.is_menu_pattern(output) {
            return InteractionMode::Menu;
        }

        // Default to normal mode
        InteractionMode::Normal
    }

    /// Check if output indicates menu mode
    fn is_menu_pattern(&self, output: &str) -> bool {
        // Reverse video is the strongest menu indicator
        if REVERSE_VIDEO.is_match(output) {
            return true;
        }

        // Check for numbered options or [x] style options
        if MENU_OPTIONS.is_match(output) {
            return true;
        }

        false
    }

    /// Check if output indicates editor mode
    fn is_editor_pattern(&self, raw_output: &str, clean_output: &str) -> bool {
        // Check for editor signatures
        if VIM_PATTERN.is_match(clean_output) {
            return true;
        }

        if NANO_PATTERN.is_match(clean_output) {
            return true;
        }

        if EMACS_PATTERN.is_match(clean_output) {
            return true;
        }

        // Check for file with line numbers (common editor status line)
        if EDITOR_PATTERN.is_match(clean_output) {
            return true;
        }

        // Check for vim-specific status line patterns
        // Example: "main.py" 12L, 234C
        if raw_output.contains("\"") && Regex::new(r"\d+L,\s*\d+C").unwrap().is_match(raw_output) {
            return true;
        }

        false
    }

    /// Check if output indicates REPL mode
    fn is_repl_pattern(&self, output: &str) -> bool {
        if PYTHON_REPL.is_match(output) {
            return true;
        }

        if NODE_REPL.is_match(output) {
            return true;
        }

        if IPDB_PATTERN.is_match(output) {
            return true;
        }

        if PDB_PATTERN.is_match(output) {
            return true;
        }

        if RUST_REPL.is_match(output) {
            return true;
        }

        false
    }

    /// Check if output indicates exiting menu mode
    fn is_menu_exit(&self, output: &str) -> bool {
        // If we see a normal command prompt, menu is done
        if output.contains("$ ") || output.contains("% ") || output.contains("> ") {
            return true;
        }

        // If we see "exiting" or similar messages
        let lower = output.to_lowercase();
        if lower.contains("exiting") || lower.contains("cancelled") || lower.contains("abort") {
            return true;
        }

        false
    }

    /// Check if output indicates exiting editor mode
    fn is_editor_exit(&self, output: &str) -> bool {
        // If we see a normal command prompt, editor is done
        if output.contains("$ ") || output.contains("% ") {
            return true;
        }

        // Vim exit messages
        if output.contains("Vim: Warning: Output not to terminal") {
            return true;
        }

        false
    }

    /// Check if output indicates exiting REPL mode
    fn is_repl_exit(&self, output: &str) -> bool {
        // If we see exit() or similar
        let lower = output.to_lowercase();
        if lower.contains("exit()") || lower.contains("quit()") {
            return true;
        }

        // If we see a normal command prompt
        if output.contains("$ ") || output.contains("% ") {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_video_detection() {
        let detector = InteractionDetector::new();
        assert!(detector.is_menu_pattern("\x1b[7m Highlighted \x1b[0m"));
    }

    #[test]
    fn test_vim_detection() {
        let detector = InteractionDetector::new();
        let output = "VIM - Vi IMproved 9.0";
        assert!(detector.is_editor_pattern(output, output));
    }

    #[test]
    fn test_nano_detection() {
        let detector = InteractionDetector::new();
        let output = "GNU nano 7.2";
        assert!(detector.is_editor_pattern(output, output));
    }

    #[test]
    fn test_python_repl_detection() {
        let detector = InteractionDetector::new();
        assert!(detector.is_repl_pattern(">>> "));
        assert!(detector.is_repl_pattern("In [1]: "));
    }
}
