use std::ptr::{null_mut, read_unaligned};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use parking_lot::RwLock;
use rdev::{Button, EventType};
use tokio::sync::watch;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::data::MonitorData;
use crate::listener::{common, keyboard};

const RAW_BUF_SIZE: usize = 64;

static mut KEYBOARD_HOOK: HHOOK = 0;

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
    CallNextHookEx(0, code, wparam, lparam)
}

unsafe fn process_raw_input(lparam: LPARAM) {
    let mut buf = [0u8; RAW_BUF_SIZE];
    let mut size = RAW_BUF_SIZE as u32;
    let written = GetRawInputData(
        lparam as _,
        RID_INPUT,
        buf.as_mut_ptr() as *mut _,
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );
    if written == u32::MAX || written < std::mem::size_of::<RAWINPUTHEADER>() as u32 {
        return;
    }

    let raw: RAWINPUT = read_unaligned(buf.as_ptr() as *const RAWINPUT);
    if raw.header.dwType != RIM_TYPEMOUSE {
        return;
    }

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
            common::process_event(&EventType::ButtonPress(Button::Unknown(data as u8)), cb);
        }
        if flags & RI_MOUSE_BUTTON_4_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Unknown(data as u8)), cb);
        }
        if flags & RI_MOUSE_BUTTON_5_DOWN != 0 {
            common::process_event(&EventType::ButtonPress(Button::Unknown(data as u8)), cb);
        }
        if flags & RI_MOUSE_BUTTON_5_UP != 0 {
            common::process_event(&EventType::ButtonRelease(Button::Unknown(data as u8)), cb);
        }
        if flags & RI_MOUSE_WHEEL != 0 {
            let delta = data as i16;
            common::process_event(&EventType::Wheel { delta_x: 0, delta_y: (delta / 120) as i64 }, cb);
        }
        if flags & RI_MOUSE_HWHEEL != 0 {
            let delta = data as i16;
            common::process_event(&EventType::Wheel { delta_x: (delta / 120) as i64, delta_y: 0 }, cb);
        }
    }
}

pub fn start(data: Arc<RwLock<MonitorData>>, change_tx: watch::Sender<()>, client_count: Arc<AtomicUsize>) {
    std::thread::spawn(move || {
        unsafe {
            CB = Some(common::CallbackData { data, change_tx, client_count });

            let class_name = b"KeyMouseMonWndClass\0";
            let hinst = GetModuleHandleA(null_mut());
            let wc = WNDCLASSA {
                style: 0,
                lpfnWndProc: Some(DefWindowProcA),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinst,
                hIcon: 0,
                hCursor: 0,
                hbrBackground: 0,
                lpszMenuName: null_mut(),
                lpszClassName: class_name.as_ptr() as _,
            };
            if RegisterClassA(&wc) == 0 {
                eprintln!("Failed to register window class");
                return;
            }

            let hwnd = CreateWindowExA(
                0,
                class_name.as_ptr() as _,
                null_mut(),
                0,
                0, 0, 0, 0,
                HWND_MESSAGE,
                0,
                hinst,
                null_mut(),
            );
            if hwnd == 0 {
                eprintln!("Failed to create message-only window");
                return;
            }

            let rid = RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: RIDEV_INPUTSINK | RIDEV_NOLEGACY,
                hwndTarget: hwnd,
            };
            if RegisterRawInputDevices(&rid, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32) == 0 {
                eprintln!("Failed to register raw input device");
                return;
            }

            KEYBOARD_HOOK = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook), 0, 0);
            if KEYBOARD_HOOK == 0 {
                eprintln!("Failed to set keyboard hook");
                return;
            }

            let mut msg = std::mem::zeroed();
            while GetMessageA(&mut msg, 0 as HWND, 0, 0) != 0 {
                if msg.message == WM_INPUT {
                    process_raw_input(msg.lParam);
                } else {
                    TranslateMessage(&msg);
                    DispatchMessageA(&msg);
                }
            }
        }
    });

    println!("Raw Input listener started.");
}
