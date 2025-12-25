use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
        MSG, RegisterClassW, RegisterShellHookWindow, RegisterWindowMessageW, TranslateMessage,
        WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
    },
};

use windows::core::w;

const HSHELL_WINDOWCREATED: usize = 0x0001;
const HSHELL_WINDOWDESTROYED: usize = 0x0002;
const HSHELL_WINDOWACTIVATED: usize = 0x0004;

#[derive(Debug, Clone)]
pub enum WindowEvent {
    WindowCreated(isize),
    WindowDestroyed(isize),
    WindowActivated(isize),
}

static EVENT_SENDER: OnceLock<Sender<WindowEvent>> = OnceLock::new();
static SHELL_HOOK_MSG: OnceLock<u32> = OnceLock::new();

pub fn start_event_listener() -> Receiver<WindowEvent> {
    let (sender, receiver) = mpsc::channel();
    EVENT_SENDER
        .set(sender)
        .expect("Failed to set event sender");

    std::thread::spawn(|| unsafe {
        let shell_msg = RegisterWindowMessageW(w!("SHELLHOOK"));
        SHELL_HOOK_MSG.set(shell_msg).ok();

        let class_name = w!("FerroDockShellHook");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(shell_hook_proc),
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("FerroDock Shell Hook"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        );

        if hwnd.0 == 0 {
            eprintln!("Failed to create shell hook window");
            return;
        }

        if !RegisterShellHookWindow(hwnd).as_bool() {
            eprintln!("Failed to register shell hook window");
            return;
        }

        println!("Shell hook window created");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    });

    receiver
}

extern "system" fn shell_hook_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let shell_msg = SHELL_HOOK_MSG.get().copied().unwrap_or(0);

        if msg == shell_msg {
            if let Some(sender) = EVENT_SENDER.get() {
                let event = match wparam.0 {
                    HSHELL_WINDOWCREATED => Some(WindowEvent::WindowCreated(lparam.0)),
                    HSHELL_WINDOWDESTROYED => Some(WindowEvent::WindowDestroyed(lparam.0)),
                    HSHELL_WINDOWACTIVATED => Some(WindowEvent::WindowActivated(lparam.0)),
                    _ => return LRESULT(0),
                };

                if let Some(e) = event {
                    let _ = sender.send(e);
                }
            }
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
