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

/// Starts FerroDock and positions its dock window within the usable desktop area.
///
/// # Examples
///
/// ```ignore
/// main();
/// ```
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

#[cfg(test)]
mod tests {
    use eframe::egui;

    #[test]
    fn viewport_builder_disables_resizing_and_maximize_button() {
        // Mirrors the exact chain of builder calls used in `main` to ensure
        // the newly added `.with_resizable(false)` / `.with_maximize_button(false)`
        // options actually take effect on the resulting `ViewportBuilder`.
        let viewport = egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_maximize_button(false)
            .with_inner_size([400.0, 80.0])
            .with_position([0.0, 0.0]);

        assert_eq!(viewport.resizable, Some(false));
        assert_eq!(viewport.maximize_button, Some(false));
    }

    #[test]
    fn viewport_builder_default_leaves_resizable_and_maximize_unset() {
        // Contrast case: without the new builder calls, egui's own defaults
        // leave these fields unset.
        let viewport = egui::ViewportBuilder::default();
        assert_eq!(viewport.resizable, None);
        assert_eq!(viewport.maximize_button, None);
    }

    #[test]
    fn cargo_toml_declares_appx_packaging_feature() {
        let manifest = include_str!("../Cargo.toml");
        assert!(
            manifest.contains("Win32_Storage_Packaging_Appx"),
            "Cargo.toml must enable the Win32_Storage_Packaging_Appx feature for the windows crate"
        );
    }

    #[test]
    fn cargo_toml_still_declares_preexisting_required_features() {
        let manifest = include_str!("../Cargo.toml");
        for feature in [
            "Win32_Foundation",
            "Win32_UI_WindowsAndMessaging",
            "Win32_Storage_FileSystem",
        ] {
            assert!(
                manifest.contains(feature),
                "expected Cargo.toml to still declare feature `{feature}`"
            );
        }
    }
}
