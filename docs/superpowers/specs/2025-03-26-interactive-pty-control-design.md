# Enhanced PTY Interaction Control Design

**Date:** 2025-03-26
**Status:** Draft
**Author:** ccmux team

## Overview

This document describes the design for enhanced PTY interaction control in ccmux, enabling Claude Code to effectively control other Claude Code instances for orchestration and automation.

### Problem Statement

The current `send` method simply writes text + `\r` to the PTY, which works for simple commands but fails with interactive prompts like `/help claude code` that display interactive interfaces requiring menu navigation (arrow keys + Enter).

### Goals

1. Support sending control keys (arrow keys, Enter, Esc, Ctrl+C, etc.)
2. Capture and parse screen content including ANSI escape sequences
3. Detect interaction modes (normal, menu, editor, REPL)
4. Maintain simplicity and consistency with existing codebase
5. Provide both CLI and programmatic interfaces

## Architecture

### Current Architecture

```
Client → Unix Socket → Daemon → Session → Pty
```

### Enhanced Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Client Layer                          │
│  send_key(), get_screen(), expect()                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Protocol Layer                           │
│  Request::{SendKey, GetScreen, Expect}                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Session Layer                            │
│  ScreenBuffer + InteractionDetector                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       PTY Layer                             │
│  send_raw(bytes)                                           │
└─────────────────────────────────────────────────────────────┘
```

## Protocol Layer

### New Request Types

```rust
pub enum Request {
    // ... existing requests ...

    /// Send control key
    #[serde(rename = "send_key")]
    SendKey {
        session: String,
        key: Key,
    },

    /// Get current screen content
    #[serde(rename = "get_screen")]
    GetScreen {
        session: String,
    },

    /// Wait for pattern match
    #[serde(rename = "expect")]
    Expect {
        session: String,
        pattern: String,
        timeout_ms: Option<u64>,
    },
}
```

### Key Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "esc")]
    Esc,
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "backspace")]
    Backspace,
    #[serde(rename = "ctrl_c")]
    CtrlC,
    #[serde(rename = "ctrl_d")]
    CtrlD,
    #[serde(rename = "ctrl_l")]
    CtrlL,
}

impl Key {
    pub fn to_bytes(&self) -> &'static [u8] {
        match self {
            Key::Up => b"\x1b[A",
            Key::Down => b"\x1b[B",
            Key::Left => b"\x1b[D",
            Key::Right => b"\x1b[C",
            Key::Enter => b"\r",
            Key::Esc => b"\x1b",
            Key::Tab => b"\t",
            Key::Backspace => b"\x7f",
            Key::CtrlC => b"\x03",
            Key::CtrlD => b"\x04",
            Key::CtrlL => b"\x0c",
        }
    }
}
```

### New Response Types

```rust
/// Screen content response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenContent {
    pub lines: Vec<String>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub mode: InteractionMode,
}

/// Interaction mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionMode {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "editor")]
    Editor,
    #[serde(rename = "repl")]
    Repl,
}
```

## Session Layer

### ScreenBuffer Module

```rust
// src/server/screen_buffer.rs

/// Screen buffer - parses and maintains PTY output
pub struct ScreenBuffer {
    cols: u16,
    rows: u16,
    lines: Vec<Vec<ScreenCell>>,
    cursor_row: u16,
    cursor_col: u16,
    mode: InteractionMode,
}

#[derive(Debug, Clone)]
struct ScreenCell {
    char: char,
}

impl ScreenBuffer {
    pub fn new(cols: u16, rows: u16) -> Self;

    /// Process raw output and update screen state
    pub fn process_output(&mut self, output: &[u8]) -> Result<()>;

    /// Get current screen content
    pub fn get_content(&self) -> ScreenContent;

    /// Detect current interaction mode
    pub fn detect_mode(&self) -> InteractionMode;

    /// Parse ANSI escape sequences
    fn parse_ansi(&mut self, data: &[u8]) -> Result<usize>;
}
```

### InteractionDetector Module

```rust
// src/server/interaction_detector.rs

/// Interaction mode detector
pub struct InteractionDetector;

impl InteractionDetector {
    /// Analyze output to detect if entering interactive mode
    pub fn detect(output: &str, previous_mode: InteractionMode) -> InteractionMode {
        // 1. Detect ANSI escape sequence patterns (e.g., reverse video menus)
        // 2. Detect specific program signatures (vim, nano, etc.)
        // 3. Detect prompt patterns
    }

    /// Check for menu patterns
    fn is_menu_pattern(output: &str) -> bool {
        // Check for ANSI reverse video, cursor positioning, etc.
    }

    /// Check for editor patterns
    fn is_editor_pattern(output: &str) -> bool {
        // Check for vim, nano, etc. signatures
    }
}
```

### Session Enhancements

```rust
impl Session {
    // New: Send control key
    pub fn send_key(&mut self, key: &Key) -> Result<()> {
        if let Some(pty) = &mut self.pty {
            pty.write_raw(key.to_bytes())
                .with_context(|| format!("Failed to send key: {:?}", key))?;
        }
        Ok(())
    }

    // New: Get screen content
    pub fn get_screen(&self) -> ScreenContent {
        self.screen_buffer.get_content()
    }

    // Enhanced: Update read_output to also update screen_buffer
    pub fn read_output(&mut self) -> Result<String> {
        let mut buf = [0u8; 8192];
        if let Some(pty) = &mut self.pty {
            match pty.read(&mut buf) {
                Ok(n) if n > 0 => {
                    // Update screen buffer with new output
                    self.screen_buffer.process_output(&buf[..n])?;

                    // ... existing logic ...
                }
                _ => Ok(String::new()),
            }
        } else {
            Ok(String::new())
        }
    }
}
```

