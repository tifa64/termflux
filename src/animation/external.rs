use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};

use crate::color::convert_vt100_color;

use super::Animation;

pub struct ExternalProgram {
    parser: vt100::Parser,
    rx: mpsc::Receiver<Vec<u8>>,
    _master: Box<dyn MasterPty + Send>,
}

/// Resolve the external command binary path.
/// 1. Check if it exists in PATH
/// 2. Check ~/.config/termflux/bin/
/// 3. If install_cmd is provided, run it via shell and re-check bin dir
/// 4. If repo_url is provided, clone + make + install to ~/.config/termflux/bin/
fn resolve_command(
    name: &str,
    repo_url: Option<&str>,
    install_cmd: Option<&str>,
) -> Option<String> {
    // 1. Check PATH
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    let bin_dir = termflux_bin_dir()?;
    let bin_path = bin_dir.join(name);

    // 2. Check ~/.config/termflux/bin/
    if bin_path.exists() {
        return Some(bin_path.to_string_lossy().to_string());
    }

    // 3. Run custom install command if provided
    if let Some(cmd) = install_cmd {
        std::fs::create_dir_all(&bin_dir).ok()?;
        eprintln!("termflux: '{}' not found, installing...", name);
        let status = Command::new("sh")
            .args(["-lc", cmd])
            .status()
            .ok()?;
        if status.success() {
            // Re-check PATH after install
            if let Ok(output) = Command::new("which").arg(name).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        eprintln!("termflux: '{}' installed successfully.", name);
                        return Some(path);
                    }
                }
            }
            // Re-check bin dir after install
            if bin_path.exists() {
                eprintln!("termflux: '{}' installed successfully.", name);
                return Some(bin_path.to_string_lossy().to_string());
            }
        }
        eprintln!("termflux: failed to install '{}'.", name);
    }

    // 4. Auto-install from repo
    let repo_url = repo_url?;
    let build_dir = termflux_config_dir()?.join("build");
    std::fs::create_dir_all(&bin_dir).ok()?;
    std::fs::create_dir_all(&build_dir).ok()?;

    let repo_dir = build_dir.join(name);

    // Clone if not already present
    if !repo_dir.exists() {
        eprintln!("termflux: cloning {} ...", repo_url);
        let status = Command::new("git")
            .args(["clone", "--depth", "1", repo_url])
            .arg(&repo_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
    }

    // Build with make
    eprintln!("termflux: building {} ...", name);
    let status = Command::new("make")
        .current_dir(&repo_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }

    // Find the built binary — check repo root and common locations
    let candidates = [
        repo_dir.join(name),
        repo_dir.join("build").join(name),
        repo_dir.join("bin").join(name),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            std::fs::copy(candidate, &bin_path).ok()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755)).ok()?;
            }
            return Some(bin_path.to_string_lossy().to_string());
        }
    }

    None
}

fn termflux_config_dir() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config").join("termflux"))
}

fn termflux_bin_dir() -> Option<PathBuf> {
    Some(termflux_config_dir()?.join("bin"))
}

impl ExternalProgram {
    /// Spawn an external command in a hidden PTY and capture its output.
    /// If the command is not found, attempts auto-install via install_cmd or repo_url.
    /// Returns None if the command cannot be found or fails to spawn.
    pub fn new(
        command: &str,
        rows: u16,
        cols: u16,
        repo_url: Option<&str>,
        install_cmd: Option<&str>,
    ) -> Option<Self> {
        // Parse command string into program + args
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        // Resolve the binary path
        let resolved = resolve_command(parts[0], repo_url, install_cmd)?;

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .ok()?;

        let mut cmd = CommandBuilder::new(&resolved);
        for arg in &parts[1..] {
            cmd.arg(arg);
        }

        let _child = pair.slave.spawn_command(cmd).ok()?;
        drop(pair.slave);

        let reader = pair.master.try_clone_reader().ok()?;
        let (tx, rx) = mpsc::channel();

        // Spawn a background thread to read PTY output
        std::thread::spawn(move || {
            pty_reader(reader, tx);
        });

        Some(Self {
            parser: vt100::Parser::new(rows, cols, 0),
            rx,
            _master: pair.master,
        })
    }

}

fn pty_reader(mut reader: Box<dyn Read + Send>, tx: mpsc::Sender<Vec<u8>>) {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

impl Animation for ExternalProgram {
    fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
        let _ = self._master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    fn tick(&mut self, _dt: f64) {
        // Drain all available bytes from the reader channel
        while let Ok(bytes) = self.rx.try_recv() {
            self.parser.process(&bytes);
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let screen = self.parser.screen();
        let rows = area.height.min(screen.size().0);
        let cols = area.width.min(screen.size().1);

        for row in 0..rows {
            for col in 0..cols {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };

                if cell.is_wide_continuation() {
                    continue;
                }

                let pos = Position::new(area.x + col, area.y + row);
                let Some(buf_cell) = buf.cell_mut(pos) else {
                    continue;
                };

                let contents = cell.contents();
                if contents.is_empty() {
                    buf_cell.set_symbol(" ");
                } else {
                    buf_cell.set_symbol(&contents);
                }

                let mut fg = convert_vt100_color(cell.fgcolor());
                let mut bg = convert_vt100_color(cell.bgcolor());

                // Resolve default colors to dark background for animation layer
                if bg == Color::Reset {
                    bg = Color::Rgb(0, 0, 0);
                }
                if fg == Color::Reset {
                    fg = Color::Rgb(204, 204, 204);
                }

                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
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
    }
}
