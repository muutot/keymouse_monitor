use std::mem;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::*;

/// Registers a single raw input device.
///
/// Returns `true` on success, `false` on failure.
pub fn register_raw_input_device(
    hwnd: HWND,
    usage_page: u16,
    usage: u16,
    flags: u32,
) -> bool {
    let rid = RAWINPUTDEVICE {
        usUsagePage: usage_page,
        usUsage: usage,
        dwFlags: flags,
        hwndTarget: hwnd,
    };
    unsafe {
        RegisterRawInputDevices(&rid, 1, mem::size_of::<RAWINPUTDEVICE>() as u32) != 0
    }
}
