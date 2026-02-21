use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use vt100::MouseProtocolEncoding;
use vt100::MouseProtocolMode;

pub fn encode_key_event(key: &KeyEvent, application_cursor: bool) -> Vec<u8> {
    // Ctrl+key combinations
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let byte = match c {
                'a'..='z' => c as u8 - b'a' + 1,
                '@' => 0,
                '[' => 27,
                '\\' => 28,
                ']' => 29,
                '^' => 30,
                '_' => 31,
                _ => return vec![],
            };
            return vec![byte];
        }
    }

    // Alt+key → ESC prefix
    if key.modifiers.contains(KeyModifiers::ALT) {
        if let KeyCode::Char(c) = key.code {
            let mut buf = vec![0x1b];
            let mut char_buf = [0u8; 4];
            buf.extend_from_slice(c.encode_utf8(&mut char_buf).as_bytes());
            return buf;
        }
    }

    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf).as_bytes().to_vec()
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Up => {
            if application_cursor {
                b"\x1bOA".to_vec()
            } else {
                b"\x1b[A".to_vec()
            }
        }
        KeyCode::Down => {
            if application_cursor {
                b"\x1bOB".to_vec()
            } else {
                b"\x1b[B".to_vec()
            }
        }
        KeyCode::Right => {
            if application_cursor {
                b"\x1bOC".to_vec()
            } else {
                b"\x1b[C".to_vec()
            }
        }
        KeyCode::Left => {
            if application_cursor {
                b"\x1bOD".to_vec()
            } else {
                b"\x1b[D".to_vec()
            }
        }
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::F(n) => encode_function_key(n),
        _ => vec![],
    }
}

fn encode_function_key(n: u8) -> Vec<u8> {
    match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => vec![],
    }
}

/// Encode a crossterm MouseEvent into terminal escape bytes.
/// Returns empty vec if the mouse protocol mode doesn't support this event.
pub fn encode_mouse_event(
    event: &MouseEvent,
    mode: MouseProtocolMode,
    encoding: MouseProtocolEncoding,
) -> Vec<u8> {
    if mode == MouseProtocolMode::None {
        return vec![];
    }

    let (button_code, is_release) = match event.kind {
        MouseEventKind::Down(btn) => (mouse_button_code(btn), false),
        MouseEventKind::Up(btn) => {
            if mode == MouseProtocolMode::Press {
                return vec![]; // X10 mode: press only
            }
            (mouse_button_code(btn), true)
        }
        MouseEventKind::Drag(btn) => {
            if mode != MouseProtocolMode::ButtonMotion && mode != MouseProtocolMode::AnyMotion {
                return vec![];
            }
            (mouse_button_code(btn) + 32, false) // motion flag
        }
        MouseEventKind::Moved => {
            if mode != MouseProtocolMode::AnyMotion {
                return vec![];
            }
            (35, false) // no button + motion flag (32 + 3)
        }
        MouseEventKind::ScrollUp => (64, false),
        MouseEventKind::ScrollDown => (65, false),
        MouseEventKind::ScrollLeft => (66, false),
        MouseEventKind::ScrollRight => (67, false),
    };

    // 1-based coordinates for the protocol
    let col = event.column + 1;
    let row = event.row + 1;

    match encoding {
        MouseProtocolEncoding::Sgr => {
            let suffix = if is_release { 'm' } else { 'M' };
            format!("\x1b[<{};{};{}{}", button_code, col, row, suffix).into_bytes()
        }
        MouseProtocolEncoding::Default | MouseProtocolEncoding::Utf8 => {
            if is_release {
                // Normal mode: release is button 3
                let cb = 3 + 32; // 35
                vec![b'\x1b', b'[', b'M', cb as u8, (col as u8).saturating_add(32), (row as u8).saturating_add(32)]
            } else {
                let cb = button_code + 32;
                vec![b'\x1b', b'[', b'M', cb as u8, (col as u8).saturating_add(32), (row as u8).saturating_add(32)]
            }
        }
    }
}

