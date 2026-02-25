use noise::{NoiseFn, Perlin};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

use super::Animation;

pub struct Fire {
    perlin: Perlin,
    heat: Vec<Vec<f64>>, // [row][col] heat values 0.0..1.0
    width: usize,
    height: usize,
    time: f64,
    bg: Color,
}

impl Fire {
    pub fn new(cols: u16, rows: u16, bg: Color) -> Self {
        let width = cols as usize;
        let height = rows as usize;
        Self {
            perlin: Perlin::new(42),
            heat: vec![vec![0.0; width]; height],
            width,
            height,
            time: 0.0,
            bg,
        }
    }

    fn heat_to_color(heat: f64) -> (Color, &'static str) {
        let h = heat.clamp(0.0, 1.0);
        if h < 0.1 {
            (Color::Rgb(0, 0, 0), " ")
        } else if h < 0.3 {
            let t = (h - 0.1) / 0.2;
            let r = (180.0 * t) as u8;
            (Color::Rgb(r, 0, 0), "░")
        } else if h < 0.5 {
            let t = (h - 0.3) / 0.2;
            let r = 180 + (75.0 * t) as u8;
            let g = (80.0 * t) as u8;
            (Color::Rgb(r, g, 0), "▒")
        } else if h < 0.7 {
            let t = (h - 0.5) / 0.2;
            let r = 255;
            let g = 80 + (120.0 * t) as u8;
            (Color::Rgb(r, g, 0), "▓")
        } else {
            let t = (h - 0.7) / 0.3;
            let r = 255;
            let g = 200 + (55.0 * t) as u8;
            let b = (200.0 * t) as u8;
            (Color::Rgb(r, g, b), "█")
        }
    }
}

impl Animation for Fire {
    fn resize(&mut self, rows: u16, cols: u16) {
        let width = cols as usize;
        let height = rows as usize;
        self.heat = vec![vec![0.0; width]; height];
        self.width = width;
        self.height = height;
    }

    fn tick(&mut self, dt: f64) {
        self.time += dt;

        if self.height == 0 || self.width == 0 {
            return;
        }

        // Seed bottom row with Perlin noise-based heat
        let bottom = self.height - 1;
        for col in 0..self.width {
            let noise_val = self.perlin.get([
                col as f64 * 0.1,
                self.time * 3.0,
            ]);
            // noise_val is roughly -1..1, map to 0.5..1.0 for bottom heat
            self.heat[bottom][col] = (0.75 + noise_val * 0.35).clamp(0.4, 1.0);
        }

        // Propagate heat upward: each cell gets a weighted average of cells below + decay
        // Work from top to bottom-1 (bottom row is the source)
        for row in 0..bottom {
            let src_row = bottom - row; // source is below
            let dst_row = src_row - 1;  // destination is above
            for col in 0..self.width {
                let below = self.heat[src_row][col];
                let below_left = if col > 0 { self.heat[src_row][col - 1] } else { below };
                let below_right = if col + 1 < self.width { self.heat[src_row][col + 1] } else { below };

                // Weighted average with some randomness via noise
                let avg = below * 0.5 + below_left * 0.2 + below_right * 0.2
                    + self.heat[dst_row][col] * 0.1;

                // Decay factor — heat dissipates as it rises
                let decay = 0.88 + self.perlin.get([
                    col as f64 * 0.15,
                    dst_row as f64 * 0.15,
                    self.time * 2.0,
                ]) * 0.08;

                self.heat[dst_row][col] = (avg * decay).clamp(0.0, 1.0);
            }
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        for row in 0..area.height {
            for col in 0..area.width {
                let pos = Position::new(area.x + col, area.y + row);
                let heat = if (row as usize) < self.height && (col as usize) < self.width {
                    self.heat[row as usize][col as usize]
                } else {
                    0.0
                };

                let (fg_color, symbol) = Self::heat_to_color(heat);

                if let Some(cell) = buf.cell_mut(pos) {
                    cell.set_symbol(symbol);
                    cell.set_fg(fg_color);
                    cell.set_bg(self.bg);
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_renders_to_buffer() {
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        let mut fire = Fire::new(40, 12, Color::Rgb(0, 0, 0));
        // Tick several times to build up heat
        for _ in 0..30 {
            fire.tick(0.033);
        }
        fire.render(area, &mut buf);
        // Bottom rows should have some non-space characters (fire)
        let has_fire = (8..12).any(|y| {
            (0..40).any(|x| {
                buf.cell(Position::new(x, y))
                    .map(|c| c.symbol() != " ")
                    .unwrap_or(false)
            })
        });
        assert!(has_fire, "Bottom rows should have fire");
    }

    #[test]
    fn test_fire_tick_generates_heat() {
        let mut fire = Fire::new(40, 12, Color::Rgb(0, 0, 0));
        for _ in 0..20 {
            fire.tick(0.033);
        }
        // Bottom row should have significant heat
        let bottom_heat: f64 = fire.heat[11].iter().sum();
        assert!(bottom_heat > 0.0, "Bottom row should have heat");
    }

    #[test]
    fn test_fire_heat_propagates_upward() {
        let mut fire = Fire::new(40, 12, Color::Rgb(0, 0, 0));
        for _ in 0..60 {
            fire.tick(0.033);
        }
        // Middle rows should have some heat (propagated from bottom)
        let mid_heat: f64 = fire.heat[6].iter().sum();
        assert!(mid_heat > 0.0, "Heat should propagate to middle rows");
    }

    #[test]
    fn test_heat_to_color_black_for_no_heat() {
        let (color, symbol) = Fire::heat_to_color(0.0);
        assert_eq!(color, Color::Rgb(0, 0, 0));
        assert_eq!(symbol, " ");
    }

    #[test]
    fn test_heat_to_color_full_heat() {
        let (color, symbol) = Fire::heat_to_color(1.0);
        assert_eq!(symbol, "█");
        // Should be bright — check red channel is 255
        if let Color::Rgb(r, _, _) = color {
            assert_eq!(r, 255);
        } else {
            panic!("Expected Rgb color");
        }
    }
}
