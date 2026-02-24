mod animation;
mod app;
mod color;
mod compositor;
mod config;
mod input;
mod pty;
mod terminal_query;
mod terminal_state;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = config::Config::load();

    // Auto-detect terminal background if not set in config
    if config.background.is_none() {
        let detected = terminal_query::detect_background_color();
        config.resolve_background(detected);
    }

    let (cols, rows) =
        crossterm::terminal::size().unwrap_or((80, 24));
    let app = app::App::new(rows, cols, config)?;
    app.run().await
}
