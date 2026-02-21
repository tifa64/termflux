mod animation;
mod app;
mod color;
mod compositor;
mod config;
mod input;
mod pty;
mod terminal_state;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let (cols, rows) =
        crossterm::terminal::size().unwrap_or((80, 24));
    let app = app::App::new(rows, cols)?;
    app.run().await
}
