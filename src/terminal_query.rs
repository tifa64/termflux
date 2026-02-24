use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::time::Duration;

/// RAII guard that disables raw mode on drop.
struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Query the terminal's background color via OSC 11.
///
/// Temporarily enters raw mode, sends the query escape, reads the response
/// with a 100ms timeout, then restores terminal state. Returns `Some("#rrggbb")`
/// on success, `None` if the terminal doesn't respond or parsing fails.
pub fn detect_background_color() -> Option<String> {
    let _guard = RawModeGuard::enable().ok()?;

    // Send OSC 11 query
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b]11;?\x07").ok()?;
    stdout.flush().ok()?;

    // Read response on a background thread with timeout
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 64];
        let mut collected = Vec::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        // Read until we get the ST terminator (\x07 or \x1b\\) or fill buffer
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.extend_from_slice(&buf[..n]);
                    // Check for BEL or ST terminator
                    if collected.contains(&0x07)
                        || collected
                            .windows(2)
                            .any(|w| w == b"\x1b\\")
                    {
                        break;
                    }
                    if collected.len() > 128 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(collected);
    });

    let data = rx.recv_timeout(Duration::from_millis(100)).ok()?;
    parse_osc11_response(&data)
}

/// Parse an OSC 11 response into a `#rrggbb` hex string.
///
/// Expected formats:
/// - `\x1b]11;rgb:RRRR/GGGG/BBBB\x07` (4-digit per channel, e.g. xterm)
/// - `\x1b]11;rgb:RR/GG/BB\x07` (2-digit per channel)
/// - ST terminator may be `\x07` or `\x1b\\`
pub fn parse_osc11_response(data: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(data).ok()?;

    // Find "rgb:" in the response
    let rgb_start = text.find("rgb:")?;
    let rgb_part = &text[rgb_start + 4..];

    // Trim any trailing control characters (BEL, ESC, etc.)
    let rgb_part = rgb_part
        .trim_end_matches('\x07')
        .trim_end_matches('\\')
        .trim_end_matches('\x1b');

    // Split into R/G/B components
    let components: Vec<&str> = rgb_part.split('/').collect();
    if components.len() != 3 {
        return None;
    }

    let mut rgb = [0u8; 3];
    for (i, comp) in components.iter().enumerate() {
        match comp.len() {
            4 => {
                // 4-digit: take high byte (first 2 hex digits)
                rgb[i] = u8::from_str_radix(&comp[..2], 16).ok()?;
            }
            2 => {
                rgb[i] = u8::from_str_radix(comp, 16).ok()?;
            }
            1 => {
                // Single digit like "f" → "ff"
                let val = u8::from_str_radix(comp, 16).ok()?;
                rgb[i] = val * 17; // 0xf -> 0xff
            }
            _ => return None,
        }
    }

    Some(format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_4digit_response() {
        // Standard xterm 4-digit-per-channel response
        let data = b"\x1b]11;rgb:0a0a/0a0a/2323\x07";
        assert_eq!(parse_osc11_response(data), Some("#0a0a23".to_string()));
    }

    #[test]
    fn test_parse_4digit_white() {
        let data = b"\x1b]11;rgb:ffff/ffff/ffff\x07";
        assert_eq!(parse_osc11_response(data), Some("#ffffff".to_string()));
    }

    #[test]
    fn test_parse_4digit_black() {
        let data = b"\x1b]11;rgb:0000/0000/0000\x07";
        assert_eq!(parse_osc11_response(data), Some("#000000".to_string()));
    }

    #[test]
    fn test_parse_2digit_response() {
        let data = b"\x1b]11;rgb:0a/0a/23\x07";
        assert_eq!(parse_osc11_response(data), Some("#0a0a23".to_string()));
    }

    #[test]
    fn test_parse_st_terminator() {
        // ESC \ terminator instead of BEL
        let data = b"\x1b]11;rgb:ff00/8000/0000\x1b\\";
        assert_eq!(parse_osc11_response(data), Some("#ff8000".to_string()));
    }

    #[test]
    fn test_parse_invalid_no_rgb() {
        let data = b"\x1b]11;unknown\x07";
        assert_eq!(parse_osc11_response(data), None);
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_osc11_response(b""), None);
    }

    #[test]
    fn test_parse_wrong_component_count() {
        let data = b"\x1b]11;rgb:ff/ff\x07";
        assert_eq!(parse_osc11_response(data), None);
    }

    #[test]
    fn test_parse_invalid_hex() {
        let data = b"\x1b]11;rgb:zzzz/0000/0000\x07";
        assert_eq!(parse_osc11_response(data), None);
    }
}
