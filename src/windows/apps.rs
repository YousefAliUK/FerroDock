use windows::Win32::Foundation::{BOOL, CloseHandle, HMODULE, HWND, LPARAM};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetClassNameW, GetWindowThreadProcessId,
};

/// Determines whether a path identifies a UWP application.
///
/// The path is recognized when it contains a known UWP directory name or when
/// an ancestor directory contains an `AppxManifest.xml` file.
///
/// # Examples
///
/// ```
/// assert!(is_uwp_app(r"C:\Program Files\WindowsApps\App.exe"));
/// assert!(!is_uwp_app(r"C:\Program Files\App.exe"));
/// ```
///
/// # Arguments
///
/// * `path` - The application path to inspect.
///
/// # Returns
///
/// `true` if the path identifies a UWP application, `false` otherwise.
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

/// Retrieves the executable path of a UWP process associated with an application frame window.
///
/// # Examples
///
/// ```no_run
/// let path = get_uwp_real_process_path(HWND(0));
/// assert!(path.is_none() || path.is_some());
/// ```
///
/// # Returns
///
/// The child process executable path when `hwnd` identifies an application frame
/// window and a process path can be retrieved; otherwise, `None`.
///
/// # Parameters
///
/// * `hwnd` - Handle to the application frame window to inspect.
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

#[cfg(test)]
mod tests {
    use super::is_uwp_app;
    use std::fs;
    use std::path::PathBuf;

    /// Creates a fresh, uniquely-named temporary directory for a test so
    /// that filesystem-walking assertions in `is_uwp_app` are deterministic
    /// and independent from other tests running in parallel.
    fn unique_temp_dir(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ferro_dock_test_{label}_{pid}_{nanos}"));
        fs::create_dir_all(&dir).expect("failed to create temp test dir");
        dir
    }

    #[test]
    fn detects_windowsapps_substring_case_insensitively() {
        assert!(is_uwp_app(
            r"C:\Program Files\WindowsApps\Some.Package_1.0.0.0_x64__abc\App.exe"
        ));
        assert!(is_uwp_app(
            r"C:\PROGRAM FILES\WINDOWSAPPS\Some.Package\App.exe"
        ));
    }

    #[test]
    fn detects_systemapps_substring() {
        assert!(is_uwp_app(
            r"C:\Windows\SystemApps\Microsoft.Something\App.exe"
        ));
    }

    #[test]
    fn detects_immersivecontrolpanel_substring() {
        assert!(is_uwp_app(
            r"C:\Windows\ImmersiveControlPanel\SystemSettings.exe"
        ));
    }

    #[test]
    fn returns_false_for_plain_win32_app_without_manifest() {
        // No known substring match, and no AppxManifest.xml exists anywhere
        // in this (non-existent) ancestry.
        assert!(!is_uwp_app(r"C:\Program Files\Notepad++\notepad++.exe"));
    }

    #[test]
    fn returns_false_for_empty_path() {
        assert!(!is_uwp_app(""));
    }

    #[test]
    fn returns_false_for_single_component_relative_path() {
        // A bare filename with no directory component: `.parent()` yields an
        // empty relative path once, then `None`, so the ancestor-walking
        // loop must terminate without panicking.
        assert!(!is_uwp_app("standalone_app_reference_xyz.exe"));
    }

    #[test]
    fn detects_appxmanifest_in_immediate_parent_directory() {
        let root = unique_temp_dir("immediate_parent");
        fs::write(root.join("AppxManifest.xml"), "<Package/>").unwrap();

        let fake_exe = root.join("App.exe");
        assert!(is_uwp_app(fake_exe.to_str().unwrap()));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_appxmanifest_several_directories_up() {
        let root = unique_temp_dir("nested_parent");
        let nested = root.join("VFS").join("ProgramFiles").join("bin");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("AppxManifest.xml"), "<Package/>").unwrap();

        let fake_exe = nested.join("App.exe");
        assert!(is_uwp_app(fake_exe.to_str().unwrap()));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn returns_false_when_no_manifest_exists_anywhere_in_real_directory_tree() {
        let root = unique_temp_dir("no_manifest");
        let nested = root.join("sub").join("dir");
        fs::create_dir_all(&nested).unwrap();

        let fake_exe = nested.join("App.exe");
        assert!(!is_uwp_app(fake_exe.to_str().unwrap()));

        let _ = fs::remove_dir_all(&root);
    }
}
