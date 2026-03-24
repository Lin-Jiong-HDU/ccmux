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

# List sessions
ccmux ls

# View session logs
ccmux logs backend -f

# Get session status
ccmux status backend

# Send input to session
ccmux send backend "continue"

# Kill a session
ccmux kill backend
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
