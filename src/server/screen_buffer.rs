//! Screen buffer for parsing PTY output and maintaining screen state
//!
//! This module implements a screen buffer that parses ANSI escape sequences
//! from PTY output and maintains the current screen state, including cursor
//! position and text content.

use crate::protocol::{InteractionMode, ScreenContent};
use anyhow::Result;

/// A single cell on the screen
#[derive(Debug, Clone, Copy)]
struct ScreenCell {
    c: char,
}

impl ScreenCell {
    fn new() -> Self {
        Self { c: ' ' }
    }
}

impl Default for ScreenCell {
    fn default() -> Self {
        Self::new()
    }
}

/// ANSI escape sequence parser state
#[derive(Debug, Clone, Copy, PartialEq)]
enum AnsiState {
    /// Normal text processing
    Ground,
    /// Escaping (received ESC)
    Escape,
    /// Control Sequence Introducer (received ESC [)
    Csi,
    /// Parameter string in CSI sequence
    CsiParam,
}

/// Screen buffer for parsing PTY output and maintaining screen state
pub struct ScreenBuffer {
    /// Screen cells organized as [row][col]
    cells: Vec<Vec<ScreenCell>>,
    /// Number of columns
    cols: usize,
    /// Number of rows
    rows: usize,
    /// Current cursor row (0-indexed)
    cursor_row: usize,
    /// Current cursor column (0-indexed)
    cursor_col: usize,
    /// Current interaction mode
    mode: InteractionMode,
    /// ANSI parser state
    ansi_state: AnsiState,
    /// CSI parameter string being built
    csi_param: String,
}

impl ScreenBuffer {
    /// Create a new screen buffer with the given dimensions
    pub fn new(cols: u16, rows: u16) -> Self {
        // Ensure minimum dimensions to prevent panics
        let cols = cols.max(1);
        let rows = rows.max(1);

        let cols = cols as usize;
        let rows = rows as usize;

        let cells = vec![vec![ScreenCell::new(); cols]; rows];

        Self {
            cells,
            cols,
            rows,
            cursor_row: 0,
            cursor_col: 0,
            mode: InteractionMode::Normal,
            ansi_state: AnsiState::Ground,
            csi_param: String::new(),
        }
    }

    /// Process output from PTY and update screen state
    pub fn process_output(&mut self, data: &[u8]) -> Result<()> {
        // Convert bytes to string, replacing invalid UTF-8
        let text = String::from_utf8_lossy(data);

        for c in text.chars() {
            self.process_char(c);
        }

        Ok(())
    }

    /// Process a single character
    fn process_char(&mut self, c: char) {
        match self.ansi_state {
            AnsiState::Ground => {
                if c == '\x1b' {
                    self.ansi_state = AnsiState::Escape;
                } else if c == '\r' {
                    // Carriage return - move cursor to start of line
                    self.cursor_col = 0;
                } else if c == '\n' {
                    // Newline - move cursor to next line and reset column
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                    if self.cursor_row >= self.rows {
                        // Scroll up: remove first row, add new empty row
                        self.cells.remove(0);
                        self.cells.push(vec![ScreenCell::new(); self.cols]);
                        self.cursor_row = self.rows - 1;
                    }
                } else if c >= ' ' && c != '\x7f' {
                    // Printable character
                    self.put_char(c);
                }
                // Ignore other control characters
            }
            AnsiState::Escape => {
                if c == '[' {
                    self.ansi_state = AnsiState::Csi;
                    self.csi_param.clear();
                } else {
                    // Not a CSI sequence, return to ground
                    self.ansi_state = AnsiState::Ground;
                }
            }
            AnsiState::Csi => {
                if c.is_ascii_digit() || c == ';' || c == '?' || c == '>' {
                    self.csi_param.push(c);
                    self.ansi_state = AnsiState::CsiParam;
                } else {
                    // CSI sequence without parameters
                    let param = self.csi_param.clone();
                    self.handle_csi(c, &param);
                    self.ansi_state = AnsiState::Ground;
                }
            }
            AnsiState::CsiParam => {
                if c.is_ascii_digit() || c == ';' || c == '?' || c == '>' {
                    self.csi_param.push(c);
                } else {
                    let param = self.csi_param.clone();
                    self.handle_csi(c, &param);
                    self.ansi_state = AnsiState::Ground;
                }
            }
        }
    }

