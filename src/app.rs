use eframe::{self, App, Frame, egui};
use egui::{Context, TextureHandle};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use crate::config::Config;
use crate::events::{self, WindowEvent};
use crate::win_api::{self, DockIcon};

use windows::Win32::Foundation::HWND;

pub struct FerroDock {
    pub config: Config,
    pub dock_items: Vec<DockIcon>,
    pub icon_textures: HashMap<String, TextureHandle>,
    event_receiver: Receiver<WindowEvent>,
}

impl Default for FerroDock {
    fn default() -> Self {
        Self {
            config: Config::default(),
            dock_items: Vec::new(),
            icon_textures: HashMap::new(),
            event_receiver: events::start_event_listener(),
        }
    }
}

impl FerroDock {
    pub fn new() -> Self {
        let initial_icons = win_api::update_running_apps();

        let event_receiver = events::start_event_listener();

        Self {
            config: Config::default(),
            dock_items: initial_icons,
            icon_textures: HashMap::new(),
            event_receiver,
        }
    }

    fn process_window_events(&mut self) -> bool {
        let mut did_something = false;

        while let Ok(event) = self.event_receiver.try_recv() {
            did_something = true;

            match event {
                WindowEvent::WindowCreated(hwnd_raw) => {
                    let hwnd = HWND(hwnd_raw as isize);

                    if let Some(icon) = win_api::get_dock_icon_for_window(hwnd) {
                        if !self.dock_items.iter().any(|i| i.path == icon.path) {
                            println!("✅ Adding: {}", icon.path);
                            self.dock_items.push(icon);
                        } else {
                            println!("❌ Already exists: {}", icon.path);
                        }
                    } else {
                        println!("❌ No icon found for window: {}", hwnd_raw);
                    }
                }

                WindowEvent::WindowDestroyed(hwnd_raw) => {
                    let _hwnd = HWND(hwnd_raw as isize);
                    println!("Window Destroyed");

                    if let Some(pos) = self.dock_items.iter().position(|item| item.hwnd == _hwnd) {
                        let path = self.dock_items[pos].path.clone();

                        let still_running = win_api::update_running_apps()
                            .iter()
                            .any(|app| app.path == path);

                        if !still_running {
                            println!("❌ Removing: {}", path);
                            let removed = self.dock_items.remove(pos);

                            if !self.dock_items.iter().any(|i| i.path == removed.path) {
                                self.icon_textures.remove(&removed.path);
                            }
                        } else {
                            if let Some(new_icon) = win_api::update_running_apps()
                                .into_iter()
                                .find(|app| app.path == path)
                            {
                                self.dock_items[pos].hwnd = new_icon.hwnd;
                                println!("✅ Updated: {}", path);
                            }
                        }
                    }
                }

                WindowEvent::WindowActivated(hwnd_raw) => {
                    let _hwnd = HWND(hwnd_raw as isize);
                }
            }
        }

        did_something
    }

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
        ctx.request_repaint();

        if self.process_window_events() {
            ctx.request_repaint();
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
