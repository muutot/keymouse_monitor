use std::mem;
use std::ptr::null_mut;
use std::sync::{Arc, atomic::AtomicUsize};
use std::thread;

use parking_lot::RwLock;
use rdev::{Button, EventType};
use tokio::sync::watch;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};

use crate::{tinfo, terror, data::MonitorData, listener::{common, keyboard}};

extern "system" {
    fn CallNextHookEx(hhk: HHOOK, code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    fn GetMessageA(msg: *mut MSG, hwnd: *mut std::ffi::c_void, msgfiltermin: u32, msgfiltermax: u32) -> i32;
}

static mut KEYBOARD_HOOK: HHOOK = null_mut();
static mut MOUSE_HOOK: HHOOK = null_mut();

static mut CB: Option<common::CallbackData> = None;

unsafe extern "system" fn hook_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        if let Some(et) = msg_to_event(wparam, lparam) {
            if let Some(ref cb) = CB {
                common::process_event(&et, cb);
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn msg_to_event(wparam: WPARAM, lparam: LPARAM) -> Option<EventType> {
    let msg = wparam as u32;
    match msg {
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
        WM_LBUTTONDOWN => Some(EventType::ButtonPress(Button::Left)),
        WM_LBUTTONUP => Some(EventType::ButtonRelease(Button::Left)),
        WM_RBUTTONDOWN => Some(EventType::ButtonPress(Button::Right)),
        WM_RBUTTONUP => Some(EventType::ButtonRelease(Button::Right)),
        WM_MBUTTONDOWN => Some(EventType::ButtonPress(Button::Middle)),
        WM_MBUTTONUP => Some(EventType::ButtonRelease(Button::Middle)),
        WM_XBUTTONDOWN => {
            let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
            let code = (mouse.mouseData >> 16) as u8;
            Some(EventType::ButtonPress(Button::Unknown(code)))
        }
        WM_XBUTTONUP => {
            let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
            let code = (mouse.mouseData >> 16) as u8;
            Some(EventType::ButtonRelease(Button::Unknown(code)))
        }
        WM_MOUSEMOVE => None,
        WM_MOUSEWHEEL => {
            let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
            let delta = (mouse.mouseData >> 16) as i16;
            Some(EventType::Wheel { delta_x: 0, delta_y: (delta / 120) as i64 })
        }
        WM_MOUSEHWHEEL => {
            let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
            let delta = (mouse.mouseData >> 16) as i16;
            Some(EventType::Wheel { delta_x: (delta / 120) as i64, delta_y: 0 })
        }
        _ => None,
    }
}

pub fn start(data: Arc<RwLock<MonitorData>>, change_tx: watch::Sender<()>, client_count: Arc<AtomicUsize>) {
    thread::spawn(move || {
        unsafe {
            CB = Some(common::CallbackData { data, change_tx, client_count });

            KEYBOARD_HOOK = SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_callback), null_mut(), 0);
            if KEYBOARD_HOOK.is_null() {
                terror!("native", "Failed to set keyboard hook");
                return;
            }

            MOUSE_HOOK = SetWindowsHookExA(WH_MOUSE_LL, Some(hook_callback), null_mut(), 0);
            if MOUSE_HOOK.is_null() {
                terror!("native", "Failed to set mouse hook");
                return;
            }

            let mut msg = mem::zeroed();
            while GetMessageA(&mut msg, null_mut(), 0, 0) != 0 {}
        }
    });

    tinfo!("native", "Native Windows listener started.");
}
