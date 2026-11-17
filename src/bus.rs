//! Commands sent to the UI from other threads (tray menu, toast clicks, second launch).

use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Just bring the window forward (already done by the sender).
    Show,
    OpenTask(String),
    NewTask,
    GoToday,
    Quit,
}

struct Bus {
    queue: Mutex<Vec<Cmd>>,
    ctx: eframe::egui::Context,
}

static BUS: OnceLock<Bus> = OnceLock::new();

pub fn init(ctx: eframe::egui::Context) {
    let _ = BUS.set(Bus { queue: Mutex::new(Vec::new()), ctx });
}

/// Queues a command and wakes the UI.
pub fn push(cmd: Cmd) {
    if let Some(b) = BUS.get() {
        if let Ok(mut q) = b.queue.lock() {
            q.push(cmd);
        }
        b.ctx.request_repaint();
    }
}

pub fn drain() -> Vec<Cmd> {
    BUS.get().and_then(|b| b.queue.lock().ok().map(|mut q| std::mem::take(&mut *q))).unwrap_or_default()
}

/// Is there a command nobody has taken yet?
pub fn pending() -> bool {
    BUS.get().and_then(|b| b.queue.lock().ok().map(|q| !q.is_empty())).unwrap_or(false)
}
