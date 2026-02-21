use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

use super::Animation;

pub struct Star {
    pub x: f64,  // 0.0..1.0 normalized horizontal position
    pub y: f64,  // 0.0..1.0 normalized vertical position
    pub z: f64,  // 0.0..1.0 depth (1.0 = near, 0.0 = far)
    speed: f64,
    twinkle_phase: f64,
}

pub struct Starfield {
    pub stars: Vec<Star>,
    time: f64,
}

impl Starfield {
    pub fn new(count: usize) -> Self {
        let mut rng = rand::thread_rng();
        let stars = (0..count)
            .map(|_| {
                let z = rng.gen::<f64>();
                Star {
                    x: rng.gen(),
                    y: rng.gen(),
                    z,
                    speed: 0.02 + z * 0.08, // near stars move faster
                    twinkle_phase: rng.gen::<f64>() * std::f64::consts::TAU,
                }
            })
            .collect();
        Self { stars, time: 0.0 }
    }

    fn star_char(z: f64) -> &'static str {
        if z < 0.33 {
            "."
        } else if z < 0.66 {
            "*"
        } else {
            "+"
        }
    }

    fn star_color(z: f64, twinkle: f64) -> Color {
        // Base brightness by depth: far=dim, near=bright
        let base = 80.0 + z * 175.0;
        // Twinkle modulates brightness by ±30
        let brightness = (base + twinkle * 30.0).clamp(40.0, 255.0) as u8;
        Color::Rgb(brightness, brightness, brightness)
    }

    fn bg_color(row: u16, total_rows: u16) -> Color {
        // Gradient from dark navy (top) to slightly lighter navy (bottom)
        let t = if total_rows > 1 {
            row as f64 / (total_rows - 1) as f64
        } else {
            0.0
        };
        let r = (5.0 + t * 5.0) as u8;
        let g = (5.0 + t * 5.0) as u8;
        let b = (20.0 + t * 15.0) as u8;
        Color::Rgb(r, g, b)
    }
}

impl Animation for Starfield {
    fn tick(&mut self, dt: f64) {
        self.time += dt;
        for star in &mut self.stars {
            star.x -= star.speed * dt;
            if star.x < 0.0 {
                star.x += 1.0;
                // Randomize y position when wrapping
                star.y = rand::thread_rng().gen();
            }
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let width = area.width as f64;
        let height = area.height as f64;

        // Fill background with navy gradient
        for row in 0..area.height {
            let bg = Self::bg_color(row, area.height);
            for col in 0..area.width {
                let pos = Position::new(area.x + col, area.y + row);
                if let Some(cell) = buf.cell_mut(pos) {
                    cell.set_symbol(" ");
                    cell.set_bg(bg);
                }
            }
        }

        // Render stars
        for star in &self.stars {
            let col = (star.x * width) as u16;
            let row = (star.y * height) as u16;

            if col >= area.width || row >= area.height {
                continue;
            }

            let pos = Position::new(area.x + col, area.y + row);
            let twinkle = (self.time * 3.0 + star.twinkle_phase).sin();
            let fg = Self::star_color(star.z, twinkle);

            if let Some(cell) = buf.cell_mut(pos) {
                cell.set_symbol(Self::star_char(star.z));
                cell.set_fg(fg);
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starfield_renders_to_buffer() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let sf = Starfield::new(100);
        sf.render(area, &mut buf);
        // Assert at least some cells have non-space content (stars)
        let has_stars = (0..24).any(|y| {
            (0..80).any(|x| {
                buf.cell(Position::new(x, y))
                    .map(|c| c.symbol() != " ")
                    .unwrap_or(false)
            })
        });
        assert!(has_stars);
    }

    #[test]
    fn test_starfield_tick_moves_stars() {
        let mut sf = Starfield::new(50);
        let positions_before: Vec<f64> = sf.stars.iter().map(|s| s.x).collect();
        sf.tick(0.1);
        let positions_after: Vec<f64> = sf.stars.iter().map(|s| s.x).collect();
        assert_ne!(positions_before, positions_after);
    }

    #[test]
    fn test_starfield_background_gradient() {
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        let sf = Starfield::new(0); // no stars, just background
        sf.render(area, &mut buf);

        // Top row bg
        let top_bg = buf.cell(Position::new(0, 0)).unwrap().bg;
        // Bottom row bg
        let bottom_bg = buf.cell(Position::new(0, 4)).unwrap().bg;

        // Extract blue channel — bottom should be brighter
        if let (Color::Rgb(_, _, b_top), Color::Rgb(_, _, b_bottom)) = (top_bg, bottom_bg) {
            assert!(b_bottom >= b_top);
        } else {
            panic!("Expected Rgb colors");
        }
    }

    #[test]
    fn test_star_wraps_around() {
        let mut sf = Starfield::new(1);
        sf.stars[0].x = 0.01;
        sf.stars[0].speed = 1.0; // very fast
        sf.tick(0.1);
        // Star should have wrapped or be near 0
        assert!(sf.stars[0].x >= 0.0 && sf.stars[0].x <= 1.0);
    }
}
