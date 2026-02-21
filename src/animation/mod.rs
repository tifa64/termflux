pub mod fire;
pub mod matrix;
pub mod starfield;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub trait Animation: Send {
    fn tick(&mut self, dt: f64);
    fn render(&self, area: Rect, buf: &mut Buffer);
}
