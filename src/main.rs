use eframe::egui::{self};
use eframe::{self};

// Module(s)
mod app;
mod config;
mod events;
mod windows;

use app::*;

// Main application structure
fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "FerroDock",
        options,
        Box::new(|_cc| Box::new(FerroDock::new())),
    );
}
