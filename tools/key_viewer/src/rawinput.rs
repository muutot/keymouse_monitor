#![cfg(windows)]

use std::ptr::{null_mut, read_unaligned};

use windows_sys::Win32::Foundation::LPARAM;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::maps;

const BUF_SIZE: usize = 128;

pub fn start() {
    unsafe {
        let class_name = b"KeyViewerRawInputWndClass\0";
        let hinst = GetModuleHandleA(null_mut());
        let wc = WNDCLASSA {
            style: 0,
            lpfnWndProc: Some(DefWindowProcA),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
            lpszClassName: class_name.as_ptr() as _,
        };
        if RegisterClassA(&wc) == 0 {
            eprintln!("rawinput: Failed to register window class");
            return;
        }

        let hwnd = CreateWindowExA(
            0,
            class_name.as_ptr() as _,
            null_mut(),
            0,
            0, 0, 0, 0,
            HWND_MESSAGE,
            null_mut(),
            hinst,
            null_mut(),
        );
        if hwnd.is_null() {
            eprintln!("rawinput: Failed to create message-only window");
            return;
        }

        let devices = [
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
        ];
        if RegisterRawInputDevices(devices.as_ptr(), 2, size_of::<RAWINPUTDEVICE>() as u32) == 0 {
            eprintln!("rawinput: Failed to register raw input devices");
            return;
        }

        println!("\nRaw Input 模式已启动，按下按键/鼠标按钮查看信息，按 Ctrl+C 退出\n");
    println!(
        "{:<7} {:<30} {:<22} {:<10} 事件名",
        "类型", "RAW 信息", "映射名称", "代码"
    );
        println!("{}", "-".repeat(100));

        let mut msg = std::mem::zeroed();
        while GetMessageA(&mut msg, null_mut(), 0, 0) != 0 {
            if msg.message == WM_INPUT {
                process_raw_input(msg.lParam);
            } else {
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
        }
    }
}

unsafe fn process_raw_input(lparam: LPARAM) {
    let mut size: u32 = 0;
    GetRawInputData(
        lparam as _,
        RID_INPUT,
        null_mut(),
        &mut size,
        size_of::<RAWINPUTHEADER>() as u32,
    );
    if size == 0 || size > BUF_SIZE as u32 {
        return;
    }

    let mut buf = [0u8; BUF_SIZE];
    let written = GetRawInputData(
        lparam as _,
        RID_INPUT,
        buf.as_mut_ptr() as _,
        &mut size,
        size_of::<RAWINPUTHEADER>() as u32,
    );
    if written == u32::MAX {
        return;
    }

    let raw: RAWINPUT = read_unaligned(buf.as_ptr() as *const RAWINPUT);
    match raw.header.dwType {
        RIM_TYPEKEYBOARD => process_keyboard(&raw.data.keyboard),
        RIM_TYPEMOUSE => process_mouse(&raw.data.mouse),
        _ => {}
    }
}

unsafe fn process_keyboard(kb: &RAWKEYBOARD) {
    let action = if kb.Flags as u32 & RI_KEY_BREAK != 0 {
        "释放"
    } else {
        "按下"
    };
    let vk = kb.VKey as u32;
    let mapped = maps::vk_name(vk).unwrap_or("(无映射)");
    let info = format!("VK={}", vk);
    let name = msg_name(kb.Message);
    println!(
        "{:<7} {:<30} {:<22} {:<10} {}",
        action, info, mapped, vk, name
    );
}

unsafe fn process_mouse(mouse: &RAWMOUSE) {
    let flags = mouse.Anonymous.Anonymous.usButtonFlags as u32;
    if flags == 0 {
        return;
    }
    let data = mouse.Anonymous.Anonymous.usButtonData;

    macro_rules! check_flag {
        ($flag:ident, $action:expr, $name:expr) => {
            if flags & $flag != 0 {
                let info = format!("{} data={}", $action, data);
                println!(
                    "{:<7} {:<30} {:<22} {:<10}",
                    "鼠标", info, $name, data
                );
            }
        };
    }

    check_flag!(RI_MOUSE_LEFT_BUTTON_DOWN, "左键按下", "mouse_left");
    check_flag!(RI_MOUSE_LEFT_BUTTON_UP, "左键释放", "mouse_left");
    check_flag!(RI_MOUSE_RIGHT_BUTTON_DOWN, "右键按下", "mouse_right");
    check_flag!(RI_MOUSE_RIGHT_BUTTON_UP, "右键释放", "mouse_right");
    check_flag!(RI_MOUSE_MIDDLE_BUTTON_DOWN, "中键按下", "mouse_middle");
    check_flag!(RI_MOUSE_MIDDLE_BUTTON_UP, "中键释放", "mouse_middle");
    check_flag!(RI_MOUSE_BUTTON_4_DOWN, "X1按下", "mouse_x1");
    check_flag!(RI_MOUSE_BUTTON_4_UP, "X1释放", "mouse_x1");
    check_flag!(RI_MOUSE_BUTTON_5_DOWN, "X2按下", "mouse_x2");
    check_flag!(RI_MOUSE_BUTTON_5_UP, "X2释放", "mouse_x2");

    if flags & RI_MOUSE_WHEEL != 0 {
        let delta = data as i16;
        let dir = if delta > 0 { "上" } else { "下" };
        println!(
            "{:<7} {:<30} {:<22} {:<10}",
            "滚轮",
            format!("垂直滚动 delta={}", delta),
            format!("scroll_{}", dir),
            delta,
        );
    }
    if flags & RI_MOUSE_HWHEEL != 0 {
        let delta = data as i16;
        let dir = if delta > 0 { "右" } else { "左" };
        println!(
            "{:<7} {:<30} {:<22} {:<10}",
            "滚轮",
            format!("水平滚动 delta={}", delta),
            format!("scroll_{}", dir),
            delta,
        );
    }
}

fn msg_name(msg: u32) -> &'static str {
    match msg {
        WM_KEYDOWN => "WM_KEYDOWN",
        WM_KEYUP => "WM_KEYUP",
        WM_SYSKEYDOWN => "WM_SYSKEYDOWN",
        WM_SYSKEYUP => "WM_SYSKEYUP",
        _ => "WM_UNKNOWN",
    }
}
