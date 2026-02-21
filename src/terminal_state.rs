pub struct TerminalState {
    parser: vt100::Parser,
}

impl TerminalState {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, 1000),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_plain_text() {
        let mut ts = TerminalState::new(24, 80);
        ts.process(b"hello");
        let screen = ts.screen();
        assert_eq!(screen.cell(0, 0).unwrap().contents(), "h");
        assert_eq!(screen.cell(0, 4).unwrap().contents(), "o");
    }

    #[test]
    fn test_process_color_escape() {
        let mut ts = TerminalState::new(24, 80);
        ts.process(b"\x1b[31mred");
        let cell = ts.screen().cell(0, 0).unwrap();
        assert_eq!(cell.fgcolor(), vt100::Color::Idx(1));
    }

    #[test]
    fn test_resize() {
        let mut ts = TerminalState::new(24, 80);
        ts.resize(50, 120);
        let (rows, cols) = ts.screen().size();
        assert_eq!(rows, 50);
        assert_eq!(cols, 120);
    }
}