### PTY Enhancements

```rust
impl Pty {
    /// New: Send raw bytes (consistent with existing write method)
    pub fn write_raw(&mut self, data: &[u8]) -> Result<usize> {
        let n = self.master.write(data)?;
        self.master.flush()?;
        Ok(n)
    }
}
```

## Client Layer

```rust
impl Client {
    /// Send control key to session
    pub fn send_key(&self, session: String, key: Key) -> Result<()> {
        let response = self.send_request(Request::SendKey { session, key })?;
        if response.success {
            Ok(())
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Get screen content
    pub fn get_screen(&self, session: String) -> Result<ScreenContent> {
        let response = self.send_request(Request::GetScreen { session })?;
        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Wait for pattern in output (enhanced version of existing wait)
    pub fn expect(&self, session: String, pattern: String, timeout_ms: Option<u64>) -> Result<bool> {
        let response = self.send_request(Request::Expect { session, pattern, timeout_ms })?;
        if response.success {
            let result: WaitResult = serde_json::from_value(response.data.unwrap_or_default())?;
            Ok(result.matched)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }
}
```

## CLI Interface

### New Commands

```bash
# Send control key
ccmux send-key <session> <key>

# Get screen content
ccmux screen <session> [--json]

# Wait for pattern (enhanced)
ccmux expect <session> <pattern> [--timeout SECONDS]
```

### CLI Definition

```rust
#[derive(Subcommand, Debug)]
pub enum Command {
    // ... existing commands ...

    /// Send control key to session
    SendKey {
        /// Session name
        session: String,
        /// Key to send (up, down, left, right, enter, esc, tab, ctrl_c, etc.)
        key: String,
    },

    /// Get session screen content
    Screen {
        /// Session name
        session: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
```

## Usage Examples

### Interactive Menu Navigation

```bash
# Send /help command
ccmux send backend "/help claude code"

# Wait for menu to appear
sleep 0.5

# Get screen content
ccmux screen backend --json

# Navigate down
ccmux send-key backend down

# Select option
ccmux send-key backend enter
```

### Programmatic Control (Rust)

```rust
let client = Client::new()?;

// Send command
client.send_input("backend".to_string(), "/help claude code".to_string())?;
std::thread::sleep(Duration::from_millis(500));

// Get screen content
let screen = client.get_screen("backend".to_string())?;
println!("Mode: {:?}", screen.mode);
for line in &screen.lines {
    println!("{}", line);
}

// Navigate based on content
client.send_key("backend".to_string(), Key::Down)?;
client.send_key("backend".to_string(), Key::Enter)?;

// Wait for completion
let completed = client.expect("backend".to_string(), "done|finished|>".to_string(), Some(120000))?;
```

### Programmatic Control (JSON/CLI)

```bash
# Get screen as JSON
SCREEN=$(ccmux screen backend --json)

# Parse and decide (using jq)
MODE=$(echo "$SCREEN" | jq -r '.mode')
if [ "$MODE" = "menu" ]; then
    ccmux send-key backend down
    ccmux send-key backend enter
fi
```

## Data Flow

```
Client Request (send_key)
    │
    ▼
Unix Socket (JSON)
    │
    ▼
Daemon Request Handler
    │
    ▼
Session.send_key()
    │
    ├─► key.to_bytes() → b"\x1b[B"
    └─► pty.write_raw(bytes)
    │
    ▼
PTY Master → Child Process
    │
    ▼
PTY Output (async)
    │
    ▼
ScreenBuffer.process_output()
    │
    ├─► Parse ANSI sequences
    ├─► Update lines, cursor
    └─► Detect mode
```

## Error Handling

| Scenario | Handling |
|----------|----------|
| Session not found | Return `Response::error("Session not found")` |
| PTY write failed | Log `error!`, return error response |
| ANSI parse failed | Skip invalid sequence, log `warn!` |
| Mode detection uncertain | Default to `InteractionMode::Normal` |
| expect timeout | Return `WaitResult { matched: false }` |

## Testing Strategy

### Unit Tests

- `tests/screen_buffer_test.rs`: Test ANSI parsing, screen updates
- `tests/interaction_detector_test.rs`: Test mode detection patterns

### Integration Tests

- `tests/interaction_test.rs`: Test send_key, get_screen, expect
- Use existing `TestDaemon` pattern from `integration_test.rs`

## Backward Compatibility

- All existing functionality remains unchanged
- New requests are optional
- Existing `send` method continues to work

## File Structure

```
src/
├── server/
│   ├── mod.rs                    # Export new modules
│   ├── pty.rs                    # Add write_raw method
│   ├── session.rs                # Add send_key, get_screen
│   ├── screen_buffer.rs          # New
│   ├── interaction_detector.rs   # New
│   └── daemon.rs                 # Add request handlers
├── protocol.rs                   # Add Key, ScreenContent, InteractionMode
├── client.rs                     # Add send_key, get_screen, expect
└── cli.rs                        # Add SendKey, Screen commands

tests/
├── screen_buffer_test.rs         # New
├── interaction_test.rs           # New
└── ... existing tests ...
```

## Implementation Phases

1. **Phase 1**: Protocol types and basic structure
2. **Phase 2**: ScreenBuffer with ANSI parsing
3. **Phase 3**: InteractionDetector
4. **Phase 4**: Session and PTY enhancements
5. **Phase 5**: Client and Daemon integration
6. **Phase 6**: CLI commands
7. **Phase 7**: Testing

## Future Enhancements

- Support for more complex ANSI sequences (colors, styles)
- Screen capture/recording
- Multi-screen support (multiple panes)
- Event subscription for screen changes
