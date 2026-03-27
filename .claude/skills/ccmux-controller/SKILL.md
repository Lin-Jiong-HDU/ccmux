---
name: ccmux-controller
description: Manage ccmux bypass sessions for controlling other Claude Code instances. Use when you need to delegate tasks to worker Claude sessions, run parallel Claude instances, or coordinate multi-agent workflows.
disable-model-invocation: true
---

# Ccmux Controller Skill

You are operating as a **controller** Claude instance that can manage **worker** Claude instances through ccmux.

## Overview

Ccmux bypass sessions allow you to:
- Create fire-and-forget worker sessions
- Delegate tasks to run in background
- Monitor worker progress
- Retrieve results when complete

## Session Management

### Create a Worker Session

```bash
# Create a new bypass session for a worker
ccmux new -n <session-name> --strategy bypass
```

The worker will use `--dangerously-skip-permissions` for autonomous operation.

### Send Tasks to Workers

```bash
# Send a task to the worker
ccmux send <session-name> "<your task description>"
```

The task runs in the background. The worker Claude processes it independently.

### Monitor Worker Status

```bash
# Check worker status
ccmux status <session-name>

# Get worker output
ccmux output <session-name> --lines 100
```

### Wait for Completion

```bash
# Wait for specific pattern in output
ccmux wait <session-name> "完成|错误|error|done|finished"

# Or wait for session completion
ccmux wait <session-name> "completed"
```

### Terminate Workers

```bash
# Kill a worker session
ccmux kill <session-name>
```

## Session State Files

Bypass sessions store state in `~/.ccmux/sessions/<name>/`:

```
~/.ccmux/sessions/<name>/
├── status.json    # Current state (running/completed/failed)
└── output.log     # Worker's output
```

## Multi-Worker Coordination

You can manage multiple workers for parallel processing:

```bash
# Create multiple workers
ccmux new -n worker1 --strategy bypass
ccmux new -n worker2 --strategy bypass
ccmux new -n worker3 --strategy bypass

# Send different tasks to each
ccmux send worker1 "Implement feature A"
ccmux send worker2 "Implement feature B"
ccmux send worker3 "Write tests"

# Monitor all workers
ccmux ls
```

## Best Practices

1. **Descriptive session names**: Use meaningful names like `feat-auth`, `fix-bug-123`, `test-api`
2. **Task specificity**: Be clear and specific in task descriptions
3. **Completion patterns**: Use unique completion markers in your tasks
4. **Output monitoring**: Check `output.log` for detailed worker activity
5. **Cleanup**: Kill completed workers to free resources

## Example Workflow

```bash
# 1. Start ccmux daemon if not running
ccmuxd &

# 2. Create a worker for authentication feature
ccmux new -n feat-auth --strategy bypass

# 3. Send the implementation task
ccmux send feat-auth "Implement JWT authentication with refresh tokens. Follow the patterns in src/auth/. Write tests in tests/auth_test.rs. Update README.md when complete."

# 4. Monitor progress
ccmux status feat-auth

# 5. Wait for completion (worker should output "完成" when done)
ccmux wait feat-auth "完成|错误"

# 6. Get the full output
ccmux output feat-auth --lines 200

# 7. Clean up
ccmux kill feat-auth
```

## Worker Expectations

Workers are expected to:
1. Process the assigned task autonomously
2. Write progress to their output log
3. Update `status.json` when complete:
   - Set `status` to `"completed"` or `"failed"`
   - Set `exit_code` (0 for success, non-zero for failure)
   - Set `end_time` to current timestamp

## Troubleshooting

### Worker not responding
```bash
# Check if still running
ccmux status <session-name>

# View recent output
ccmux output <session-name> --lines 50

# Check the status file directly
cat ~/.ccmux/sessions/<session-name>/status.json
```

### Force stop a stuck worker
```bash
ccmux kill <session-name>
```
