// Note: OutputBuffer is not public, so we test via Session
// These tests verify the protocol and client methods

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
