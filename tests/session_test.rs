use ccmux::protocol::SessionStatus;
use ccmux::server::SessionHandle;

// Note: Full PTY tests require actual process spawning
// These are unit tests for the data structures

#[test]
fn test_session_info() {
    // Test that session info structure is correct
    let info = ccmux::protocol::SessionInfo {
        id: "test".to_string(),
        status: SessionStatus::Running,
        pid: Some(12345),
        cwd: "/home/test".to_string(),
        strategy: "auto-safe".to_string(),
        created_at: "2026-03-24T10:00:00Z".to_string(),
        uptime_secs: Some(3600),
        last_output: Some("Last output line".to_string()),
    };

    assert_eq!(info.id, "test");
    assert_eq!(info.status, SessionStatus::Running);
}

#[test]
fn test_session_handle() {
    let handle = SessionHandle {
        id: "session-123".to_string(),
    };
    assert_eq!(handle.id, "session-123");
}
