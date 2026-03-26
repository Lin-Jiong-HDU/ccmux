use ccmux::protocol::InteractionMode;
use ccmux::server::InteractionDetector;

#[test]
fn test_detect_normal_mode() {
    let detector = InteractionDetector::new();
    let output = "hello world\nfoo bar";
    let mode = detector.detect(output, InteractionMode::Normal);
    assert_eq!(mode, InteractionMode::Normal);
}

#[test]
fn test_detect_menu_pattern() {
    let detector = InteractionDetector::new();
    // Reverse video ANSI sequence indicates menu
    let output = "\x1b[7m Option 1 \x1b[0m\r\n  Option 2";
    let mode = detector.detect(output, InteractionMode::Normal);
    assert_eq!(mode, InteractionMode::Menu);
}

#[test]
fn test_detect_editor_pattern_vim() {
    let detector = InteractionDetector::new();
    // Vim shows this on startup
    let output = "VIM - Vi IMproved";
    let mode = detector.detect(output, InteractionMode::Normal);
    assert_eq!(mode, InteractionMode::Editor);
}

#[test]
fn test_detect_editor_pattern_nano() {
    let detector = InteractionDetector::new();
    let output = "GNU nano 7.2";
    let mode = detector.detect(output, InteractionMode::Normal);
    assert_eq!(mode, InteractionMode::Editor);
}

#[test]
fn test_mode_persistence() {
    let detector = InteractionDetector::new();
    // Once in menu mode, stay there unless clear exit signal
    let output = "more options";
    let mode = detector.detect(output, InteractionMode::Menu);
    assert_eq!(mode, InteractionMode::Menu);
}
