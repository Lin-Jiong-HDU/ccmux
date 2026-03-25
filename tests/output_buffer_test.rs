// Tests for JSON serialization of protocol types used by subscribe/wait commands.
// These tests ensure StreamEvent and WaitResult are serialized as expected.

#[test]
fn test_subscribe_request_serialization() {
    use ccmux::protocol::Request;

    let req = Request::Subscribe {
        session: "backend".to_string(),
        since: Some(1732560000000),
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("subscribe"));
    assert!(json.contains("backend"));
    assert!(json.contains("1732560000000"));
}

#[test]
fn test_wait_request_serialization() {
    use ccmux::protocol::Request;

    let req = Request::Wait {
        session: "worker".to_string(),
        pattern: "error|done".to_string(),
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("wait"));
    assert!(json.contains("worker"));
    assert!(json.contains("error|done"));
}

#[test]
fn test_stream_event_serialization() {
    use ccmux::protocol::StreamEvent;

    let event = StreamEvent {
        event_type: "output".to_string(),
        ts: Some(1234567890),
        text: Some("Hello".to_string()),
        status: None,
        reason: None,
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("output"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_stream_event_with_status() {
    use ccmux::protocol::{StreamEvent, SessionStatus};

    let event = StreamEvent {
        event_type: "status".to_string(),
        ts: Some(1234567890),
        text: None,
        status: Some(SessionStatus::Paused),
        reason: Some("file_write".to_string()),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("paused"));
    assert!(json.contains("file_write"));
}

#[test]
fn test_wait_result_serialization() {
    use ccmux::protocol::WaitResult;

    let result = WaitResult {
        matched: true,
        pattern: Some("error|Error".to_string()),
        output: Some("error: something failed".to_string()),
        timestamp: Some(1234567890),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("matched"));
    assert!(json.contains("error"));
}

#[test]
fn test_wait_result_not_matched() {
    use ccmux::protocol::WaitResult;

    let result = WaitResult {
        matched: false,
        pattern: Some("error".to_string()),
        output: None,
        timestamp: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"matched\":false"));
}
