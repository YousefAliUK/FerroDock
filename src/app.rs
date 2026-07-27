// ! Future Improvements: Use IShellItemImageFactory for getting icons
use eframe::{self, App, Frame, egui};
use egui::{Context, TextureHandle};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::Receiver;

use crate::config::Config;
use crate::events::{self, WindowEvent};
use crate::windows::{
    DockIcon, focus_or_minimize_window, get_uwp_icon, get_window_title, hicon_to_color_image,
    is_uwp_app, update_running_apps,
};

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetCursorPos, GetWindowRect, IsWindow,
};

pub struct FerroDock {
    pub config: Config,
    pub dock_items: Vec<DockIcon>,
    pub icon_textures: HashMap<String, TextureHandle>,
    pub pending_sync_frames: u8,
    event_receiver: Receiver<WindowEvent>,
}

impl Default for FerroDock {
    fn default() -> Self {
        Self {
            config: Config::default(),
            dock_items: Vec::new(),
            icon_textures: HashMap::new(),
            pending_sync_frames: 0,
            event_receiver: events::start_event_listener(),
        }
    }
}

impl FerroDock {
    pub fn new() -> Self {
        let initial_icons = update_running_apps();
        let event_receiver = events::start_event_listener();

        Self {
            config: Config::default(),
            dock_items: initial_icons,
            icon_textures: HashMap::new(),
            pending_sync_frames: 0,
            event_receiver,
        }
    }

    fn process_window_events(&mut self) -> bool {
        let mut did_something = false;

        while let Ok(_event) = self.event_receiver.try_recv() {
            did_something = true;
        }

        if did_something {
            self.pending_sync_frames = 15;
            self.dock_items = update_running_apps();

            // Garbage-collect stale textures for applications no longer in the dock
            let active_paths: std::collections::HashSet<&String> =
                self.dock_items.iter().map(|i| &i.path).collect();
            self.icon_textures
                .retain(|path, _| active_paths.contains(path));
        }

        did_something
    }

    fn draw_dock_ui(&mut self, ui: &mut egui::Ui) {
        let Config {
            background_color,
            border_color,
            border_width,
            left_margin,
            right_margin,
            top_margin,
            bottom_margin,
            icon_spacing,
            icon_size,
            show_active_indicators,
            hover_scale,
            ..
        } = self.config;

        let frame = egui::Frame::none()
            .fill(background_color)
            .stroke(egui::Stroke::new(border_width, border_color))
            .rounding(egui::Rounding::from(self.config.rounding))
            .inner_margin(egui::Margin {
                left: left_margin,
                right: right_margin,
                top: top_margin,
                bottom: bottom_margin + if show_active_indicators { 4.0 } else { 0.0 },
            });

        let frame_response = frame.show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = icon_spacing;

                for item in &self.dock_items {
                    if let Some(texture) = self.icon_textures.get(&item.path) {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;

                            let (rect, response) = ui.allocate_exact_size(icon_size, egui::Sense::click());
                            let is_hovered = response.hovered();

                            let display_size = if is_hovered {
                                icon_size * hover_scale
                            } else {
                                icon_size
                            };

                            let icon_rect = egui::Rect::from_center_size(rect.center(), display_size);
                            let image = egui::Image::new(texture);
                            image.paint_at(ui, icon_rect);

                            // App title tooltip
                            let app_title = if item.hwnd.0 != 0 {
                                let title = get_window_title(item.hwnd);
                                if title.is_empty() {
                                    Path::new(&item.path)
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("App")
                                        .to_string()
                                } else {
                                    title
                                }
                            } else {
                                Path::new(&item.path)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("App")
                                    .to_string()
                            };

                            let response = response.on_hover_text(app_title);

                            if response.clicked() {
                                if item.hwnd.0 != 0 && unsafe { IsWindow(item.hwnd).as_bool() } {
                                    focus_or_minimize_window(item.hwnd);
                                } else {
                                    if is_uwp_app(&item.path) {
                                        let _ = Command::new("explorer.exe").arg(&item.path).spawn();
                                    } else {
                                        let _ = Command::new(&item.path).spawn();
                                    }
                                }
                            }

                            // macOS Active Indicator Dot
                            if show_active_indicators {
                                let has_window = item.hwnd.0 != 0;
                                let dot_color = if has_window {
                                    egui::Color32::from_rgba_unmultiplied(240, 240, 245, 220)
                                } else {
                                    egui::Color32::TRANSPARENT
                                };

                                let dot_center = egui::pos2(rect.center().x, rect.max.y + 2.0);
                                ui.painter().circle_filled(dot_center, 2.5, dot_color);
                            }
                        });
                    }
                }
            });
        });

        // Hardware Win32 Cursor Position Hit-Testing for Passthrough
        let is_cursor_over_dock = unsafe {
            let mut cursor_pt = POINT::default();
            if GetCursorPos(&mut cursor_pt).is_ok() {
                let main_hwnd = FindWindowW(windows::core::w!("eframe"), None);
                let main_hwnd = if main_hwnd.0 == 0 {
                    FindWindowW(None, windows::core::w!("FerroDock"))
                } else {
                    main_hwnd
                };

                if main_hwnd.0 != 0 {
                    let mut win_rect = RECT::default();
                    if GetWindowRect(main_hwnd, &mut win_rect).is_ok() {
                        let ppp = ui.ctx().pixels_per_point();
                        let dock_rect = frame_response.response.rect;

                        let dock_left = win_rect.left + (dock_rect.min.x * ppp) as i32;
                        let dock_top = win_rect.top + (dock_rect.min.y * ppp) as i32;
                        let dock_right = win_rect.left + (dock_rect.max.x * ppp) as i32;
                        let dock_bottom = win_rect.top + (dock_rect.max.y * ppp) as i32;

                        cursor_pt.x >= dock_left
                            && cursor_pt.x <= dock_right
                            && cursor_pt.y >= dock_top
                            && cursor_pt.y <= dock_bottom
                    } else {
                        false
                    }
                } else {
                    frame_response.response.hovered()
                }
            } else {
                false
            }
        } || frame_response.response.hovered();

        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::MousePassthrough(!is_cursor_over_dock));
    }
}

impl App for FerroDock {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if self.process_window_events() {
            ctx.request_repaint();
        }

        if self.pending_sync_frames > 0 {
            self.pending_sync_frames -= 1;
            self.dock_items = update_running_apps();
            ctx.request_repaint();
        }

        for icon in &self.dock_items {
            if !self.icon_textures.contains_key(&icon.path) {
                let color_image = if is_uwp_app(&icon.path) {
                    get_uwp_icon(&icon.path)
                } else {
                    hicon_to_color_image(icon.hicon)
                };

                if let Some(color_image) = color_image {
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
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}
