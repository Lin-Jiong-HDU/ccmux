use ccmux::protocol::{Key, Request, Response, SessionStatus};

#[test]
fn test_request_serialize_new() {
    let req = Request::New {
        name: "test-session".to_string(),
        cwd: Some("/home/user/project".to_string()),
        strategy: Some("auto-safe".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"new\""));
    assert!(json.contains("test-session"));
}

#[test]
fn test_request_deserialize() {
    let json = r#"{"new":{"name":"test","cwd":null,"strategy":null}}"#;
    let req: Request = serde_json::from_str(json).unwrap();
    matches!(req, Request::New { .. });
}

#[test]
fn test_response_serialize_success() {
    let resp = Response::success("test-id");
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"success\":true"));
}

#[test]
fn test_session_status_display() {
    assert_eq!(SessionStatus::Running.to_string(), "running");
    assert_eq!(SessionStatus::Paused.to_string(), "paused");
    assert_eq!(SessionStatus::Stopped.to_string(), "stopped");
}

#[test]
fn test_key_serialize() {
    let key = Key::Down;
    let json = serde_json::to_string(&key).unwrap();
    assert_eq!(json, r#""down""#);
}

#[test]
fn test_key_deserialize() {
    let json = r#""up""#;
    let key: Key = serde_json::from_str(json).unwrap();
    assert!(matches!(key, Key::Up));
}

#[test]
fn test_key_to_bytes() {
    assert_eq!(Key::Enter.to_bytes(), b"\r");
    assert_eq!(Key::Esc.to_bytes(), b"\x1b");
    assert_eq!(Key::Up.to_bytes(), b"\x1b[A");
    assert_eq!(Key::Down.to_bytes(), b"\x1b[B");
    assert_eq!(Key::CtrlC.to_bytes(), b"\x03");
}