fn mouse_button_code(btn: MouseButton) -> u8 {
    match btn {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_char() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), vec![b'a']);
    }

    #[test]
    fn test_encode_enter() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), vec![b'\r']);
    }

    #[test]
    fn test_encode_backspace() {
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), vec![0x7f]);
    }

    #[test]
    fn test_encode_ctrl_c() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(encode_key_event(&key, false), vec![0x03]);
    }

    #[test]
    fn test_encode_ctrl_a() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(encode_key_event(&key, false), vec![0x01]);
    }

    #[test]
    fn test_encode_ctrl_z() {
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(encode_key_event(&key, false), vec![0x1a]);
    }

    #[test]
    fn test_encode_arrow_normal_mode() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), b"\x1b[A".to_vec());
    }

    #[test]
    fn test_encode_arrow_application_mode() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, true), b"\x1bOA".to_vec());
    }

    #[test]
    fn test_encode_alt_key() {
        let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
        assert_eq!(encode_key_event(&key, false), b"\x1bb".to_vec());
    }

    #[test]
    fn test_encode_function_keys() {
        let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), b"\x1bOP".to_vec());

        let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), b"\x1b[15~".to_vec());
    }

    #[test]
    fn test_encode_home_end() {
        let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), b"\x1b[H".to_vec());

        let key = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), b"\x1b[F".to_vec());
    }

    #[test]
    fn test_encode_utf8_char() {
        let key = KeyEvent::new(KeyCode::Char('日'), KeyModifiers::NONE);
        assert_eq!(encode_key_event(&key, false), "日".as_bytes().to_vec());
    }

    // Mouse encoding tests

    #[test]
    fn test_mouse_none_mode_returns_empty() {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        assert!(encode_mouse_event(&event, MouseProtocolMode::None, MouseProtocolEncoding::Sgr).is_empty());
    }

    #[test]
    fn test_mouse_sgr_left_press() {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        let bytes = encode_mouse_event(&event, MouseProtocolMode::PressRelease, MouseProtocolEncoding::Sgr);
        assert_eq!(bytes, b"\x1b[<0;6;11M"); // button 0, col 6 (1-based), row 11
    }

    #[test]
    fn test_mouse_sgr_left_release() {
        let event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        let bytes = encode_mouse_event(&event, MouseProtocolMode::PressRelease, MouseProtocolEncoding::Sgr);
        assert_eq!(bytes, b"\x1b[<0;6;11m"); // lowercase m for release
    }

    #[test]
    fn test_mouse_sgr_right_press() {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        let bytes = encode_mouse_event(&event, MouseProtocolMode::PressRelease, MouseProtocolEncoding::Sgr);
        assert_eq!(bytes, b"\x1b[<2;1;1M"); // button 2, col 1, row 1
    }

    #[test]
    fn test_mouse_sgr_scroll_up() {
        let event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        let bytes = encode_mouse_event(&event, MouseProtocolMode::Press, MouseProtocolEncoding::Sgr);
        assert_eq!(bytes, b"\x1b[<64;11;6M"); // scroll up = 64
    }

    #[test]
    fn test_mouse_normal_left_press() {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        let bytes = encode_mouse_event(&event, MouseProtocolMode::PressRelease, MouseProtocolEncoding::Default);
        // cb = 0 + 32 = 32, col = 5 + 1 + 32 = 38, row = 10 + 1 + 32 = 43
        assert_eq!(bytes, vec![0x1b, b'[', b'M', 32, 38, 43]);
    }

    #[test]
    fn test_mouse_press_only_ignores_release() {
        let event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        assert!(encode_mouse_event(&event, MouseProtocolMode::Press, MouseProtocolEncoding::Sgr).is_empty());
    }

    #[test]
    fn test_mouse_drag_requires_motion_mode() {
        let event = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        // PressRelease mode doesn't support drag
        assert!(encode_mouse_event(&event, MouseProtocolMode::PressRelease, MouseProtocolEncoding::Sgr).is_empty());
        // ButtonMotion mode supports drag
        let bytes = encode_mouse_event(&event, MouseProtocolMode::ButtonMotion, MouseProtocolEncoding::Sgr);
        assert_eq!(bytes, b"\x1b[<32;11;6M"); // button 0 + 32 motion flag
    }
}
