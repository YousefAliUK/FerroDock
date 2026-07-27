use eframe::egui::{self};
use eframe::{self};
use ::windows::Win32::Foundation::RECT;
use ::windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, LOGPIXELSX, ReleaseDC};
use ::windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

// Module(s)
mod app;
mod config;
mod events;
mod windows;

use app::*;

fn main() {
    let mut work_area = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work_area as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }

    let hdc = unsafe { GetDC(None) };
    let dpi = if hdc.is_invalid() {
        96.0
    } else {
        let d = unsafe { GetDeviceCaps(hdc, LOGPIXELSX) } as f32;
        let _ = unsafe { ReleaseDC(None, hdc) };
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_maximize_button(false)
            .with_inner_size([dock_width, dock_height])
            .with_position([pos_x, pos_y]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "FerroDock",
        options,
        Box::new(|_cc| Box::new(FerroDock::new())),
    );
}
