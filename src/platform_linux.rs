//! Linux glue with the same interface as `platform.rs`: single instance (a unix socket in `$XDG_RUNTIME_DIR`),
//! showing and hiding the main window, and the autostart `.desktop` file.

use eframe::egui::{self, ViewportCommand};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

pub const WINDOW_TITLE: &str = "Planora";

static CTX: OnceLock<egui::Context> = OnceLock::new();
static LISTENER: OnceLock<UnixListener> = OnceLock::new();
static VISIBLE: AtomicBool = AtomicBool::new(true);

/// Hands the egui context to the platform layer: showing and hiding the window goes through viewport commands.
pub fn init(ctx: &egui::Context) {
    let _ = CTX.set(ctx.clone());
}

/// Path of the single-instance socket. With `PLANORA_DATA_DIR` set (tests, screenshots) it gets a suffix,
/// so a test instance never talks to the user's real Planora.
fn socket_path() -> PathBuf {
    let suffix = match std::env::var("PLANORA_DATA_DIR") {
        Ok(d) if !d.is_empty() => {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            d.hash(&mut h);
            format!("-{:x}", h.finish())
        }
        _ => String::new(),
    };
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        // SAFETY: getuid has no preconditions.
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/planora-{}", unsafe { libc::getuid() })));
    let _ = std::fs::create_dir_all(&base);
    base.join(format!("planora{suffix}.sock"))
}

/// Claims the single-instance socket. Returns `false` when another Planora already runs;
/// in that case it has been asked to show its window.
pub fn claim_single_instance() -> bool {
    let path = socket_path();
    if let Ok(mut s) = UnixStream::connect(&path) {
        let _ = s.write_all(b"show\n");
        return false;
    }
    // Nobody answered: a stale socket file may be left over from a crash.
    let _ = std::fs::remove_file(&path);
    if let Ok(l) = UnixListener::bind(&path) {
        let _ = LISTENER.set(l);
    }
    true
}

/// Background thread: whenever a second launch connects, shows the main window and calls `on_show`.
pub fn watch_for_second_launch(on_show: impl Fn() + Send + 'static) {
    let Some(listener) = LISTENER.get() else { return };
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(conn).read_line(&mut line).is_ok() && line.trim() == "show" {
                show_main_window();
                on_show();
            }
        }
    });
}

pub fn show_main_window() {
    VISIBLE.store(true, Ordering::Relaxed);
    if let Some(ctx) = CTX.get() {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

pub fn hide_main_window() {
    VISIBLE.store(false, Ordering::Relaxed);
    if let Some(ctx) = CTX.get() {
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        ctx.request_repaint();
    }
}

/// Is the main window currently on screen (not hidden in the tray)?
pub fn main_window_visible() -> bool {
    VISIBLE.load(Ordering::Relaxed)
}

// ───────────────────────── autostart ─────────────────────────

fn autostart_file() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("autostart").join("planora.desktop"))
}

/// A `.desktop` `Exec` value: the path in double quotes with the reserved characters escaped.
fn exec_quote(path: &str) -> String {
    let mut s = String::from("\"");
    for c in path.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('"');
    s
}

fn entry(exe: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Planora\nComment=A local planner with reminders\nExec={} --minimized\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exec_quote(exe)
    )
}

/// The real state of the file, not a remembered setting.
pub fn autostart_enabled() -> bool {
    autostart_file().and_then(|f| std::fs::read_to_string(f).ok()).is_some_and(|t| t.lines().any(|l| l.starts_with("Exec=") && l.ends_with("--minimized")))
}

/// Writes or removes `~/.config/autostart/planora.desktop` (`"<exe>" --minimized`). Returns whether it succeeded.
pub fn set_autostart(on: bool) -> bool {
    let Some(file) = autostart_file() else { return false };
    if on {
        let Ok(exe) = std::env::current_exe() else { return false };
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(file, entry(&exe.display().to_string())).is_ok()
    } else {
        match std::fs::remove_file(file) {
            Ok(()) => true,
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_entry_quotes_the_path() {
        let e = entry("/opt/my apps/Planora");
        assert!(e.contains("Exec=\"/opt/my apps/Planora\" --minimized\n"));
        assert_eq!(exec_quote("/a\"b$c"), "\"/a\\\"b\\$c\"");
    }
}
