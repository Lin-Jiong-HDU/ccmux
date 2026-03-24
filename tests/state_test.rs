use ccmux::state::{SessionState, SessionStatus, State};
use tempfile::TempDir;

#[test]
fn test_create_new_state() {
    let state = State::new();
    assert!(state.sessions.is_empty());
}

#[test]
fn test_add_session() {
    let mut state = State::new();
    let session = SessionState {
        id: "test".to_string(),
        status: SessionStatus::Running,
        pid: Some(12345),
        cwd: "/home/test".to_string(),
        strategy: "auto-safe".to_string(),
        created_at: "2026-03-24T10:00:00Z".to_string(),
        log_file: "/tmp/test.log".to_string(),
    };
    state.add_session(session);
    assert_eq!(state.sessions.len(), 1);
}

#[test]
fn test_save_and_load_state() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("state.json");

    let mut state = State::new();
    state.add_session(SessionState {
        id: "test".to_string(),
        status: SessionStatus::Running,
        pid: Some(12345),
        cwd: "/home/test".to_string(),
        strategy: "auto-safe".to_string(),
        created_at: "2026-03-24T10:00:00Z".to_string(),
        log_file: "/tmp/test.log".to_string(),
    });

    state.save_to(&path).unwrap();

    let loaded = State::load_from(&path).unwrap();
    assert_eq!(loaded.sessions.len(), 1);
    assert_eq!(loaded.sessions["test"].pid, Some(12345));
}
