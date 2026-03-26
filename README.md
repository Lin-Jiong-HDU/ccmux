# ccmux

Claude Code session manager - like tmux, but for Claude Code.

## Features

- Manage multiple Claude Code sessions
- Background daemon with Unix socket communication
- Configurable auto/pause strategies
- Session state persistence
- CLI and programmatic interfaces

## Installation

```bash
cargo install --path .
```

## Usage

Start the daemon:
```bash
ccmuxd
```

In another terminal:
```bash
# Create a new session
ccmux new -n backend

# List sessions (use --json for machine-readable output)
ccmux ls
ccmux ls --json

# View session logs
ccmux logs backend -f          # Follow mode
ccmux logs backend --tail 100  # Custom line count (default: 50)

# Get session status
ccmux status backend
ccmux status backend --json    # JSON output

# Send input to session
ccmux send backend "continue"

# Kill a session
ccmux kill backend
```

### Programmatic Control (for Claude Code)

```bash
# Subscribe to session output
ccmux subscribe <session>
ccmux subscribe <session> --follow  # Continuous polling
ccmux subscribe <session> --since 1732560000000  # Only output after Unix epoch timestamp (milliseconds)

# Wait for specific output pattern
ccmux wait <session> "pattern"
ccmux wait <session> "完成|错误" --timeout 120

# Get screen content (for interactive menu navigation)
ccmux screen <session>
ccmux screen <session> --json  # Machine-readable JSON output
```

### Interactive Menu Control

```bash
# Send a command that shows a menu
ccmux send backend "/help claude code"

# Wait for menu to appear
sleep 0.5

# Get screen content (includes menu options)
ccmux screen backend

# Navigate menu
ccmux send-key backend down
ccmux send-key backend down

# Select option
ccmux send-key backend enter
```

### Programmatic Control (Enhanced)

```bash
# Get screen as JSON for parsing
SCREEN=$(ccmux screen backend --json)

# Extract mode using jq
MODE=$(echo "$SCREEN" | jq -r '.mode')

# Act based on mode
if [ "$MODE" = "menu" ]; then
    ccmux send-key backend down
    ccmux send-key backend enter
fi
```

### Example: Claude Code controlling Claude Code

```bash
# Create a worker session
ccmux new -n worker

# Send a task to the worker
ccmux send worker "请帮我实现用户认证功能"

# Wait for completion or error (with 5 minute timeout)
ccmux wait worker "完成|错误|error|done" --timeout 300

# Or continuously monitor output
ccmux subscribe worker --follow
```

## Strategies

ccmux supports three strategies:

- **auto-safe**: Read operations are automatic, write/execute require approval (default)
- **auto-all**: All operations are automatic
- **manual**: All operations require approval

Configure in `~/.config/ccmux/config.toml`:

```toml
[default]
strategy = "auto-safe"

[strategy.auto-safe]
file_read = "auto"
file_write = "pause"
command_exec = "pause"
tool_use = "auto"
```

## Architecture

- `ccmuxd`: Daemon managing all sessions
- `ccmux`: CLI client for controlling the daemon

Communication is via Unix socket using JSON protocol.

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin ccmuxd
```

## License

MIT
