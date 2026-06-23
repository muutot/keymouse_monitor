use std::ptr::null_mut;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub fn create_message_window(class_name: &[u8], wnd_proc: WNDPROC) -> Result<HWND, String> {
    unsafe {
        let hinst = GetModuleHandleA(null_mut());
        if hinst.is_null() {
            return Err("GetModuleHandleA failed".into());
        }

        let wc = WNDCLASSA {
            style: 0,
            lpfnWndProc: wnd_proc,
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
            return Err("Failed to register window class".into());
        }

        let hwnd = CreateWindowExA(
            0,
            class_name.as_ptr() as _,
            null_mut(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            hinst,
            null_mut(),
        );
        if hwnd.is_null() {
            return Err("Failed to create message-only window".into());
        }

        Ok(hwnd)
    }
}
