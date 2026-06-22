use std::mem;
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{FILETIME, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, GetThreadTimes};
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const BENCH_SECS: u64 = 6;
const POLL_MS: u32 = 2;

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    (ft.dwHighDateTime as u64) << 32 | ft.dwLowDateTime as u64
}

fn thread_cpu_time() -> Duration {
    unsafe {
        let ft = || FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut c, mut e, mut k, mut u) = (ft(), ft(), ft(), ft());
        if GetThreadTimes(GetCurrentThread(), &mut c, &mut e, &mut k, &mut u) != 0 {
            Duration::from_nanos((filetime_to_u64(&k) + filetime_to_u64(&u)) * 100)
        } else {
            Duration::ZERO
        }
    }
}

fn msg_loop(
    process_msg: unsafe fn(&MSG),
) -> Duration {
    unsafe {
        let wall_start = Instant::now();
        let cpu_start = thread_cpu_time();
        let mut msg = mem::zeroed();
        loop {
            while PeekMessageA(&mut msg, 0 as HWND, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return thread_cpu_time().checked_sub(cpu_start).unwrap_or(Duration::ZERO);
                }
                process_msg(&msg);
            }
            if wall_start.elapsed() >= Duration::from_secs(BENCH_SECS) {
                break;
            }
            // Yield to OS — avoids busy-wait while still checking for messages frequently
            // The hook / raw input callbacks are triggered inside PeekMessageA
            windows_sys::Win32::System::Threading::Sleep(POLL_MS);
        }
        thread_cpu_time().checked_sub(cpu_start).unwrap_or(Duration::ZERO)
    }
}

// ── Approach 1: WH_MOUSE_LL hook ──────────────────────────────────────────

static mut BENCH_HOOK: isize = 0;

unsafe extern "system" fn hook_cb(_code: i32, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if _code >= 0 && wparam as u32 == WM_MOUSEMOVE {
        return CallNextHookEx(0, _code, wparam, _lparam);
    }
    CallNextHookEx(0, _code, wparam, _lparam)
}

unsafe fn process_noop(_msg: &MSG) {}

fn bench_wh_mouse_ll() -> Duration {
    unsafe {
        BENCH_HOOK = SetWindowsHookExA(WH_MOUSE_LL, Some(hook_cb), 0, 0);
        if BENCH_HOOK == 0 {
            eprintln!("  FAILED to set WH_MOUSE_LL hook");
            return Duration::ZERO;
        }
        let cpu = msg_loop(process_noop);
        let _ = PostThreadMessageA(GetCurrentThreadId(), WM_QUIT, 0, 0);
        cpu
    }
}

// ── Helper: hidden message-only window ────────────────────────────────────

unsafe fn create_hidden_window(class_name: &[u8]) -> Option<HWND> {
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
        return None;
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
    if hwnd == 0 { None } else { Some(hwnd) }
}

unsafe fn register_raw_input(hwnd: HWND) -> bool {
    let rid = RAWINPUTDEVICE {
        usUsagePage: 0x01,
        usUsage: 0x02,
        dwFlags: RIDEV_INPUTSINK | RIDEV_NOLEGACY,
        hwndTarget: hwnd,
    };
    RegisterRawInputDevices(&rid, 1, mem::size_of::<RAWINPUTDEVICE>() as u32) != 0
}

// ── Approach 2: Raw Input via DispatchMessage → wnd_proc ──────────────────

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_INPUT {
        let mut size: u32 = 0;
        GetRawInputData(lparam as _, RID_INPUT, null_mut(), &mut size, mem::size_of::<RAWINPUTHEADER>() as u32);
        if size > 0 {
            let mut buf = vec![0u8; size as usize];
            let written = GetRawInputData(
                lparam as _, RID_INPUT,
                buf.as_mut_ptr() as *mut _, &mut size,
                mem::size_of::<RAWINPUTHEADER>() as u32,
            );
            if written != u32::MAX {
                let raw = &*(buf.as_ptr() as *const RAWINPUT);
                if raw.header.dwType == RIM_TYPEMOUSE {
                    let _ = raw.data.mouse.Anonymous.Anonymous.usButtonFlags;
                }
            }
        }
        return 0;
    }
    DefWindowProcA(hwnd, msg, wparam, lparam)
}

