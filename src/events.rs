// TODO: Use RegisterShellHookWindow instead of window events for instant updates
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};

use windows::Win32::{
    Foundation::HWND,
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, MSG, TranslateMessage, WINEVENT_OUTOFCONTEXT,
            WINEVENT_SKIPOWNPROCESS,
        },
    },
};

const EVENT_OBJECT_CREATE: u32 = 0x8000;
const EVENT_OBJECT_DESTROY: u32 = 0x8001;
const EVENT_OBJECT_SHOW: u32 = 0x8002;
const EVENT_OBJECT_HIDE: u32 = 0x8003;

#[derive(Debug, Clone)]
pub enum WindowEvent {
    WindowCreated(isize),
    WindowDestroyed(isize),
    WindowShown(isize),
    WindowHidden(isize),
}

static EVENT_SENDER: OnceLock<Sender<WindowEvent>> = OnceLock::new();

extern "system" fn win_event_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if id_object != 0 {
        return;
    }

    if let Some(sender) = EVENT_SENDER.get() {
        let event = match event {
            EVENT_OBJECT_CREATE => WindowEvent::WindowCreated(hwnd.0 as isize),
            EVENT_OBJECT_DESTROY => WindowEvent::WindowDestroyed(hwnd.0 as isize),
            EVENT_OBJECT_SHOW => WindowEvent::WindowShown(hwnd.0 as isize),
            EVENT_OBJECT_HIDE => WindowEvent::WindowHidden(hwnd.0 as isize),
            _ => return,
        };

        let _ = sender.send(event);
    }
}

pub fn start_event_listener() -> Receiver<WindowEvent> {
    let (sender, receiver) = mpsc::channel();
    EVENT_SENDER
        .set(sender)
        .expect("Failed to set event sender");

    std::thread::spawn(|| unsafe {
        let hook_create = SetWinEventHook(
            EVENT_OBJECT_CREATE,
            EVENT_OBJECT_CREATE,
            None,
            Some(win_event_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        let hook_destroy = SetWinEventHook(
            EVENT_OBJECT_DESTROY,
            EVENT_OBJECT_DESTROY,
            None,
            Some(win_event_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        let hook_show = SetWinEventHook(
            EVENT_OBJECT_SHOW,
            EVENT_OBJECT_SHOW,
            None,
            Some(win_event_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        let hook_hide = SetWinEventHook(
            EVENT_OBJECT_HIDE,
            EVENT_OBJECT_HIDE,
            None,
            Some(win_event_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        if hook_create.is_invalid()
            || hook_destroy.is_invalid()
            || hook_show.is_invalid()
            || hook_hide.is_invalid()
        {
            eprintln!("Failed to install one or more window event hooks");
            return;
        }

        println!("Window event listener started");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }

        let _ = UnhookWinEvent(hook_create);
        let _ = UnhookWinEvent(hook_destroy);
        let _ = UnhookWinEvent(hook_show);
        let _ = UnhookWinEvent(hook_hide);
    });

    receiver
}