    /// Put a character at the current cursor position
    fn put_char(&mut self, c: char) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col] = ScreenCell { c };
            self.cursor_col += 1;

            // Auto-wrap to next line if needed
            if self.cursor_col >= self.cols {
                self.cursor_col = 0;
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    // Scroll up
                    self.cells.remove(0);
                    self.cells.push(vec![ScreenCell::new(); self.cols]);
                    self.cursor_row = self.rows - 1;
                }
            }
        }
    }

    /// Handle a CSI (Control Sequence Introducer) sequence
    fn handle_csi(&mut self, terminator: char, params: &str) {
        match terminator {
            // Cursor positioning sequences
            'A' => {
                // Cursor up
                let n = self.parse_csi_param(params, 0, 1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'B' => {
                // Cursor down
                let n = self.parse_csi_param(params, 0, 1);
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
            }
            'C' => {
                // Cursor forward (right)
                let n = self.parse_csi_param(params, 0, 1);
                self.cursor_col = (self.cursor_col + n).min(self.cols - 1);
            }
            'D' => {
                // Cursor back (left)
                let n = self.parse_csi_param(params, 0, 1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'H' | 'f' => {
                // Cursor position
                let (row, col) = self.parse_csi_2_params(params, 1, 1);
                self.cursor_row = (row.saturating_sub(1)).min(self.rows - 1);
                self.cursor_col = (col.saturating_sub(1)).min(self.cols - 1);
            }
            // Erase sequences
            'J' => {
                // Erase display
                let n = self.parse_csi_param(params, 0, 0);
                self.erase_display(n);
            }
            'K' => {
                // Erase line
                let n = self.parse_csi_param(params, 0, 0);
                self.erase_line(n);
            }
            // Other sequences - ignore for now
            _ => {
                // Unknown CSI sequence, ignore
            }
        }
    }

    /// Parse a CSI parameter with default value
    fn parse_csi_param(&self, params: &str, idx: usize, default: usize) -> usize {
        params
            .split(';')
            .nth(idx)
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Parse two CSI parameters with default values
    fn parse_csi_2_params(&self, params: &str, default1: usize, default2: usize) -> (usize, usize) {
        let parts: Vec<&str> = params.split(';').collect();
        let p1 = parts
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default1);
        let p2 = parts
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(default2);
        (p1, p2)
    }

    /// Erase display (0: from cursor to end, 1: from start to cursor, 2: entire display)
    fn erase_display(&mut self, n: usize) {
        match n {
            0 => {
                // Erase from cursor to end of screen
                for row in self.cursor_row..self.rows {
                    let start_col = if row == self.cursor_row {
                        self.cursor_col
                    } else {
                        0
                    };
                    for col in start_col..self.cols {
                        self.cells[row][col] = ScreenCell::new();
                    }
                }
            }
            1 => {
                // Erase from start of screen to cursor
                for row in 0..=self.cursor_row {
                    let end_col = if row == self.cursor_row {
                        self.cursor_col + 1
                    } else {
                        self.cols
                    };
                    for col in 0..end_col {
                        self.cells[row][col] = ScreenCell::new();
                    }
                }
            }
            2 => {
                // Erase entire screen
                for row in 0..self.rows {
                    for col in 0..self.cols {
                        self.cells[row][col] = ScreenCell::new();
                    }
                }
            }
            _ => {}
        }
    }

    /// Erase line (0: from cursor to end, 1: from start to cursor, 2: entire line)
    fn erase_line(&mut self, n: usize) {
        if self.cursor_row >= self.rows {
            return;
        }
        match n {
            0 => {
                // Erase from cursor to end of line
                for col in self.cursor_col..self.cols {
                    self.cells[self.cursor_row][col] = ScreenCell::new();
                }
            }
            1 => {
                // Erase from start of line to cursor
                for col in 0..=self.cursor_col {
                    self.cells[self.cursor_row][col] = ScreenCell::new();
                }
            }
            2 => {
                // Erase entire line
                for col in 0..self.cols {
                    self.cells[self.cursor_row][col] = ScreenCell::new();
                }
            }
            _ => {}
        }
    }

    /// Get the current screen content
    pub fn get_content(&self) -> ScreenContent {
        let lines: Vec<String> = self
            .cells
            .iter()
            .map(|row| {
                let line: String = row.iter().map(|cell| cell.c).collect();
                // Trim trailing spaces for cleaner output
                line.trim_end().to_string()
            })
            .collect();

        ScreenContent {
            lines,
            cursor_row: self.cursor_row as u16,
            cursor_col: self.cursor_col as u16,
            mode: self.mode,
        }
    }

    /// Get the current interaction mode
    pub fn detect_mode(&self) -> InteractionMode {
        self.mode
    }

    /// Set the interaction mode
    pub fn set_mode(&mut self, mode: InteractionMode) {
        self.mode = mode;
    }

    /// Resize the screen buffer
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let new_cols = cols.max(1) as usize;
        let new_rows = rows.max(1) as usize;

        // Adjust rows
        if new_rows > self.rows {
            // Add new rows
            for _ in 0..(new_rows - self.rows) {
                self.cells.push(vec![ScreenCell::new(); self.cols]);
            }
        } else if new_rows < self.rows {
            // Remove rows from top (scroll)
            let remove_count = self.rows - new_rows;
            for _ in 0..remove_count {
                self.cells.remove(0);
            }
        }

        // Adjust columns
        for row in &mut self.cells {
            if new_cols > self.cols {
                // Expand row
                row.resize(new_cols, ScreenCell::new());
            } else if new_cols < self.cols {
                // Shrink row
                row.truncate(new_cols);
            }
        }

        self.cols = new_cols;
        self.rows = new_rows;

        // Ensure cursor is within bounds
        self.cursor_row = self.cursor_row.min(self.rows - 1);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
    }

    /// Clear the screen
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = ScreenCell::new();
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
