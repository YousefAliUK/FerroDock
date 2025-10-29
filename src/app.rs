use eframe::{self, App, Frame, egui};
use egui::{Context, TextureHandle};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::win_api::{self, DockIcon};

pub struct FerroDock {
    pub config: Config,
    pub dock_items: Vec<DockIcon>,
    pub icon_textures: HashMap<String, TextureHandle>,
    pub last_update: Instant,
}

impl Default for FerroDock {
    fn default() -> Self {
        Self {
            config: Config::default(),
            dock_items: Vec::new(),
            icon_textures: HashMap::new(),
            last_update: Instant::now() - Duration::from_secs(10),
        }
    }
}

impl FerroDock {
    fn draw_dock_ui(&self, ui: &mut egui::Ui) {
        let Config {
            background_color,
            left_margin,
            right_margin,
            top_margin,
            bottom_margin,
            icon_spacing,
            icon_size,
            ..
        } = self.config;

        let _ = egui::Frame::none()
            .fill(background_color)
            .rounding(egui::Rounding::from(self.config.rounding))
            .inner_margin(egui::Margin {
                left: left_margin,
                right: right_margin,
                top: top_margin,
                bottom: bottom_margin,
            })
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = icon_spacing;

                    for item in &self.dock_items {
                        if let Some(texture) = self.icon_textures.get(&item.path) {
                            let button = egui::ImageButton::new(texture);

                            if ui.add_sized(icon_size, button).clicked() {
                                println!("Clicked on: {}", item.path)
                            }
                        }
                    }
                })
            });
    }
}

impl App for FerroDock {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if self.last_update.elapsed() > Duration::from_secs(2) {
            self.dock_items = win_api::update_running_apps();
            self.last_update = Instant::now();
        }

        for icon in &self.dock_items {
            if !self.icon_textures.contains_key(&icon.path) {
                if let Some(color_image) = win_api::hicon_to_color_image(icon.hicon) {
                    let texture = ctx.load_texture(&icon.path, color_image, Default::default());
                    self.icon_textures.insert(icon.path.clone(), texture);
                }
            }
        }

        egui::Area::new(egui::Id::new("ferro_dock_area"))
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -self.config.bottom_margin),
            )
            .show(ctx, |ui| {
                self.draw_dock_ui(ui);
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
