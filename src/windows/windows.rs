use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow,
    GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, SetForegroundWindow, ShowWindow, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
    WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use crate::windows::DockIcon;

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
        if let Some(icon) = crate::windows::get_dock_icon_for_window(hwnd) {
            if !current_icons.iter().any(|i| i.path == icon.path) {
                current_icons.push(icon);
            }
        }
    }

    current_icons
}

pub fn is_dock_worthy_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let is_tool_window = (ex_style & WS_EX_TOOLWINDOW.0) != 0;
        let is_app_window = (ex_style & WS_EX_APPWINDOW.0) != 0;
        let has_owner = GetWindow(hwnd, GW_OWNER).0 != 0;

        if is_tool_window {
            return false;
        }

        if has_owner && !is_app_window {
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

pub fn focus_or_minimize_window(hwnd: HWND) {
    if hwnd.0 == 0 {
        return;
    }
    unsafe {
        if !IsWindow(hwnd).as_bool() {
            return;
        }

        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        } else {
            let mut foreground_pid = 0;
            let foreground_thread_id = GetWindowThreadProcessId(foreground, Some(&mut foreground_pid));
            let current_thread_id = GetCurrentThreadId();

            if foreground_thread_id != 0 && foreground_thread_id != current_thread_id {
                let _ = AttachThreadInput(current_thread_id, foreground_thread_id, true);
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                } else {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }
                let _ = SetForegroundWindow(hwnd);
                let _ = BringWindowToTop(hwnd);
                let _ = AttachThreadInput(current_thread_id, foreground_thread_id, false);
            } else {
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                } else {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }
                let _ = SetForegroundWindow(hwnd);
                let _ = BringWindowToTop(hwnd);
            }
        }
    }
}

pub fn get_window_title(hwnd: HWND) -> String {
    if hwnd.0 == 0 {
        return String::new();
    }
    unsafe {
        let mut buf: [u16; 512] = [0; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if is_dock_worthy_window(hwnd) {
        unsafe {
            let windows: &mut Vec<HWND> = &mut *(lparam.0 as *mut _);
            windows.push(hwnd);
        }
    }

    true.into()
}
