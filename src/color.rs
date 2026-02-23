use ratatui::style::Color;

pub fn convert_vt100_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Linear interpolation between two RGB colors.
/// `factor` = 0.0 returns `a`, factor = 1.0 returns `b`.
pub fn blend_rgb(a: (u8, u8, u8), b: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    let f = factor.clamp(0.0, 1.0);
    let inv = 1.0 - f;
    (
        (a.0 as f32 * inv + b.0 as f32 * f + 0.5) as u8,
        (a.1 as f32 * inv + b.1 as f32 * f + 0.5) as u8,
        (a.2 as f32 * inv + b.2 as f32 * f + 0.5) as u8,
    )
}

/// Resolve a vt100::Color to concrete RGB values.
/// Default color resolves to a typical terminal foreground/background.
#[allow(dead_code)]
pub fn resolve_to_rgb(color: vt100::Color, is_fg: bool) -> (u8, u8, u8) {
    match color {
        vt100::Color::Default => {
            if is_fg {
                (204, 204, 204) // light gray foreground
            } else {
                (0, 0, 0) // black background
            }
        }
        vt100::Color::Idx(i) => idx_to_rgb(i),
        vt100::Color::Rgb(r, g, b) => (r, g, b),
    }
}

/// Resolve a ratatui::Color to concrete RGB values.
/// Used by the compositor to read back animation buffer colors for blending.
pub fn ratatui_color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => idx_to_rgb(i),
        Color::Reset => (0, 0, 0),
        Color::Black => (0, 0, 0),
        Color::Red => (128, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (128, 128, 0),
        Color::Blue => (0, 0, 128),
        Color::Magenta => (128, 0, 128),
        Color::Cyan => (0, 128, 128),
        Color::Gray => (192, 192, 192),
        Color::DarkGray => (128, 128, 128),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (0, 0, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
    }
}

/// Convert xterm 256-color index to RGB.
pub fn idx_to_rgb(i: u8) -> (u8, u8, u8) {
    match i {
        // Standard colors (0-7)
        0 => (0, 0, 0),       // black
        1 => (128, 0, 0),     // red
        2 => (0, 128, 0),     // green
        3 => (128, 128, 0),   // yellow
        4 => (0, 0, 128),     // blue
        5 => (128, 0, 128),   // magenta
        6 => (0, 128, 128),   // cyan
        7 => (192, 192, 192), // white (light gray)
        // Bright colors (8-15)
        8 => (128, 128, 128),  // bright black (dark gray)
        9 => (255, 0, 0),     // bright red
        10 => (0, 255, 0),    // bright green
        11 => (255, 255, 0),  // bright yellow
        12 => (0, 0, 255),    // bright blue
        13 => (255, 0, 255),  // bright magenta
        14 => (0, 255, 255),  // bright cyan
        15 => (255, 255, 255), // bright white
        // 6x6x6 color cube (16-231)
        16..=231 => {
            let idx = i - 16;
            let b_idx = idx % 6;
            let g_idx = (idx / 6) % 6;
            let r_idx = idx / 36;
            let r = if r_idx == 0 { 0 } else { 55 + 40 * r_idx };
            let g = if g_idx == 0 { 0 } else { 55 + 40 * g_idx };
            let b = if b_idx == 0 { 0 } else { 55 + 40 * b_idx };
            (r, g, b)
        }
        // Grayscale ramp (232-255)
        232..=255 => {
            let v = 8 + 10 * (i - 232);
            (v, v, v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_default_color() {
        assert_eq!(convert_vt100_color(vt100::Color::Default), Color::Reset);
    }

    #[test]
    fn test_convert_indexed_color() {
        assert_eq!(convert_vt100_color(vt100::Color::Idx(1)), Color::Indexed(1));
    }

    #[test]
    fn test_convert_rgb_color() {
        assert_eq!(
            convert_vt100_color(vt100::Color::Rgb(255, 128, 0)),
            Color::Rgb(255, 128, 0)
        );
    }

    #[test]
    fn test_blend_rgb_midpoint() {
        assert_eq!(blend_rgb((0, 0, 0), (100, 100, 100), 0.5), (50, 50, 50));
    }

    #[test]
    fn test_blend_rgb_full_a() {
        assert_eq!(blend_rgb((255, 0, 0), (0, 255, 0), 0.0), (255, 0, 0));
    }

    #[test]
    fn test_blend_rgb_full_b() {
        assert_eq!(blend_rgb((255, 0, 0), (0, 255, 0), 1.0), (0, 255, 0));
    }

    #[test]
    fn test_blend_rgb_clamps_factor() {
        // factor > 1.0 clamps to 1.0
        assert_eq!(blend_rgb((255, 0, 0), (0, 255, 0), 2.0), (0, 255, 0));
        // factor < 0.0 clamps to 0.0
        assert_eq!(blend_rgb((255, 0, 0), (0, 255, 0), -1.0), (255, 0, 0));
    }

    #[test]
    fn test_idx_to_rgb_standard_red() {
        assert_eq!(idx_to_rgb(1), (128, 0, 0));
    }

    #[test]
    fn test_idx_to_rgb_bright_green() {
        assert_eq!(idx_to_rgb(10), (0, 255, 0));
    }

    #[test]
    fn test_idx_to_rgb_color_cube() {
        // Index 16 = (0,0,0) — first cube entry
        assert_eq!(idx_to_rgb(16), (0, 0, 0));
        // Index 196 = pure red in cube: r=5, g=0, b=0 → (255, 0, 0)
        // 196 - 16 = 180; 180/36 = 5, remainder 0 → g=0, b=0
        assert_eq!(idx_to_rgb(196), (255, 0, 0));
    }

    #[test]
    fn test_idx_to_rgb_grayscale_ramp() {
        // Index 232 = darkest gray
        let (r, _, _) = idx_to_rgb(232);
        assert_eq!(r, 8);
        // Index 255 = lightest gray
        let (r, _, _) = idx_to_rgb(255);
        assert_eq!(r, 238);
    }

    #[test]
    fn test_resolve_to_rgb_default_fg() {
        assert_eq!(resolve_to_rgb(vt100::Color::Default, true), (204, 204, 204));
    }

    #[test]
    fn test_resolve_to_rgb_default_bg() {
        assert_eq!(resolve_to_rgb(vt100::Color::Default, false), (0, 0, 0));
    }

    #[test]
    fn test_resolve_to_rgb_indexed() {
        assert_eq!(resolve_to_rgb(vt100::Color::Idx(9), false), (255, 0, 0));
    }

    #[test]
    fn test_resolve_to_rgb_rgb() {
        assert_eq!(resolve_to_rgb(vt100::Color::Rgb(10, 20, 30), true), (10, 20, 30));
    }

    #[test]
    fn test_ratatui_color_to_rgb() {
        assert_eq!(ratatui_color_to_rgb(Color::Rgb(10, 20, 30)), (10, 20, 30));
        assert_eq!(ratatui_color_to_rgb(Color::Indexed(1)), (128, 0, 0));
        assert_eq!(ratatui_color_to_rgb(Color::Reset), (0, 0, 0));
        assert_eq!(ratatui_color_to_rgb(Color::Red), (128, 0, 0));
    }
}
