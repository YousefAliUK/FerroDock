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
    pub position_set: bool,
    event_receiver: Receiver<WindowEvent>,
}

impl Default for FerroDock {
    /// Creates a dock with default configuration and empty application and texture collections.
    ///
    /// # Examples
    ///
    /// ```
    /// let dock = FerroDock::default();
    /// assert!(!dock.position_set);
    /// ```
    fn default() -> Self {
        Self {
            config: Config::default(),
            dock_items: Vec::new(),
            icon_textures: HashMap::new(),
            pending_sync_frames: 0,
            position_set: false,
            event_receiver: events::start_event_listener(),
        }
    }
}

impl FerroDock {
    /// Creates a dock initialized with the currently running applications.
    ///
    /// # Examples
    ///
    /// ```
    /// let dock = FerroDock::new();
    /// assert!(!dock.position_set);
    /// ```
    pub fn new() -> Self {
        let initial_icons = update_running_apps();
        let event_receiver = events::start_event_listener();

        Self {
            config: Config::default(),
            dock_items: initial_icons,
            icon_textures: HashMap::new(),
            pending_sync_frames: 0,
            position_set: false,
            event_receiver,
        }
    }

    /// Processes queued window events and refreshes the dock state when events are available.
    ///
    /// Stale icon textures are removed after a refresh.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut dock = FerroDock::new();
    /// let refreshed = dock.process_window_events();
    ///
    /// assert!(!refreshed || dock.pending_sync_frames == 15);
    /// ```
    ///
    /// Returns `true` if at least one queued event was processed, `false` otherwise.
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
    /// Updates the dock's position, application state, icon textures, and rendered UI.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut dock = FerroDock::default();
    /// let context = egui::Context::default();
    /// let mut frame = eframe::Frame::default();
    ///
    /// dock.update(&context, &mut frame);
    /// assert!(dock.position_set);
    /// ```
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if !self.position_set {
            self.position_set = true;
            let mut work_area = RECT::default();
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                    windows::Win32::UI::WindowsAndMessaging::SPI_GETWORKAREA,
                    0,
                    Some(&mut work_area as *mut _ as *mut _),
                    windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                );
            }
            let hdc = unsafe { windows::Win32::Graphics::Gdi::GetDC(None) };
            let dpi = if hdc.is_invalid() {
                96.0
            } else {
                let d = unsafe { windows::Win32::Graphics::Gdi::GetDeviceCaps(hdc, windows::Win32::Graphics::Gdi::LOGPIXELSX) } as f32;
                let _ = unsafe { windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc) };
                d
            };
            let scale_factor = (dpi / 96.0).max(1.0);
            let work_left = (work_area.left as f32) / scale_factor;
            let work_right = (work_area.right as f32) / scale_factor;
            let work_bottom = (work_area.bottom as f32) / scale_factor;
            let work_w = work_right - work_left;
            let dock_width = (work_w * 0.5).min(750.0);
            let dock_height = 80.0;
            let pos_x = work_left + (work_w - dock_width) / 2.0;
            let pos_y = work_bottom - dock_height - 2.0;

            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(pos_x, pos_y)));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(dock_width, dock_height)));
        }

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

    /// Uses a fully transparent color to clear the application viewport.
    ///
    /// # Examples
    ///
    /// ```
    /// let color = egui::Color32::TRANSPARENT.to_normalized_gamma_f32();
    /// assert_eq!(color, [0.0, 0.0, 0.0, 0.0]);
    /// ```
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Builds a `FerroDock` directly (bypassing `new`/`default`) so tests can
    /// supply their own event channel instead of spinning up the real Win32
    /// shell-hook listener thread.
    fn make_dock_with_receiver(receiver: Receiver<WindowEvent>) -> FerroDock {
        FerroDock {
            config: Config::default(),
            dock_items: Vec::new(),
            icon_textures: HashMap::new(),
            pending_sync_frames: 0,
            position_set: false,
            event_receiver: receiver,
        }
    }

    #[test]
    fn default_initializes_position_set_to_false() {
        let dock = FerroDock::default();
        assert!(!dock.position_set);
        assert_eq!(dock.pending_sync_frames, 0);
        assert!(dock.icon_textures.is_empty());
    }

    #[test]
    fn new_initializes_position_set_to_false() {
        let dock = FerroDock::new();
        assert!(!dock.position_set);
    }

    #[test]
    fn process_window_events_returns_false_and_leaves_state_untouched_when_idle() {
        let (_sender, receiver) = mpsc::channel();
        let mut dock = make_dock_with_receiver(receiver);
        dock.pending_sync_frames = 3;
        dock.icon_textures
            .insert("C:\\some\\untouched\\app.exe".to_string(), {
                let ctx = egui::Context::default();
                ctx.load_texture(
                    "untouched",
                    egui::ColorImage::new([1, 1], egui::Color32::TRANSPARENT),
                    egui::TextureOptions::default(),
                )
            });

        let did_something = dock.process_window_events();

        assert!(!did_something);
        // No events were received, so nothing should have been mutated,
        // including the texture cache (no garbage collection should run).
        assert_eq!(dock.pending_sync_frames, 3);
        assert_eq!(dock.icon_textures.len(), 1);
    }

    #[test]
    fn process_window_events_garbage_collects_stale_textures_on_event() {
        let (sender, receiver) = mpsc::channel();
        sender.send(WindowEvent::WindowCreated).unwrap();
        let mut dock = make_dock_with_receiver(receiver);

        // Seed the texture cache with an entry for a path that cannot
        // possibly correspond to a currently running window.
        let stale_path = "C:\\definitely\\not\\a\\real\\running\\app_ferrodock_test.exe".to_string();
        let ctx = egui::Context::default();
        let texture = ctx.load_texture(
            "stale",
            egui::ColorImage::new([1, 1], egui::Color32::TRANSPARENT),
            egui::TextureOptions::default(),
        );
        dock.icon_textures.insert(stale_path.clone(), texture);

        let did_something = dock.process_window_events();

        assert!(did_something);
        assert_eq!(dock.pending_sync_frames, 15);
        assert!(
            !dock.icon_textures.contains_key(&stale_path),
            "stale texture entries no longer backed by a dock item must be evicted"
        );

        // Invariant enforced by the retain() call: every remaining texture
        // key must correspond to a path currently present in dock_items.
        let active_paths: std::collections::HashSet<&String> =
            dock.dock_items.iter().map(|i| &i.path).collect();
        for path in dock.icon_textures.keys() {
            assert!(active_paths.contains(path));
        }
    }
}
