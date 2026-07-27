use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{CloseHandle, HMODULE, HWND, LPARAM, WPARAM},
    Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetObjectW, HBITMAP,
    },
    System::{
        ProcessStatus::GetModuleFileNameExW,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
    UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
    UI::WindowsAndMessaging::{
        CopyIcon, DestroyIcon, GCLP_HICON, GetClassLongPtrW, GetIconInfo,
        GetWindowThreadProcessId, HICON, ICON_BIG, ICONINFO, SendMessageW, WM_GETICON,
    },
};

use crate::windows::is_dock_worthy_window;

#[derive(Clone, PartialEq)]
pub struct DockIcon {
    pub path: String,
    pub hicon: HICON,
    pub hwnd: HWND,
}

pub fn hicon_to_color_image(hicon: HICON) -> Option<eframe::egui::ColorImage> {
    if hicon.is_invalid() {
        return None;
    }

    let hicon_clone = match unsafe { CopyIcon(hicon) } {
        Ok(icon) => icon,
        Err(_) => return None,
    };

    let mut icon_info = ICONINFO::default();
    if unsafe { GetIconInfo(hicon_clone, &mut icon_info).is_err() } {
        let _ = unsafe { DestroyIcon(hicon_clone) };
        return None;
    }

    let hbm_color = icon_info.hbmColor;
    let hbm_mask = icon_info.hbmMask;

    let mut bmp = BITMAP::default();
    unsafe {
        if GetObjectW(
            hbm_color,
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        ) == 0
        {
            destroy_icon_data(hbm_color, hbm_mask, hicon_clone);
            return None;
        };
    };

    let width = bmp.bmWidth;
    let height = bmp.bmHeight;

    if width <= 0 || height <= 0 {
        destroy_icon_data(hbm_color, hbm_mask, hicon_clone);
        return None;
    };

    let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let hdc = unsafe { CreateCompatibleDC(None) };
    if hdc.is_invalid() {
        destroy_icon_data(hbm_color, hbm_mask, hicon_clone);
        return None;
    }

    let result = unsafe {
        GetDIBits(
            hdc,
            hbm_color,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bi as *mut _,
            DIB_RGB_COLORS,
        )
    };

    let _ = unsafe { DeleteDC(hdc) };

    if result == 0 {
        destroy_icon_data(hbm_color, hbm_mask, hicon_clone);
        return None;
    }

    let rgba_pixels: Vec<eframe::egui::Color32> = pixels
        .chunks_exact(4)
        .map(|p| eframe::egui::Color32::from_rgba_unmultiplied(p[2], p[1], p[0], p[3]))
        .collect();

    destroy_icon_data(hbm_color, hbm_mask, hicon_clone);

    Some(eframe::egui::ColorImage {
        size: [width as usize, height as usize],
        pixels: rgba_pixels,
    })
}

pub fn get_uwp_icon(exe_path: &str) -> Option<eframe::egui::ColorImage> {
    let mut current_dir = Path::new(exe_path).parent();

    while let Some(dir) = current_dir {
        let manifest_path = dir.join("AppxManifest.xml");
        if manifest_path.exists() {
            if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                if let Some(icon_relative) = parse_logo_from_manifest(&manifest_content) {
                    if let Some(icon_path) = find_best_icon(dir, &icon_relative) {
                        if let Some(img) = load_png_as_color_image(&icon_path) {
                            return Some(img);
                        }
                    }
                }
            }
        }
        current_dir = dir.parent();
    }

    None
}

fn parse_logo_from_manifest(xml: &str) -> Option<String> {
    for attr in ["Square150x150Logo", "Square44x44Logo", "Logo"] {
        if let Some(start) = xml.find(&format!("{}=\"", attr)) {
            let start = start + attr.len() + 2;
            if let Some(end) = xml[start..].find('"') {
                return Some(xml[start..start + end].to_string());
            }
        }
    }

    None
}