fn bench_rawinput_wndproc() -> Duration {
    unsafe {
        let hwnd = create_hidden_window(b"BenchWndProc\0");
        if hwnd.is_none() { eprintln!("  FAILED create window"); return Duration::ZERO; }
        let hwnd = hwnd.unwrap();

        // Register class with custom proc
        let hinst = GetModuleHandleA(null_mut());
        let wc = WNDCLASSA {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0, cbWndExtra: 0,
            hInstance: hinst, hIcon: 0, hCursor: 0, hbrBackground: 0,
            lpszMenuName: null_mut(),
            lpszClassName: b"BenchWndProc\0".as_ptr() as _,
        };
        let _ = RegisterClassA(&wc);

        if !register_raw_input(hwnd) { eprintln!("  FAILED register raw input"); return Duration::ZERO; }

        let cpu = msg_loop(|msg| {
            TranslateMessage(msg);
            DispatchMessageA(msg);
        });
        let _ = PostThreadMessageA(GetCurrentThreadId(), WM_QUIT, 0, 0);
        cpu
    }
}

// ── Approach 3: Raw Input directly in message loop ────────────────────────

unsafe fn process_raw_input(lparam: LPARAM) {
    const BUF_SZ: usize = 64;
    let mut buf = [0u8; BUF_SZ];
    let mut size = BUF_SZ as u32;
    let written = GetRawInputData(
        lparam as _, RID_INPUT,
        buf.as_mut_ptr() as *mut _, &mut size,
        mem::size_of::<RAWINPUTHEADER>() as u32,
    );
    if written != u32::MAX && written >= mem::size_of::<RAWINPUTHEADER>() as u32 {
        let raw = &*(buf.as_ptr() as *const RAWINPUT);
        if raw.header.dwType == RIM_TYPEMOUSE {
            let _ = raw.data.mouse.Anonymous.Anonymous.usButtonFlags;
        }
    }
}

fn bench_rawinput_direct() -> Duration {
    unsafe {
        let hwnd = create_hidden_window(b"BenchDirect\0");
        if hwnd.is_none() { eprintln!("  FAILED create window"); return Duration::ZERO; }
        let hwnd = hwnd.unwrap();

        if !register_raw_input(hwnd) { eprintln!("  FAILED register raw input"); return Duration::ZERO; }

        let cpu = msg_loop(|msg| {
            if msg.message == WM_INPUT {
                process_raw_input(msg.lParam);
            } else {
                TranslateMessage(msg);
                DispatchMessageA(msg);
            }
        });
        let _ = PostThreadMessageA(GetCurrentThreadId(), WM_QUIT, 0, 0);
        cpu
    }
}

// ── Runner ────────────────────────────────────────────────────────────────

fn run_bench(name: &str, f: fn() -> Duration) {
    print!("  {:<40}", name);
    let cpu = f();
    if cpu == Duration::ZERO {
        println!("  FAILED");
    } else {
        let wall = BENCH_SECS as f64;
        println!("  {:>8.3} ms total  ({:>5.1} µs/s)", cpu.as_secs_f64() * 1000.0, cpu.as_secs_f64() * 1_000_000.0 / wall);
    }
}

fn main() {
    println!("Mouse listener CPU benchmark");
    println!("Test duration: {} s per approach", BENCH_SECS);
    println!("Move your mouse continuously during each test.");
    println!("Press Enter to start each approach when ready.\n");

    let approaches: [(&str, fn() -> Duration); 3] = [
        ("WH_MOUSE_LL (early return on WM_MOUSEMOVE)", bench_wh_mouse_ll),
        ("Raw Input via DispatchMessage → wnd_proc", bench_rawinput_wndproc),
        ("Raw Input direct in msg loop + stack buffer", bench_rawinput_direct),
    ];

    for (i, (name, f)) in approaches.iter().enumerate() {
        println!("[{}/{}] Approach: {}", i + 1, approaches.len(), name);
        print!("  Press Enter to start... ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        run_bench(name, *f);
        println!();
    }

    println!("{:─^60}", "");
    println!("Lower CPU time = better (less per-mouse-move overhead).");
    println!("The hook callback / WM_INPUT handler is measured via GetThreadTimes");
    println!("on the worker thread. The PeekMessageA + Sleep(2ms) loop is common to all.");
}
