#![cfg(windows)]

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::LPARAM;
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub fn start() {
    unsafe {
        let hwnd = match keymouse_rawinput::create_message_window(
            b"KeyViewerRawInputWndClass\0",
            Some(DefWindowProcA),
        ) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("rawinput: {}", e);
                return;
            }
        };

        for &(page, usage) in &[(0x01, 0x06), (0x01, 0x02)] {
            if !keymouse_rawinput::register_raw_input_device(hwnd, page, usage, RIDEV_INPUTSINK) {
                eprintln!(
                    "rawinput: Failed to register raw input device ({:04x}/{:04x})",
                    page, usage
                );
                return;
            }
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
    let raw = match keymouse_rawinput::read_raw_input(lparam) {
        Some(raw) => raw,
        None => return,
    };
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
    let mapped = keymouse_common::maps::vk_to_string(vk).unwrap_or("(无映射)");
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
                println!("{:<7} {:<30} {:<22} {:<10}", "鼠标", info, $name, data);
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
