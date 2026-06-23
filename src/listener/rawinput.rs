use std::mem;
use std::ptr::null_mut;
use std::sync::{atomic::AtomicUsize, Arc};
use std::thread;

use parking_lot::RwLock;
use rdev::{Button, EventType};
use tokio::sync::watch;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::{
    data::MonitorData,
    listener::{common, keyboard},
    terror, tinfo,
};

static mut KEYBOARD_HOOK: HHOOK = null_mut();

static mut CB: Option<common::CallbackData> = None;

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam as u32;
        let event_type = match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
                let key = keyboard::vk_to_key_with_ext(kb.vkCode, kb.flags & LLKHF_EXTENDED != 0);
                Some(EventType::KeyPress(key))
            }
            WM_KEYUP | WM_SYSKEYUP => {
                let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
                let key = keyboard::vk_to_key_with_ext(kb.vkCode, kb.flags & LLKHF_EXTENDED != 0);
                Some(EventType::KeyRelease(key))
            }
            _ => None,
        };

        if let Some(et) = event_type {
            if let Some(ref cb) = CB {
                common::process_event(&et, cb);
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn process_raw_input(lparam: LPARAM) {
    let raw = match keymouse_rawinput::read_raw_input(lparam) {
        Some(raw) if raw.header.dwType == RIM_TYPEMOUSE => raw,
        _ => return,
    };

    let mouse = &raw.data.mouse;
    let flags = mouse.Anonymous.Anonymous.usButtonFlags as u32;
    if flags == 0 {
        return;
    }

    if let Some(ref cb) = CB {
        let data = mouse.Anonymous.Anonymous.usButtonData;

        if flags & RI_MOUSE_LEFT_BUTTON_DOWN != 0 {
            common::process_event(&EventType::ButtonPress(Button::Left), cb);
        }
        if flags & RI_MOUSE_LEFT_BUTTON_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Left), cb);
        }
        if flags & RI_MOUSE_RIGHT_BUTTON_DOWN != 0 {
            common::process_event(&EventType::ButtonPress(Button::Right), cb);
        }
        if flags & RI_MOUSE_RIGHT_BUTTON_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Right), cb);
        }
        if flags & RI_MOUSE_MIDDLE_BUTTON_DOWN != 0 {
            common::process_event(&EventType::ButtonPress(Button::Middle), cb);
        }
        if flags & RI_MOUSE_MIDDLE_BUTTON_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Middle), cb);
        }
        if flags & RI_MOUSE_BUTTON_4_DOWN != 0 {
            common::process_event(&EventType::ButtonPress(Button::Unknown(1)), cb);
        }
        if flags & RI_MOUSE_BUTTON_4_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Unknown(1)), cb);
        }
        if flags & RI_MOUSE_BUTTON_5_DOWN != 0 {
            common::process_event(&EventType::ButtonPress(Button::Unknown(2)), cb);
        }
        if flags & RI_MOUSE_BUTTON_5_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Unknown(2)), cb);
        }
        if flags & RI_MOUSE_WHEEL != 0 {
            let delta = data as i16;
            common::process_event(
                &EventType::Wheel {
                    delta_x: 0,
                    delta_y: (delta / 120) as i64,
                },
                cb,
            );
        }
        if flags & RI_MOUSE_HWHEEL != 0 {
            let delta = data as i16;
            common::process_event(
                &EventType::Wheel {
                    delta_x: (delta / 120) as i64,
                    delta_y: 0,
                },
                cb,
            );
        }
    }
}

pub fn start(
    data: Arc<RwLock<MonitorData>>,
    change_tx: watch::Sender<()>,
    client_count: Arc<AtomicUsize>,
) {
    thread::spawn(move || unsafe {
        CB = Some(common::CallbackData {
            data,
            change_tx,
            client_count,
        });

        let hwnd = match keymouse_rawinput::create_message_window(
            b"KeyMouseMonWndClass\0",
            Some(DefWindowProcA),
        ) {
            Ok(h) => h,
            Err(e) => {
                terror!("rawinput", "{}", e);
                return;
            }
        };

        if !keymouse_rawinput::register_raw_input_device(
            hwnd,
            0x01,
            0x02,
            RIDEV_INPUTSINK | RIDEV_NOLEGACY,
        ) {
            terror!("rawinput", "Failed to register raw input device");
            return;
        }

        KEYBOARD_HOOK = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook), null_mut(), 0);
        if KEYBOARD_HOOK.is_null() {
            terror!("rawinput", "Failed to set keyboard hook");
            return;
        }

        let mut msg = mem::zeroed();
        while GetMessageA(&mut msg, null_mut(), 0, 0) != 0 {
            if msg.message == WM_INPUT {
                process_raw_input(msg.lParam);
            } else {
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
        }
    });

    tinfo!("rawinput", "Raw Input listener started.");
}
