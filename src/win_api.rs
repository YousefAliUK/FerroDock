use windows::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    System::{
        ProcessStatus::*,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
    UI::WindowsAndMessaging::*,
};

// Represents an Icon
#[derive(Clone, PartialEq)]
pub struct DockIcon {
    pub path: String,
    pub hicon: HICON,
}

pub fn is_dock_worthy_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        if GetWindowTextLengthW(hwnd) == 0 {
            return false;
        }

        if GetWindow(hwnd, GW_OWNER).0 != 0 {
            return false;
        }

        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if (style & WS_EX_TOOLWINDOW.0) != 0 {
            return false;
        }

        let mut is_cloacked: u32 = 0;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut is_cloacked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && is_cloacked != 0
        {
            return false;
        }

        true
    }
}

pub fn get_dock_icon_for_window(hwnd: HWND) -> Option<DockIcon> {
    if !is_dock_worthy_window(hwnd) {
        return None;
    }

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

    let path_str = String::from_utf16_lossy(&path_buf[..len as usize]);

    let app_own_path = std::env::current_exe().ok()?.to_string_lossy().to_string();

    if path_str == app_own_path {
        return None;
    }

    let hicon = unsafe {
        let result = SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0));

        if result.0 == 0 {
            HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as isize)
        } else {
            HICON(result.0)
        }
    };

    if hicon.is_invalid() {
        return None;
    }

    Some(DockIcon {
        hicon,
        path: path_str,
    })
}

pub fn update_running_apps() -> Vec<DockIcon> {
    let mut open_windows: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut open_windows as *mut _ as isize),
        );
    }

    let mut current_icons: Vec<DockIcon> = Vec::new();

    for hwnd in open_windows {
        let mut process_id: u32 = 0;
        unsafe {
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        };

        if process_id != 0 {
            let process_handle = unsafe {
                OpenProcess(
                    PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                    false,
                    process_id,
                )
            };

            if let Ok(handle) = process_handle {
                let mut path_buf: [u16; 260] = [0; 260];

                let len = unsafe { GetModuleFileNameExW(handle, HMODULE(0), &mut path_buf) };

                if len > 0 {
                    let path_str = String::from_utf16_lossy(&path_buf[..len as usize]);

                    let app_own_path = std::env::current_exe()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    if path_str == app_own_path {
                        let _ = unsafe { CloseHandle(handle) };
                        continue;
                    }

                    if !current_icons.iter().any(|icon| icon.path == path_str) {
                        let hicon = unsafe {
                            SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0))
                        };

                        let hicon = if hicon.0 == 0 {
                            unsafe { HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as isize) }
                        } else {
                            HICON(hicon.0)
                        };

                        if !hicon.is_invalid() {
                            current_icons.push(DockIcon {
                                hicon,
                                path: path_str,
                            })
                        }
                    }
                }

                let _ = unsafe { CloseHandle(handle) };
            }
        }
    }

    current_icons
}

pub fn hicon_to_color_image(hicon: HICON) -> Option<eframe::egui::ColorImage> {
    let hicon_clone = match unsafe { CopyIcon(hicon) } {
        Ok(icon) => icon,
        Err(_) => return None,
    };
    let _ = hicon_clone;

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

fn destroy_icon_data(hbm_color: HBITMAP, hbm_mask: HBITMAP, hicon: HICON) {
    unsafe {
        let _ = DeleteObject(hbm_color);
        let _ = DeleteObject(hbm_mask);
        let _ = DestroyIcon(hicon);
    }
}

extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        };

        if GetWindowTextLengthW(hwnd) == 0 {
            return true.into();
        }

        if GetWindow(hwnd, GW_OWNER).0 != 0 {
            return true.into();
        }

        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

        if (style & WS_EX_TOOLWINDOW.0) != 0 {
            return true.into();
        }

        let mut is_cloacked: u32 = 0;

        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut is_cloacked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && is_cloacked != 0
        {
            return true.into();
        }

        let windows: &mut Vec<HWND> = &mut *(lparam.0 as *mut _);
        windows.push(hwnd);
    }

    true.into()
}
