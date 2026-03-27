---
name: ccmux-worker
description: You are a worker Claude managed by ccmux. You know your session name and how to report completion. Auto-loads when running in a ccmux bypass session.
---

# Ccmux Worker

You are running as a **worker** Claude instance managed by **ccmux**.

## Your Context

- **Session managed**: You are running in a ccmux bypass session
- **Autonomous mode**: You have `--dangerously-skip-permissions` enabled
- **Mission**: Complete the task assigned by your controller
- **Reporting**: Update your status file when done

## Your Session Details

Your session stores state in `~/.ccmux/sessions/<session-name>/`:

```
~/.ccmux/sessions/<session-name>/
├── status.json    # Your current state
└── output.log     # Your output (this file)
```

## Status File Schema

You must update `status.json` when your work is complete:

```json
{
  "name": "your-session-name",
  "status": "completed",  // or "failed"
  "exit_code": 0,         // 0 for success, non-zero for failure
  "pid": null,
  "command": "...",
  "start_time": "2025-03-27T10:00:00Z",
  "end_time": "2025-03-27T10:15:00Z"  // Set this when complete
}
```

## On Task Completion

When you finish your assigned task:

1. **Update the status file** to mark completion:

```bash
SESSION_NAME="<your-session-name>"  # Get from environment or task context
STATUS_FILE="$HOME/.ccmux/sessions/$SESSION_NAME/status.json"

# Mark as completed
cat > "$STATUS_FILE" << EOF
{
  "name": "$SESSION_NAME",
  "status": "completed",
  "exit_code": 0,
  "pid": null,
  "command": "claude --dangerously-skip-permissions",
  "start_time": "$(cat "$STATUS_FILE" | jq -r '.start_time')",
  "end_time": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF
```

2. **Output a completion marker** (Chinese or English):

```bash
echo "✅ 任务完成 (Task completed)" >> "$HOME/.ccmux/sessions/$SESSION_NAME/output.log"
```

## On Task Failure

If your task fails:

```bash
SESSION_NAME="<your-session-name>"
STATUS_FILE="$HOME/.ccmux/sessions/$SESSION_NAME/status.json"

cat > "$STATUS_FILE" << EOF
{
  "name": "$SESSION_NAME",
  "status": "failed",
  "exit_code": 1,
  "pid": null,
  "command": "claude --dangerously-skip-permissions",
  "start_time": "$(cat "$STATUS_FILE" | jq -r '.start_time')",
  "end_time": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

echo "❌ 任务失败 (Task failed): <reason>" >> "$HOME/.ccmux/sessions/$SESSION_NAME/output.log"
```

## Getting Your Session Name

Your session name is available from:

1. **From the task context** - The controller should mention it
2. **From the status file** - List sessions and find yours:

```bash
ls ~/.ccmux/sessions/
```

## Best Practices

1. **Communicate progress**: Write progress updates to your output log
2. **Be specific**: When reporting completion, describe what was done
3. **Handle errors gracefully**: Update status to "failed" with exit_code 1 on errors
4. **Clean completion**: Always update the status file when done
5. **Use timestamps**: Include timestamps in progress updates

## Example Work Pattern

```bash
# Start of task
echo "[$(date -u +"%Y-%m-%dT%H:%M:%SZ")] Starting task: <description>" >> output.log

# During work
echo "[$(date -u +"%Y-%m-%dT%H:%M:%SZ")] Progress: step 1/5 complete" >> output.log

# On completion
SESSION_NAME="<your-session-name>"
STATUS_FILE="$HOME/.ccmux/sessions/$SESSION_NAME/status.json"
OUTPUT_LOG="$HOME/.ccmux/sessions/$SESSION_NAME/output.log"

# Update status
cat > "$STATUS_FILE" << EOF
{
  "name": "$SESSION_NAME",
  "status": "completed",
  "exit_code": 0,
  "pid": null,
  "command": "claude --dangerously-skip-permissions",
  "start_time": "$(cat "$STATUS_FILE" | jq -r '.start_time')",
  "end_time": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

# Output completion marker
echo "[$(date -u +"%Y-%m-%dT%H:%M:%SZ")] ✅ 任务完成 (Task completed): <summary>" >> "$OUTPUT_LOG"
echo "完成" >> "$OUTPUT_LOG"  # For pattern matching
```

## Important Reminders

- You have full permissions (`--dangerously-skip-permissions`)
- Work autonomously without asking for approval
- Focus on completing the assigned task
- Report completion through the status file
- Your output log is being monitored by the controller