fn find_best_icon(package_dir: &Path, relative_path: &str) -> Option<std::path::PathBuf> {
    let base_path = package_dir.join(relative_path);

    if base_path.exists() {
        return Some(base_path);
    }

    let stem = base_path.file_stem()?.to_str()?;
    let parent = base_path.parent()?;

    for scale in ["400", "300", "200", "150", "125", "100"] {
        let scaled_name = format!("{}.scale-{}.png", stem, scale);
        let scaled_path = parent.join(&scaled_name);

        if scaled_path.exists() {
            return Some(scaled_path);
        }
    }

    for size in ["256", "128", "64", "48", "44", "32"] {
        let sized_name = format!("{}.targetsize-{}.png", stem, size);
        let sized_path = parent.join(&sized_name);

        if sized_path.exists() {
            return Some(sized_path);
        }
    }

    None
}

fn load_png_as_color_image(path: &std::path::PathBuf) -> Option<eframe::egui::ColorImage> {
    let img = image::open(path).ok()?.to_rgba8();
    let (w, h) = (img.width(), img.height());

    let resized = if w > 128 || h > 128 {
        image::imageops::resize(&img, 128, 128, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let size = [resized.width() as usize, resized.height() as usize];
    let pixels: Vec<eframe::egui::Color32> = resized
        .pixels()
        .map(|p| eframe::egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    Some(eframe::egui::ColorImage { size, pixels })
}

fn destroy_icon_data(hbm_color: HBITMAP, hbm_mask: HBITMAP, hicon: HICON) {
    unsafe {
        let _ = DeleteObject(hbm_color);
        let _ = DeleteObject(hbm_mask);
        let _ = DestroyIcon(hicon);
    }
}

pub fn get_dock_icon_for_window(hwnd: HWND) -> Option<DockIcon> {
    if !is_dock_worthy_window(hwnd) {
        return None;
    }

    let uwp_real_path = crate::windows::get_uwp_real_process_path(hwnd);

    let path_str = if let Some(real_path) = uwp_real_path {
        real_path
    } else {
        let mut process_id: u32 = 0;
        unsafe {
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        }

        if process_id == 0 {
            return None;
        }

        let process_handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                process_id,
            )
        };

        let Ok(handle) = process_handle else {
            return None;
        };

        let mut path_buf: [u16; 260] = [0; 260];
        let len = unsafe { GetModuleFileNameExW(handle, HMODULE(0), &mut path_buf) };
        let _ = unsafe { CloseHandle(handle) };

        if len == 0 {
            return None;
        }

        String::from_utf16_lossy(&path_buf[..len as usize])
    };

    // Skip our own window
    if let Ok(own_path) = std::env::current_exe() {
        if path_str == own_path.to_string_lossy() {
            return None;
        }
    }

    // Filter out known background system apps that aren't user-facing
    let background_apps = [
        "TextInputHost.exe",           // Windows IME
        "SearchHost.exe",              // Windows Search
        "StartMenuExperienceHost.exe", // Start Menu
        "ShellExperienceHost.exe",     // Shell components
        "LockApp.exe",                 // Lock screen
    ];

    if background_apps.iter().any(|app| path_str.ends_with(app)) {
        return None;
    }

    if !is_dock_worthy_window(hwnd) {
        return None;
    }

    let hicon = unsafe {
        let result = SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0));

        if result.0 != 0 {
            HICON(result.0)
        } else {
            let class_icon = GetClassLongPtrW(hwnd, GCLP_HICON);
            if class_icon != 0 {
                HICON(class_icon as isize)
            } else {
                let utf16_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
                let mut shfi = SHFILEINFOW::default();
                let res = SHGetFileInfoW(
                    PCWSTR(utf16_path.as_ptr()),
                    windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                    Some(&mut shfi as *mut _ as *mut _),
                    std::mem::size_of::<SHFILEINFOW>() as u32,
                    SHGFI_ICON | SHGFI_LARGEICON,
                );

                if res != 0 && !shfi.hIcon.is_invalid() {
                    shfi.hIcon
                } else {
                    HICON(0)
                }
            }
        }
    };

    if hicon.is_invalid() && !crate::windows::is_uwp_app(&path_str) {
        return None;
    }

    Some(DockIcon {
        hicon,
        path: path_str,
        hwnd,
    })
}
