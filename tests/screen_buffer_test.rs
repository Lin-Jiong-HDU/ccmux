use ccmux::protocol::InteractionMode;
use ccmux::server::ScreenBuffer;

#[test]
fn test_screen_buffer_new() {
    let buffer = ScreenBuffer::new(80, 24);
    let content = buffer.get_content();
    assert_eq!(content.lines.len(), 24);
    assert_eq!(content.cursor_row, 0);
    assert_eq!(content.cursor_col, 0);
    assert_eq!(content.mode, InteractionMode::Normal);
}

#[test]
fn test_screen_buffer_basic_text() {
    let mut buffer = ScreenBuffer::new(80, 24);
    buffer.process_output(b"Hello").unwrap();
    let content = buffer.get_content();
    assert_eq!(content.lines[0], "Hello");
}

#[test]
fn test_screen_buffer_newline() {
    let mut buffer = ScreenBuffer::new(80, 24);
    buffer.process_output(b"Hello\nWorld").unwrap();
    let content = buffer.get_content();
    assert_eq!(content.lines[0], "Hello");
    assert_eq!(content.lines[1], "World");
}

#[test]
fn test_screen_buffer_carriage_return() {
    let mut buffer = ScreenBuffer::new(80, 24);
    buffer.process_output(b"Hello\rWorld").unwrap();
    let content = buffer.get_content();
    assert_eq!(content.lines[0], "World");
}

#[test]
fn test_screen_buffer_ansi_cursor_up() {
    let mut buffer = ScreenBuffer::new(80, 24);
    buffer.process_output(b"Line 1\nLine 2").unwrap();
    buffer.process_output(b"\x1b[A").unwrap(); // Cursor up
    buffer.process_output(b"Modified").unwrap();
    let content = buffer.get_content();
    // After cursor up, "Modified" should overwrite part of "Line 1"
    assert!(content.lines[0].contains("Modified"));
}
