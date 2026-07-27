use windows::Win32::Foundation::{BOOL, CloseHandle, HMODULE, HWND, LPARAM};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetClassNameW, GetWindowThreadProcessId,
};

pub fn is_uwp_app(path: &str) -> bool {
    let lower = path.to_lowercase();
    if lower.contains("windowsapps")
        || lower.contains("systemapps")
        || lower.contains("immersivecontrolpanel")
    {
        return true;
    }

    let mut current = std::path::Path::new(path).parent();
    while let Some(dir) = current {
        if dir.join("AppxManifest.xml").exists() {
            return true;
        }
        current = dir.parent();
    }

    false
}

pub fn get_uwp_real_process_path(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut class_name: [u16; 256] = [0; 256];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len == 0 {
            return None;
        }

        let name = String::from_utf16_lossy(&class_name[..len as usize]);
        if name != "ApplicationFrameWindow" {
            return None;
        }

        let afh_pid = {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            pid
        };

        struct TargetData {
            afh_pid: u32,
            path: Option<String>,
        }

        let mut data = TargetData {
            afh_pid,
            path: None,
        };

        extern "system" fn check_child(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                let data = &mut *(lparam.0 as *mut TargetData);
                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));

                if pid != 0 && pid != data.afh_pid {
                    if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
                        let mut buf: [u16; 260] = [0; 260];
                        let len = GetModuleFileNameExW(handle, HMODULE(0), &mut buf);
                        let _ = CloseHandle(handle);

                        if len > 0 {
                            let child_path = String::from_utf16_lossy(&buf[..len as usize]);
                            data.path = Some(child_path);
                            return false.into();
                        }
                    }
                }
                true.into()
            }
        }

        let _ = EnumChildWindows(
            hwnd,
            Some(check_child),
            LPARAM(&mut data as *mut _ as isize),
        );

        data.path
    }
}
