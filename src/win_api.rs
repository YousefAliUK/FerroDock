use std::path::Path;

use windows::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    System::{
        ProcessStatus::*,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
    UI::WindowsAndMessaging::*,
};
