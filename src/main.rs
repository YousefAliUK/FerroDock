// Imports
use eframe::egui::{self, Context};
use eframe::{self, App, Frame};

// Configuration
#[derive(Default)]
struct Config {
    rounding: f32,
    icon_size: egui::Vec2,
    icon_spacing: f32,
    background_color: egui::Color32,
    // auto_hide: bool,
    // pinned_apps: Vec<String>,
    // background_opacity: f32,
    // animation_duration: f32,
    // show_labels: bool,
    // hide_delay: f32,
}

// Main application structure
struct FerroDock {
    config: Config,
    dock_items: Vec<String>,
}

impl FerroDock {
    fn draw_dock_ui(&self, ui: &mut egui::Ui) {
        let _ = egui::Frame::none()
            .fill(self.config.background_color)
            .rounding(egui::Rounding::from(self.config.rounding))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let icon_size = self.config.icon_size;
                        let icon_spacing = self.config.icon_spacing;
                        let dock_items_len = self.dock_items.len();

                        for (i, item) in self.dock_items.iter().enumerate() {
                            ui.add(egui::Button::new(item).min_size(icon_size));

                            if i < dock_items_len - 1 {
                                ui.add_space(icon_spacing);
                            }
                        }
                    })
                })
            });
    }
}

impl App for FerroDock {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("ferro_dock_panel")
                    .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                    .show_inside(ui, |ui| {
                        self.draw_dock_ui(ui);
                    })
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // [0.1, 0.1, 0.1, 1.0] // Dark background
        // [0.2, 0.2, 0.2, 1.0] // Slightly lighter dark background
        // [0.15, 0.15, 0.15, 1.0] // Medium dark background
        // [0.18, 0.18, 0.18, 1.0] // Balanced dark background
        // [0.12, 0.12, 0.12, 1.0] // Custom dark background
        // [0.13, 0.13, 0.13, 1.0] // Final choice for dark background
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

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
        Box::new(|_cc| Box::new(FerroDock::default())),
    );
}

impl Config {
    fn default() -> Self {
        Self {
            rounding: 12.0,
            icon_size: egui::vec2(32.0, 32.0),
            icon_spacing: 8.0,
            background_color: egui::Color32::from_rgba_unmultiplied(25, 25, 25, 180),
        }
    }
}

impl Default for FerroDock {
    fn default() -> Self {
        Self {
            config: Config::default(),
            dock_items: vec!["App1".to_string(), "App2".to_string(), "App3".to_string()],
        }
    }
}
