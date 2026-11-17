#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bus;
mod config;
mod fmt;
mod i18n;
mod model;
#[allow(dead_code)]
mod nimvoro;
mod notify;
mod pdf;
mod platform;
mod strings;
#[allow(dead_code)]
mod theme;
mod tray;

use eframe::egui;

/// The repository, kept in one place (About page and the update check).
pub const REPO_URL: &str = "https://github.com/GrigoryyKach/planora";

fn main() -> eframe::Result<()> {
    if !platform::claim_single_instance() {
        // Another Planora is running (maybe hidden in the tray); it was asked to show its window.
        return Ok(());
    }
    let minimized = std::env::args().any(|a| a == "--minimized");
    let dir = config::data_dir();
    let cfg = config::Config::load_from(&dir);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(platform::WINDOW_TITLE)
        .with_app_id("local.planora")
        .with_min_inner_size([1000.0, 640.0])
        .with_visible(!minimized)
        .with_icon(std::sync::Arc::new(app_icon()));
    let mut centered = true;
    match cfg.window {
        Some(w) if w.w >= 1000.0 && w.h >= 640.0 => {
            viewport = viewport.with_inner_size([w.w, w.h]);
            if let (Some(x), Some(y)) = (w.x, w.y) {
                if x > -20000.0 && y > -20000.0 && x < 20000.0 && y < 20000.0 {
                    viewport = viewport.with_position([x, y]);
                    centered = false;
                }
            }
        }
        _ => viewport = viewport.with_inner_size([1280.0, 820.0]),
    }
    let options = eframe::NativeOptions { viewport, centered, ..Default::default() };
    eframe::run_native(platform::WINDOW_TITLE, options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, cfg, dir, minimized)))))
}

/// Window icon from `assets/icon.png` (the same artwork that is embedded in the exe).
fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default()
}
