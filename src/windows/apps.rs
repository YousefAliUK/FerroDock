use windows::Win32::Foundation::{BOOL, CloseHandle, HMODULE, HWND, LPARAM};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindowVisible,
};

pub fn has_visible_uwp_window(target_path: &str) -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};

    static TARGET: OnceLock<Mutex<String>> = OnceLock::new();
    static FOUND: AtomicBool = AtomicBool::new(false);

    let mutex = TARGET.get_or_init(|| Mutex::new(String::new()));
    *mutex.lock().unwrap() = target_path.to_string();
    FOUND.store(false, Ordering::SeqCst);

    extern "system" fn check_afh(hwnd: HWND, _: LPARAM) -> BOOL {
        unsafe {
            // Check if this is an ApplicationFrameWindow
            let mut class_name: [u16; 256] = [0; 256];
            let len = GetClassNameW(hwnd, &mut class_name);
            if len == 0 {
                return true.into();
            }

            let name = String::from_utf16_lossy(&class_name[..len as usize]);
            if name != "ApplicationFrameWindow" {
                return true.into();
            }

            if !IsWindowVisible(hwnd).as_bool() {
                return true.into();
            }

            let target = if let Some(m) = TARGET.get() {
                if let Ok(t) = m.lock() {
                    t.clone()
                } else {
                    return true.into();
                }
            } else {
                return true.into();
            };

            // Get AFH process ID to distinguish from child processes
            let afh_pid = {
                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                pid
            };

            // Data passed to child enumeration callback
            struct ChildData {
                afh_pid: u32,
                target_path: String,
                found: bool,
            }

            let mut child_data = ChildData {
                afh_pid,
                target_path: target,
                found: false,
            };

            extern "system" fn check_child(hwnd: HWND, lparam: LPARAM) -> BOOL {
                unsafe {
                    let data = &mut *(lparam.0 as *mut ChildData);

                    let mut pid: u32 = 0;
                    GetWindowThreadProcessId(hwnd, Some(&mut pid));

                    // Child with different PID = the actual UWP app
                    if pid != 0 && pid != data.afh_pid {
                        if let Ok(handle) =
                            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                        {
                            let mut buf: [u16; 260] = [0; 260];
                            let len = GetModuleFileNameExW(handle, HMODULE(0), &mut buf);
                            let _ = CloseHandle(handle);

                            if len > 0 {
                                let child_path = String::from_utf16_lossy(&buf[..len as usize]);
                                if child_path == data.target_path {
                                    data.found = true;
                                    return false.into();
                                }
                            }
                        }
                    }

                    true.into()
                }
            }

            let _ = EnumChildWindows(
                hwnd,
                Some(check_child),
                LPARAM(&mut child_data as *mut _ as isize),
            );

            if child_data.found {
                FOUND.store(true, Ordering::SeqCst);
                return false.into();
            }

            true.into()
        }
    }

    unsafe {
        let _ = EnumWindows(Some(check_afh), LPARAM(0));
    }

    FOUND.load(Ordering::SeqCst)
}

pub fn is_uwp_app_running(exe_path: &str) -> bool {
    has_visible_uwp_window(exe_path)
}

pub fn is_uwp_app(path: &str) -> bool {
    path.contains("Program Files\\WindowsApps")
        || path.contains("Program Files/WindowsApps")
        || path.contains("ImmersiveControlPanel")
        || path.contains("SystemApps")
}
