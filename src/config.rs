use eframe::egui;

// Configuration
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    pub rounding: f32,
    pub icon_size: egui::Vec2,
    pub icon_spacing: f32,
    pub background_color: egui::Color32,
    pub right_margin: f32,
    pub left_margin: f32,
    pub top_margin: f32,
    pub bottom_margin: f32,
    // auto_hide: bool,
    // pinned_apps: Vec<String>,
    // background_opacity: f32,
    // animation_duration: f32,
    // show_labels: bool,
    // hide_delay: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rounding: 12.0,
            icon_size: egui::vec2(32.0, 32.0),
            icon_spacing: 8.0,
            background_color: egui::Color32::from_rgba_unmultiplied(25, 25, 25, 180),
            right_margin: 10.0,
            left_margin: 10.0,
            top_margin: 5.0,
            bottom_margin: 5.0,
        }
    }
}
