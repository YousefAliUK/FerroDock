use windows::Win32::Foundation::{BOOL, CloseHandle, HMODULE, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetClassNameW, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOOLWINDOW,
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

        let windows: &mut Vec<HWND> = &mut *(lparam.0 as *mut _);
        windows.push(hwnd);
    }

    true.into()
}

/// Check if there are actual File Explorer windows open (not just shell/taskbar)
fn has_explorer_windows() -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};

    static FOUND: AtomicBool = AtomicBool::new(false);
    FOUND.store(false, Ordering::SeqCst);

    extern "system" fn check_explorer(hwnd: HWND, _: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return true.into();
            }

            let mut class_name: [u16; 256] = [0; 256];
            let len = GetClassNameW(hwnd, &mut class_name);

            if len > 0 {
                let name = String::from_utf16_lossy(&class_name[..len as usize]);
                if name == "CabinetWClass" {
                    FOUND.store(true, Ordering::SeqCst);
                    return false.into();
                }
            }

            true.into()
        }
    }

    unsafe {
        let _ = EnumWindows(Some(check_explorer), LPARAM(0));
    }

    FOUND.load(Ordering::SeqCst)
}

pub fn has_visible_window(path: &str) -> bool {
    use std::sync::OnceLock;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    if path.to_lowercase().ends_with("explorer.exe") {
        return has_explorer_windows();
    }

    static TARGET_PATH: OnceLock<Mutex<String>> = OnceLock::new();
    static FOUND: AtomicBool = AtomicBool::new(false);
    // Initialize or update the target path
    let mutex = TARGET_PATH.get_or_init(|| Mutex::new(String::new()));
    *mutex.lock().unwrap() = path.to_string();
    FOUND.store(false, Ordering::SeqCst);
    extern "system" fn check_window(hwnd: HWND, _: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return true.into();
            }

            let mut cloaked: u32 = 0;
            if DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut _ as *mut _,
                std::mem::size_of::<u32>() as u32,
            )
            .is_ok()
                && cloaked != 0
            {
                return true.into();
            }

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

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return true.into();
            }
            if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
            {
                let mut buf: [u16; 260] = [0; 260];
                let len = GetModuleFileNameExW(handle, HMODULE(0), &mut buf);
                let _ = CloseHandle(handle);
                if len > 0 {
                    let proc_path = String::from_utf16_lossy(&buf[..len as usize]);

                    if let Some(mutex) = TARGET_PATH.get() {
                        if let Ok(target) = mutex.lock() {
                            if proc_path == *target {
                                FOUND.store(true, Ordering::SeqCst);
                                return false.into();
                            }
                        }
                    }
                }
            }
        }
        true.into()
    }
    unsafe {
        let _ = EnumWindows(Some(check_window), LPARAM(0));
    }

    FOUND.load(Ordering::SeqCst)
}
