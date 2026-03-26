use ccmux::protocol::{InteractionMode, Key, Request, Response, ScreenContent, SessionStatus};

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

#[test]
fn test_interaction_mode_serialize() {
    let mode = InteractionMode::Menu;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, r#""menu""#);
}

#[test]
fn test_interaction_mode_deserialize() {
    let json = r#""menu""#;
    let mode: InteractionMode = serde_json::from_str(json).unwrap();
    assert_eq!(mode, InteractionMode::Menu);
}

#[test]
fn test_screen_content_serialize() {
    let content = ScreenContent {
        lines: vec!["hello".to_string(), "world".to_string()],
        cursor_row: 5,
        cursor_col: 10,
        mode: InteractionMode::Normal,
    };
    let json = serde_json::to_string(&content).unwrap();
    let parsed: ScreenContent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.lines.len(), 2);
    assert_eq!(parsed.cursor_row, 5);
    assert_eq!(parsed.cursor_col, 10);
}
