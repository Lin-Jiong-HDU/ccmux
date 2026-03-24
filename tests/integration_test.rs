//! Integration tests

use ccmux::client::Client;
use ccmux::protocol::{Request, Response};
use std::process::{Command, Child};
use std::time::Duration;
use std::thread;

struct DaemonProcess {
    child: Child,
}

impl DaemonProcess {
    fn start() -> anyhow::Result<Self> {
        let child = Command::new("cargo")
            .args(["run", "--bin", "ccmuxd"])
            .spawn()?;

        // Wait for daemon to start
        thread::sleep(Duration::from_secs(2));

        Ok(Self { child })
    }
}

impl Drop for DaemonProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
#[ignore] // Run with --ignored flag when daemon is available
fn test_daemon_lifecycle() {
    let _daemon = DaemonProcess::start().expect("Failed to start daemon");

    let client = Client::new().expect("Failed to create client");
    assert!(client.is_daemon_running(), "Daemon should be running");
}

#[test]
#[ignore]
fn test_session_lifecycle() {
    let _daemon = DaemonProcess::start().expect("Failed to start daemon");
    let client = Client::new().expect("Failed to create client");

    // Create session
    let session = client.new_session(
        "test-session".to_string(),
        Some("/tmp".to_string()),
        Some("auto-safe".to_string()),
    ).expect("Failed to create session");

    assert_eq!(session.id, "test-session");

    // List sessions
    let sessions = client.list_sessions().expect("Failed to list sessions");
    assert!(sessions.iter().any(|s| s.id == "test-session"));

    // Kill session
    client.kill_session("test-session".to_string()).expect("Failed to kill session");

    // Verify session is gone
    let sessions = client.list_sessions().expect("Failed to list sessions");
    assert!(!sessions.iter().any(|s| s.id == "test-session"));
}

#[test]
fn test_client_socket_path() {
    let client = Client::new().unwrap();
    // Socket path should contain ccmux.sock
    let path = client.socket_path().to_string_lossy();
    assert!(path.contains("ccmux.sock"), "Socket path should contain ccmux.sock");
}

#[test]
fn test_request_serialization() {
    let request = Request::New {
        name: "test".to_string(),
        cwd: Some("/tmp".to_string()),
        strategy: Some("auto-safe".to_string()),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("new"));
    assert!(json.contains("test"));
}

#[test]
fn test_response_parsing() {
    let json = r#"{"success":true,"data":{"id":"test"},"error":null}"#;
    let response: Response = serde_json::from_str(json).unwrap();
    assert!(response.success);
}
