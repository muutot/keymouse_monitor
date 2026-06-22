use std::borrow::Cow;
use std::ptr::null_mut;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::RwLock;
use rdev::{Button, EventType};
use tokio::sync::watch;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};

use crate::data::MonitorData;
use crate::maps;

extern "system" {
    fn CallNextHookEx(hhk: HHOOK, code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    fn GetMessageA(msg: *mut MSG, hwnd: *mut std::ffi::c_void, msgfiltermin: u32, msgfiltermax: u32) -> i32;
}

static mut KEYBOARD_HOOK: HHOOK = 0;
static mut MOUSE_HOOK: HHOOK = 0;

struct CallbackData {
    data: Arc<RwLock<MonitorData>>,
    change_tx: watch::Sender<()>,
    client_count: Arc<AtomicUsize>,
}

static mut CB: Option<CallbackData> = None;

unsafe extern "system" fn hook_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let event_type = msg_to_event(wparam, lparam);
        if let Some(et) = event_type {
            if matches!(et, EventType::MouseMove { .. }) {
                return CallNextHookEx(0, code, wparam, lparam);
            }
            if let Some(ref cb) = CB {
                let key_name: Option<Cow<'static, str>> = match &et {
                    EventType::KeyRelease(key) => maps::key_to_string(key),
                    EventType::ButtonPress(button) => maps::button_to_string(button),
                    EventType::Wheel { delta_x, delta_y } => {
                        if *delta_y > 0 {
                            Some(Cow::Borrowed("mouse_scroll_up"))
                        } else if *delta_y < 0 {
                            Some(Cow::Borrowed("mouse_scroll_down"))
                        } else if *delta_x > 0 {
                            Some(Cow::Borrowed("scroll_right_dir"))
                        } else if *delta_x < 0 {
                            Some(Cow::Borrowed("scroll_left_dir"))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(ref name) = key_name {
                    cb.data.write().increase_count(name);
                    if cb.client_count.load(Ordering::Relaxed) > 0 {
                        cb.change_tx.send_modify(|_| ());
                    }
                }
            }
        }
    }
    CallNextHookEx(0, code, wparam, lparam)
}

unsafe fn msg_to_event(wparam: WPARAM, lparam: LPARAM) -> Option<EventType> {
    let msg = wparam as u32;
    match msg {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
            let key = vk_to_key_with_ext(kb.vkCode, kb.flags & LLKHF_EXTENDED != 0);
            Some(EventType::KeyPress(key))
        }
        WM_KEYUP | WM_SYSKEYUP => {
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
            let key = vk_to_key_with_ext(kb.vkCode, kb.flags & LLKHF_EXTENDED != 0);
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
        WM_MOUSEMOVE => {
            let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
            Some(EventType::MouseMove { x: mouse.pt.x as f64, y: mouse.pt.y as f64 })
        }
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

fn vk_to_key_with_ext(vk: u32, extended: bool) -> rdev::Key {
    use rdev::Key;
    match vk {
        0x0D if extended => Key::KpReturn,
        _ => key_from_code(vk as u16),
    }
}

fn key_from_code(code: u16) -> rdev::Key {
    match code {
        164 => rdev::Key::Alt,
        165 => rdev::Key::AltGr,
        0x08 => rdev::Key::Backspace,
        20 => rdev::Key::CapsLock,
        162 => rdev::Key::ControlLeft,
        163 => rdev::Key::ControlRight,
        46 => rdev::Key::Delete,
        40 => rdev::Key::DownArrow,
        35 => rdev::Key::End,
        27 => rdev::Key::Escape,
        112 => rdev::Key::F1,
        113 => rdev::Key::F2,
        114 => rdev::Key::F3,
        115 => rdev::Key::F4,
        116 => rdev::Key::F5,
        117 => rdev::Key::F6,
        118 => rdev::Key::F7,
        119 => rdev::Key::F8,
        120 => rdev::Key::F9,
        121 => rdev::Key::F10,
        122 => rdev::Key::F11,
        123 => rdev::Key::F12,
        36 => rdev::Key::Home,
        45 => rdev::Key::Insert,
        37 => rdev::Key::LeftArrow,
        91 => rdev::Key::MetaLeft,
        92 => rdev::Key::MetaRight,
        144 => rdev::Key::NumLock,
        34 => rdev::Key::PageDown,
        33 => rdev::Key::PageUp,
        19 => rdev::Key::Pause,
        44 => rdev::Key::PrintScreen,
        0x0D => rdev::Key::Return,
        39 => rdev::Key::RightArrow,
        145 => rdev::Key::ScrollLock,
        160 => rdev::Key::ShiftLeft,
        161 => rdev::Key::ShiftRight,
        32 => rdev::Key::Space,
        0x09 => rdev::Key::Tab,
        38 => rdev::Key::UpArrow,

        65 => rdev::Key::KeyA,
        66 => rdev::Key::KeyB,
        67 => rdev::Key::KeyC,
        68 => rdev::Key::KeyD,
        69 => rdev::Key::KeyE,
        70 => rdev::Key::KeyF,
        71 => rdev::Key::KeyG,
        72 => rdev::Key::KeyH,
        73 => rdev::Key::KeyI,
        74 => rdev::Key::KeyJ,
        75 => rdev::Key::KeyK,
        76 => rdev::Key::KeyL,
        77 => rdev::Key::KeyM,
        78 => rdev::Key::KeyN,
        79 => rdev::Key::KeyO,
        80 => rdev::Key::KeyP,
        81 => rdev::Key::KeyQ,
        82 => rdev::Key::KeyR,
        83 => rdev::Key::KeyS,
        84 => rdev::Key::KeyT,
        85 => rdev::Key::KeyU,
        86 => rdev::Key::KeyV,
        87 => rdev::Key::KeyW,
        88 => rdev::Key::KeyX,
        89 => rdev::Key::KeyY,
        90 => rdev::Key::KeyZ,

        49 => rdev::Key::Num1,
        50 => rdev::Key::Num2,
        51 => rdev::Key::Num3,
        52 => rdev::Key::Num4,
        53 => rdev::Key::Num5,
        54 => rdev::Key::Num6,
        55 => rdev::Key::Num7,
        56 => rdev::Key::Num8,
        57 => rdev::Key::Num9,
        48 => rdev::Key::Num0,

        192 => rdev::Key::BackQuote,
        189 => rdev::Key::Minus,
        187 => rdev::Key::Equal,
        219 => rdev::Key::LeftBracket,
        221 => rdev::Key::RightBracket,
        186 => rdev::Key::SemiColon,
        222 => rdev::Key::Quote,
        220 => rdev::Key::BackSlash,
        226 => rdev::Key::IntlBackslash,
        188 => rdev::Key::Comma,
        190 => rdev::Key::Dot,
        191 => rdev::Key::Slash,

        109 => rdev::Key::KpMinus,
        107 => rdev::Key::KpPlus,
        106 => rdev::Key::KpMultiply,
        111 => rdev::Key::KpDivide,
        96 => rdev::Key::Kp0,
        97 => rdev::Key::Kp1,
        98 => rdev::Key::Kp2,
        99 => rdev::Key::Kp3,
        100 => rdev::Key::Kp4,
        101 => rdev::Key::Kp5,
        102 => rdev::Key::Kp6,
        103 => rdev::Key::Kp7,
        104 => rdev::Key::Kp8,
        105 => rdev::Key::Kp9,
        110 => rdev::Key::KpDelete,

        _ => rdev::Key::Unknown(code.into()),
    }
}

pub fn start(data: Arc<RwLock<MonitorData>>, change_tx: watch::Sender<()>, client_count: Arc<AtomicUsize>) {
    std::thread::spawn(move || {
        unsafe {
            CB = Some(CallbackData { data, change_tx, client_count });

            KEYBOARD_HOOK = SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_callback), 0, 0);
            if KEYBOARD_HOOK == 0 {
                eprintln!("Failed to set keyboard hook");
                return;
            }

            MOUSE_HOOK = SetWindowsHookExA(WH_MOUSE_LL, Some(hook_callback), 0, 0);
            if MOUSE_HOOK == 0 {
                eprintln!("Failed to set mouse hook");
                return;
            }

            let mut msg = std::mem::zeroed();
            while GetMessageA(&mut msg, null_mut(), 0, 0) != 0 {}
        }
    });

    println!("Native Windows listener started.");
}
