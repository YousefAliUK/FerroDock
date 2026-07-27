use eframe::egui;

// Configuration
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    pub rounding: f32,
    pub icon_size: egui::Vec2,
    pub icon_spacing: f32,
    pub background_color: egui::Color32,
    pub border_color: egui::Color32,
    pub border_width: f32,
    pub right_margin: f32,
    pub left_margin: f32,
    pub top_margin: f32,
    pub bottom_margin: f32,
    pub show_active_indicators: bool,
    pub hover_scale: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rounding: 16.0,
            icon_size: egui::vec2(40.0, 40.0),
            icon_spacing: 10.0,
            background_color: egui::Color32::from_rgba_unmultiplied(20, 20, 25, 210),
            border_color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 35),
            border_width: 1.0,
            right_margin: 14.0,
            left_margin: 14.0,
            top_margin: 8.0,
            bottom_margin: 8.0,
            show_active_indicators: true,
            hover_scale: 1.25,
        }
    }
}
