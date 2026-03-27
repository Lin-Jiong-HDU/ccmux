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

ccmux supports four strategies:

- **auto-safe**: Read operations are automatic, write/execute require approval (default)
- **auto-all**: All operations are automatic
- **manual**: All operations require approval
- **bypass**: No PTY, file-based session management with `--dangerously-skip-permissions`

Configure in `~/.config/ccmux/config.toml`:

```toml
[default]
strategy = "auto-safe"

[strategy.auto-safe]
file_read = "auto"
file_write = "pause"
command_exec = "pause"
tool_use = "auto"

[strategy.bypass]
file_read = "auto"
file_write = "auto"
command_exec = "auto"
tool_use = "auto"
bypass_permissions = true
```

### Bypass Strategy

The `bypass` strategy launches Claude Code with `--dangerously-skip-permissions` and manages sessions via file-based state synchronization instead of PTY.

**Features:**
- Fire-and-forget task execution
- No PTY overhead
- State managed via JSON files in `~/.ccmux/sessions/<name>/`
- Claude Code skill writes output directly to logs

**Usage:**

```bash
# Create a bypass session
ccmux new -n worker --strategy bypass

# Send a task (runs in background)
ccmux send worker "实现用户认证功能"

# Check status (reads from status.json)
ccmux status worker

# View output (reads from output.log)
ccmux output worker --lines 100

# Wait for pattern match (scans output.log)
ccmux wait worker "完成|错误"
```

**Directory Structure:**

```
~/.ccmux/sessions/<name>/
├── status.json    # Session state (ccmux + skill)
└── output.log     # Task output (skill writes)
```

**Skill Integration:**

The Claude Code instance running the task should update status.json when complete:

```bash
# When task completes, the skill should do:
echo '{"status":"completed","exit_code":0}' > ~/.ccmux/sessions/worker/status.json
```

**Status File Schema:**

```json
{
  "name": "worker",
  "status": "running",  // idle | running | completed | failed
  "exit_code": null,
  "pid": 12345,
  "command": "claude --dangerously-skip-permissions \"task\"",
  "start_time": "2025-03-27T10:00:00Z",
  "end_time": null
}
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
