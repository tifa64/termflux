use ratatui::layout::Position;
use ratatui::style::{Color, Modifier, Style};

use crate::animation::Animation;
use crate::color::{blend_rgb, convert_vt100_color, ratatui_color_to_rgb, resolve_to_rgb};
use crate::terminal_state::TerminalState;

pub struct Compositor {
    pub animation_enabled: bool,
    pub opacity: f32,
    pub fallback_bg: Color,
}

impl Compositor {
    /// Render the composited frame: animation background + shell overlay.
    pub fn render(
        &self,
        ts: &TerminalState,
        animation: &dyn Animation,
        frame: &mut ratatui::Frame,
    ) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Pass 1: Render animation into the entire buffer
        if self.animation_enabled {
            animation.render(area, buf);
        } else {
            // Fill with configured fallback background
            for row in 0..area.height {
                for col in 0..area.width {
                    let pos = Position::new(area.x + col, area.y + row);
                    if let Some(cell) = buf.cell_mut(pos) {
                        cell.set_symbol(" ");
                        cell.set_bg(self.fallback_bg);
                        cell.set_fg(Color::Reset);
                    }
                }
            }
        }

        // Pass 2: Overlay shell cells on top
        let screen = ts.screen();
        let rows = area.height.min(screen.size().0);
        let cols = area.width.min(screen.size().1);

        for row in 0..rows {
            for col in 0..cols {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };

                // Skip wide character continuation cells
                if cell.is_wide_continuation() {
                    continue;
                }

                let contents = cell.contents();
                let has_content = !contents.is_empty() && contents != " ";
                let has_explicit_bg = cell.bgcolor() != vt100::Color::Default;

                // If the cell has no content and no explicit background, let animation show
                if !has_content && !has_explicit_bg {
                    continue;
                }

                // This is a shell cell — overlay it
                let pos = Position::new(area.x + col, area.y + row);
                let Some(buf_cell) = buf.cell_mut(pos) else {
                    continue;
                };

                if contents.is_empty() {
                    buf_cell.set_symbol(" ");
                } else {
                    buf_cell.set_symbol(&contents);
                }

                let mut fg = convert_vt100_color(cell.fgcolor());
                let mut bg = convert_vt100_color(cell.bgcolor());

                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Opacity blending: blend shell bg with animation bg
                if self.opacity < 1.0 {
                    let anim_bg_rgb = ratatui_color_to_rgb(buf_cell.bg);
                    let shell_bg_rgb = resolve_to_rgb(cell.bgcolor(), false);
                    let blended = blend_rgb(anim_bg_rgb, shell_bg_rgb, self.opacity);
                    bg = Color::Rgb(blended.0, blended.1, blended.2);
                }

                buf_cell.set_fg(fg);
                buf_cell.set_bg(bg);

                let mut modifier = Modifier::empty();
                if cell.bold() {
                    modifier |= Modifier::BOLD;
                }
                if cell.italic() {
                    modifier |= Modifier::ITALIC;
                }
                if cell.underline() {
                    modifier |= Modifier::UNDERLINED;
                }
                buf_cell.set_style(Style::default().add_modifier(modifier));
            }
        }

        // Set cursor position
        if !screen.hide_cursor() {
            let (cursor_row, cursor_col) = screen.cursor_position();
            if cursor_row < area.height && cursor_col < area.width {
                frame.set_cursor_position(Position::new(cursor_col, cursor_row));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_cell_shows_animation() {
        let mut ts = TerminalState::new(24, 80);
        ts.process(b"hi");

        let screen = ts.screen();

        // Cell (0,0) has "h" → shell content
        let cell = screen.cell(0, 0).unwrap();
        let contents = cell.contents();
        let has_content = !contents.is_empty() && contents != " ";
        assert!(has_content);

        // Cell (0,5) is empty → animation shows through
        let cell = screen.cell(0, 5).unwrap();
        let contents = cell.contents();
        let has_content = !contents.is_empty() && contents != " ";
        let has_explicit_bg = cell.bgcolor() != vt100::Color::Default;
        assert!(!has_content && !has_explicit_bg);
    }

    #[test]
    fn test_colored_bg_is_shell_content() {
        let mut ts = TerminalState::new(24, 80);
        ts.process(b"\x1b[41m ");

        let screen = ts.screen();
        let cell = screen.cell(0, 0).unwrap();
        let has_explicit_bg = cell.bgcolor() != vt100::Color::Default;
        assert!(has_explicit_bg, "Colored bg should be treated as shell content");
    }

    #[test]
    fn test_compositor_struct_fields() {
        let comp = Compositor {
            animation_enabled: true,
            opacity: 0.8,
            fallback_bg: Color::Rgb(10, 10, 35),
        };
        assert_eq!(comp.opacity, 0.8);
        assert!(comp.animation_enabled);
        assert_eq!(comp.fallback_bg, Color::Rgb(10, 10, 35));
    }
}
