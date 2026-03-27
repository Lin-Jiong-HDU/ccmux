use ccmux::server::status_file::{StatusFile, BypassStatus};
use tempfile::TempDir;

#[test]
fn test_status_file_create_and_save() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let status = StatusFile::new(
        "test-session".to_string(),
        "claude --dangerously-skip-permissions \"test\"".to_string()
    );

    status.save(base).unwrap();

    assert!(StatusFile::exists(base, "test-session"));

    let loaded = StatusFile::load(base, "test-session").unwrap();
    assert_eq!(loaded.name, "test-session");
    assert_eq!(loaded.status, BypassStatus::Idle);
    assert!(loaded.pid.is_none());
}

#[test]
fn test_status_file_mark_running() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let mut status = StatusFile::new(
        "test-session".to_string(),
        "command".to_string()
    );

    status.mark_running(12345);
    status.save(base).unwrap();

    let loaded = StatusFile::load(base, "test-session").unwrap();
    assert_eq!(loaded.status, BypassStatus::Running);
    assert_eq!(loaded.pid, Some(12345));
}

#[test]
fn test_status_file_mark_completed() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let mut status = StatusFile::new(
        "test-session".to_string(),
        "command".to_string()
    );

    status.mark_running(12345);
    status.mark_completed(0);
    status.save(base).unwrap();

    let loaded = StatusFile::load(base, "test-session").unwrap();
    assert_eq!(loaded.status, BypassStatus::Completed);
    assert_eq!(loaded.exit_code, Some(0));
    assert!(loaded.end_time.is_some());
}

#[test]
fn test_status_file_mark_failed() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let mut status = StatusFile::new(
        "test-session".to_string(),
        "command".to_string()
    );

    status.mark_running(12345);
    status.mark_completed(1);
    status.save(base).unwrap();

    let loaded = StatusFile::load(base, "test-session").unwrap();
    assert_eq!(loaded.status, BypassStatus::Failed);
    assert_eq!(loaded.exit_code, Some(1));
}

#[test]
fn test_status_file_paths() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let session_dir = StatusFile::session_dir(base, "test");
    assert_eq!(session_dir, base.join("sessions").join("test"));

    let status_path = StatusFile::status_path(base, "test");
    assert_eq!(status_path, base.join("sessions").join("test").join("status.json"));

    let output_path = StatusFile::output_path(base, "test");
    assert_eq!(output_path, base.join("sessions").join("test").join("output.log"));
}
