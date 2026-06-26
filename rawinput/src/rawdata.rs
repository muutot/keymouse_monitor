use std::ptr::{null_mut, read_unaligned};

use windows_sys::Win32::Foundation::LPARAM;
use windows_sys::Win32::UI::Input::*;

/// Reads raw input data from a `WM_INPUT` message's `lParam`.
///
/// Uses the double-call pattern (query size, then read data) with a
/// 256-byte stack buffer — sufficient for standard mouse / keyboard HID reports.
/// Returns `None` if the data is too large or the call fails.
///
/// # Safety
///
/// `lparam` must be a valid `LPARAM` from a `WM_INPUT` message.
/// The caller must ensure the underlying `RAWINPUT` structure is
/// still valid for the duration of this call.
pub unsafe fn read_raw_input(lparam: LPARAM) -> Option<RAWINPUT> {
    let mut size: u32 = 0;
    GetRawInputData(
        lparam as _,
        RID_INPUT,
        null_mut(),
        &mut size,
        size_of::<RAWINPUTHEADER>() as u32,
    );
    if size == 0 || size > 256 {
        return None;
    }

    let mut buf = [0u8; 256];
    let written = GetRawInputData(
        lparam as _,
        RID_INPUT,
        buf.as_mut_ptr() as _,
        &mut size,
        size_of::<RAWINPUTHEADER>() as u32,
    );
    if written == u32::MAX {
        return None;
    }

    Some(read_unaligned(buf.as_ptr() as *const RAWINPUT))
}
