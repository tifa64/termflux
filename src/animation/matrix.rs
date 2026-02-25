use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

use super::Animation;

/// Half-width katakana range for matrix-style characters
const KATAKANA: &[char] = &[
    'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ',
    'ｰ', 'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ',
    'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ',
    'ﾄ', 'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ', 'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ',
    'ﾎ', 'ﾏ', 'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ', 'ﾔ', 'ﾕ', 'ﾖ', 'ﾗ',
    'ﾘ', 'ﾙ', 'ﾚ', 'ﾛ', 'ﾜ', 'ﾝ',
];

struct Drop {
    col: u16,
    y: f64,       // current head position (fractional row)
    speed: f64,   // rows per second
    length: u16,  // trail length
    chars: Vec<char>,
}

pub struct MatrixRain {
    drops: Vec<Drop>,
    max_cols: u16,
    max_rows: u16,
    time: f64,
    spawn_accum: f64, // accumulated time for spawning new drops
    bg: Color,
}

impl MatrixRain {
    pub fn new(cols: u16, rows: u16, bg: Color) -> Self {
        Self {
            drops: Vec::new(),
            max_cols: cols,
            max_rows: rows,
            time: 0.0,
            spawn_accum: 0.0,
            bg,
        }
    }

    fn random_char() -> char {
        let mut rng = rand::thread_rng();
        KATAKANA[rng.gen_range(0..KATAKANA.len())]
    }

    fn spawn_drop(col: u16, max_rows: u16) -> Drop {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(5..=max_rows.max(5));
        let speed = rng.gen_range(8.0..25.0);
        let chars: Vec<char> = (0..length).map(|_| Self::random_char()).collect();
        Drop {
            col,
            y: -(length as f64),
            speed,
            length,
            chars,
        }
    }

    fn trail_color(distance_from_head: u16, trail_length: u16) -> Color {
        if distance_from_head == 0 {
            // Head: bright white-green
            Color::Rgb(200, 255, 200)
        } else {
            let t = distance_from_head as f64 / trail_length as f64;
            // Fade from bright green to dark green
            let g = (255.0 * (1.0 - t * 0.8)) as u8;
            let r = (50.0 * (1.0 - t)) as u8;
            Color::Rgb(r, g, 0)
        }
    }
}

impl Animation for MatrixRain {
    fn resize(&mut self, rows: u16, cols: u16) {
        self.max_cols = cols;
        self.max_rows = rows;
    }

    fn tick(&mut self, dt: f64) {
        self.time += dt;
        self.spawn_accum += dt;

        // Move existing drops
        for drop in &mut self.drops {
            drop.y += drop.speed * dt;
            // Randomize a char occasionally for shimmer effect
            let mut rng = rand::thread_rng();
            if rng.gen::<f64>() < 0.05 {
                let idx = rng.gen_range(0..drop.chars.len());
                drop.chars[idx] = Self::random_char();
            }
        }

        // Remove drops that have fully scrolled off screen
        self.drops.retain(|d| {
            (d.y as i32 - d.length as i32) < self.max_rows as i32
        });

        // Spawn new drops at a steady rate (~2 per frame at 30fps → ~60/sec spread across columns)
        let spawn_interval = 0.015;
        while self.spawn_accum >= spawn_interval {
            self.spawn_accum -= spawn_interval;
            if self.max_cols > 0 {
                let col = rand::thread_rng().gen_range(0..self.max_cols);
                self.drops.push(Self::spawn_drop(col, self.max_rows));
            }
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let bg = self.bg;
        for row in 0..area.height {
            for col in 0..area.width {
                let pos = Position::new(area.x + col, area.y + row);
                if let Some(cell) = buf.cell_mut(pos) {
                    cell.set_symbol(" ");
                    cell.set_bg(bg);
                }
            }
        }

        // Render drops
        for drop in &self.drops {
            let head_row = drop.y as i32;

            for i in 0..drop.length {
                let row = head_row - i as i32;
                if row < 0 || row >= area.height as i32 {
                    continue;
                }
                if drop.col >= area.width {
                    continue;
                }

                let pos = Position::new(area.x + drop.col, area.y + row as u16);
                let fg = Self::trail_color(i, drop.length);
                let ch = drop.chars[i as usize % drop.chars.len()];

                if let Some(cell) = buf.cell_mut(pos) {
                    let mut s = [0u8; 4];
                    cell.set_symbol(ch.encode_utf8(&mut s));
                    cell.set_fg(fg);
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_renders_to_buffer() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let mut rain = MatrixRain::new(80, 24, Color::Rgb(0, 0, 0));
        // Tick several times to let drops spawn and fall
        for _ in 0..30 {
            rain.tick(0.033);
        }
        rain.render(area, &mut buf);
        // Should have at least some non-space cells
        let has_chars = (0..24).any(|y| {
            (0..80).any(|x| {
                buf.cell(Position::new(x, y))
                    .map(|c| c.symbol() != " ")
                    .unwrap_or(false)
            })
        });
        assert!(has_chars);
    }

    #[test]
    fn test_matrix_tick_spawns_drops() {
        let mut rain = MatrixRain::new(80, 24, Color::Rgb(0, 0, 0));
        assert!(rain.drops.is_empty());
        for _ in 0..10 {
            rain.tick(0.033);
        }
        assert!(!rain.drops.is_empty());
    }

    #[test]
    fn test_matrix_drops_removed_offscreen() {
        let mut rain = MatrixRain::new(80, 24, Color::Rgb(0, 0, 0));
        // Spawn a drop and move it far off screen
        rain.drops.push(Drop {
            col: 0,
            y: 100.0,
            speed: 10.0,
            length: 5,
            chars: vec!['ｱ'; 5],
        });
        rain.tick(0.0); // triggers cleanup
        assert!(rain.drops.is_empty());
    }

    #[test]
    fn test_trail_color_head_is_bright() {
        let head = MatrixRain::trail_color(0, 10);
        assert_eq!(head, Color::Rgb(200, 255, 200));
    }
}
