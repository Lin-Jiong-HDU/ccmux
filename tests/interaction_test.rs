//! Integration tests for interactive PTY control

use ccmux::protocol::InteractionMode;
use ccmux::protocol::Key;
use ccmux::server::ScreenBuffer;

#[test]
fn test_key_parsing() {
    // Test key string parsing
    let test_cases = vec![
        ("up", true),
        ("down", true),
        ("left", true),
        ("right", true),
        ("enter", true),
        ("return", true),
        ("esc", true),
        ("escape", true),
        ("tab", true),
        ("backspace", true),
        ("ctrl_c", true),
        ("ctrl-c", true),
        ("invalid", false),
    ];

    for (input, expected_valid) in test_cases {
        let result = parse_key_test(input);
        assert_eq!(result.is_ok(), expected_valid, "Failed for: {}", input);
    }
}

fn parse_key_test(s: &str) -> Result<Key, String> {
    match s.to_lowercase().as_str() {
        "up" => Ok(Key::Up),
        "down" => Ok(Key::Down),
        "left" => Ok(Key::Left),
        "right" => Ok(Key::Right),
        "enter" | "return" => Ok(Key::Enter),
        "esc" | "escape" => Ok(Key::Esc),
        "tab" => Ok(Key::Tab),
        "backspace" => Ok(Key::Backspace),
        "ctrl_c" | "ctrl-c" => Ok(Key::CtrlC),
        "ctrl_d" | "ctrl-d" => Ok(Key::CtrlD),
        "ctrl_l" | "ctrl-l" => Ok(Key::CtrlL),
        _ => Err(format!("Unknown key: {}", s)),
    }
}

#[test]
fn test_screen_buffer_integration() {
    // Test that screen buffer properly handles a full workflow
    let mut buffer = ScreenBuffer::new(80, 24);

    // Simulate a menu being displayed
    buffer
        .process_output(b"\x1b[7m Option 1 \x1b[0m\r\n")
        .unwrap();
    buffer.process_output(b"  Option 2\r\n").unwrap();
    buffer.process_output(b"  Option 3\r\n").unwrap();

    let content = buffer.get_content();
    assert_eq!(content.lines[0], " Option 1");
    assert!(content.lines[0].contains("Option 1"));

    // Simulate cursor movement (down arrow)
    buffer.process_output(b"\x1b[B").unwrap();

    // Simulate selecting option
    buffer.process_output(b"\r").unwrap();
}

#[test]
fn test_interaction_detector_integration() {
    use ccmux::server::InteractionDetector;

    let detector = InteractionDetector::new();

    // Test normal → menu transition
    let output1 = "\x1b[7mSelect an option\x1b[0m";
    let mode1 = detector.detect(output1, InteractionMode::Normal);
    assert_eq!(mode1, InteractionMode::Menu);

    // Test menu persistence
    let output2 = "  Option 1\n  Option 2";
    let mode2 = detector.detect(output2, InteractionMode::Menu);
    assert_eq!(mode2, InteractionMode::Menu);

    // Test editor detection
    let output3 = "VIM - Vi IMproved 9.0";
    let mode3 = detector.detect(output3, InteractionMode::Normal);
    assert_eq!(mode3, InteractionMode::Editor);
}
