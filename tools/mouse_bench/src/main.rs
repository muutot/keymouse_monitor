use std::io::Write;
use std::mem;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{FILETIME, GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, GetThreadTimes};
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const BENCH_SECS: u64 = 6;
const POLL_MS: u32 = 2;

static AUTO_ENABLED: AtomicBool = AtomicBool::new(false);
static STIM_HZ: AtomicU32 = AtomicU32::new(100);
static mut STIM_HWND: isize = 0;

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

unsafe fn send_mouse_event() {
    let mut input: INPUT = mem::zeroed();
    input.r#type = INPUT_MOUSE;
    input.Anonymous.mi = MOUSEINPUT {
        dx: 1,
        dy: 0,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MOVE,
        time: 0,
        dwExtraInfo: 0,
    };
    SendInput(1, &input, size_of::<INPUT>() as i32);
    let hwnd = STIM_HWND as HWND;
    if !hwnd.is_null() {
        PostMessageA(hwnd, WM_MOUSEMOVE, 0, 0);
    }
}

fn msg_loop(
    process_msg: unsafe fn(&MSG),
) -> Duration {
    unsafe {
        // Drain stale WM_QUIT from previous benc h runs
        let mut msg = mem::zeroed();
        while PeekMessageA(&mut msg, null_mut(), WM_QUIT, WM_QUIT, PM_REMOVE) != 0 {}

        let wall_start = Instant::now();
        let cpu_start = thread_cpu_time();
        let stim_interval = Duration::from_nanos(1_000_000_000 / STIM_HZ.load(Ordering::Relaxed).max(1) as u64);
        let mut last_stim = Instant::now();
        loop {
            while PeekMessageA(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return thread_cpu_time().checked_sub(cpu_start).unwrap_or(Duration::ZERO);
                }
                process_msg(&msg);
            }
            if wall_start.elapsed() >= Duration::from_secs(BENCH_SECS) {
                break;
            }
            if AUTO_ENABLED.load(Ordering::Relaxed) && last_stim.elapsed() >= stim_interval {
                send_mouse_event();
                last_stim = Instant::now();
            }
            // Yield to OS — avoids busy-wait while still checking for messages frequently
            // The hook / raw input callbacks are triggered inside PeekMessageA
            windows_sys::Win32::System::Threading::Sleep(POLL_MS);
        }
        thread_cpu_time().checked_sub(cpu_start).unwrap_or(Duration::ZERO)
    }
}

// ── Parallel message loop (no internal stimulator, uses stop flag) ────────

fn msg_loop_par(
    stop: &AtomicBool,
    process_msg: unsafe fn(&MSG),
) -> Duration {
    unsafe {
        let mut msg = mem::zeroed();
        while PeekMessageA(&mut msg, null_mut(), WM_QUIT, WM_QUIT, PM_REMOVE) != 0 {}
        let cpu_start = thread_cpu_time();
        loop {
            while PeekMessageA(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return thread_cpu_time().checked_sub(cpu_start).unwrap_or(Duration::ZERO);
                }
                process_msg(&msg);
            }
            if stop.load(Ordering::Relaxed) {
                break;
            }
            windows_sys::Win32::System::Threading::Sleep(POLL_MS);
        }
        thread_cpu_time().checked_sub(cpu_start).unwrap_or(Duration::ZERO)
    }
}

// ── Approach 1: WH_MOUSE_LL hook ──────────────────────────────────────────

static mut BENCH_HOOK: HHOOK = null_mut();

unsafe extern "system" fn hook_cb(_code: i32, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if _code >= 0 && wparam as u32 == WM_MOUSEMOVE {
        return CallNextHookEx(null_mut(), _code, wparam, _lparam);
    }
    CallNextHookEx(null_mut(), _code, wparam, _lparam)
}

unsafe fn process_noop(_msg: &MSG) {}

fn bench_wh_mouse_ll() -> Duration {
    unsafe {
        BENCH_HOOK = SetWindowsHookExA(WH_MOUSE_LL, Some(hook_cb), null_mut(), 0);
        if BENCH_HOOK.is_null() {
            eprintln!("  FAILED to set WH_MOUSE_LL hook");
            return Duration::ZERO;
        }
        let cpu = msg_loop(process_noop);
        UnhookWindowsHookEx(BENCH_HOOK);
        cpu
    }
}



// ── Approach 2: Raw Input via DispatchMessage → wnd_proc ──────────────────

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_INPUT {
        if let Some(raw) = keymouse_rawinput::read_raw_input(lparam) {
            if raw.header.dwType == RIM_TYPEMOUSE {
                let _ = raw.data.mouse.Anonymous.Anonymous.usButtonFlags;
            }
        }
        return 0;
    }
    DefWindowProcA(hwnd, msg, wparam, lparam)
}

fn bench_rawinput_wndproc() -> Duration {
    unsafe {
        let hwnd = match keymouse_rawinput::create_message_window(b"BenchWndProc\0", Some(wnd_proc)) {
            Ok(h) => h,
            Err(e) => { eprintln!("  FAILED: {}", e); return Duration::ZERO; }
        };
        if !keymouse_rawinput::register_raw_input_device(hwnd, 0x01, 0x02, RIDEV_INPUTSINK | RIDEV_NOLEGACY) {
            eprintln!("  FAILED register raw input"); return Duration::ZERO;
        }

        STIM_HWND = hwnd as isize;
        let cpu = msg_loop(|msg| {
            TranslateMessage(msg);
            DispatchMessageA(msg);
        });
        STIM_HWND = 0;
        let _ = PostThreadMessageA(GetCurrentThreadId(), WM_QUIT, 0, 0);
        cpu
    }
}

// ── Approach 3: Raw Input directly in message loop ────────────────────────

unsafe fn process_raw_input(lparam: LPARAM) {
    if let Some(raw) = keymouse_rawinput::read_raw_input(lparam) {
        if raw.header.dwType == RIM_TYPEMOUSE {
            let _ = raw.data.mouse.Anonymous.Anonymous.usButtonFlags;
        }
    }
}

fn bench_rawinput_direct() -> Duration {
    unsafe {
        let hwnd = match keymouse_rawinput::create_message_window(b"BenchDirect\0", Some(DefWindowProcA)) {
            Ok(h) => h,
            Err(e) => { eprintln!("  FAILED: {}", e); return Duration::ZERO; }
        };
        if !keymouse_rawinput::register_raw_input_device(hwnd, 0x01, 0x02, RIDEV_INPUTSINK | RIDEV_NOLEGACY) {
            eprintln!("  FAILED register raw input"); return Duration::ZERO;
        }

        STIM_HWND = hwnd as isize;
        let cpu = msg_loop(|msg| {
            if msg.message == WM_INPUT {
                process_raw_input(msg.lParam);
            } else {
                TranslateMessage(msg);
                DispatchMessageA(msg);
            }
        });
        STIM_HWND = 0;
        let _ = PostThreadMessageA(GetCurrentThreadId(), WM_QUIT, 0, 0);
        cpu
    }
}

// ── Runner ────────────────────────────────────────────────────────────────

fn run_bench(name: &str, f: fn() -> Duration, auto: bool, hz: u32) -> BenchResult {
    if auto {
        print!("  {:<40}  (auto @ {} Hz)", name, hz);
    } else {
        print!("  {:<40}", name);
    }

    AUTO_ENABLED.store(auto, Ordering::Relaxed);
    STIM_HZ.store(hz, Ordering::Relaxed);
    let cpu = f();
    AUTO_ENABLED.store(false, Ordering::Relaxed);

    if cpu == Duration::ZERO {
        println!("  FAILED");
    } else {
        let wall = BENCH_SECS as f64;
        println!("  {:>8.3} ms total  ({:>5.1} µs/s)", cpu.as_secs_f64() * 1000.0, cpu.as_secs_f64() * 1_000_000.0 / wall);
    }
    BenchResult { name: name.to_string(), cpu }
}

struct BenchResult {
    name: String,
    cpu: Duration,
}

fn parse_args() -> (bool, u32, bool, Option<usize>, bool) {
    let mut auto = false;
    let mut hz = 100u32;
    let mut json = false;
    let mut only: Option<usize> = None;
    let mut sequential = false;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--auto" | "-a" => auto = true,
            "--json" | "-j" => json = true,
            "--sequential" | "-s" => sequential = true,
            "--hz" => {
                if i + 1 < args.len() {
                    i += 1;
                    hz = args[i].parse().unwrap_or(100);
                }
            }
            s if s.starts_with("--hz=") => {
                hz = s[5..].parse().unwrap_or(100);
            }
            "--only" => {
                if i + 1 < args.len() {
                    i += 1;
                    only = args[i].parse::<usize>().ok().filter(|n| *n >= 1 && *n <= 3);
                }
            }
            s if s.starts_with("--only=") => {
                only = s[7..].parse::<usize>().ok().filter(|n| *n >= 1 && *n <= 3);
            }
            _ => {}
        }
        i += 1;
    }
    (auto, hz, json, only, sequential)
}

// ── Parallel mode ──────────────────────────────────────────────────────────

unsafe fn send_mouse_event_parallel() {
    let mut input: INPUT = mem::zeroed();
    input.r#type = INPUT_MOUSE;
    input.Anonymous.mi = MOUSEINPUT {
        dx: 1, dy: 0, mouseData: 0,
        dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0,
    };
    SendInput(1, &input, mem::size_of::<INPUT>() as i32);
}

unsafe fn parallel_wh_mouse_ll(stop: &AtomicBool) -> Duration {
    BENCH_HOOK = SetWindowsHookExA(WH_MOUSE_LL, Some(hook_cb), null_mut(), 0);
    if BENCH_HOOK.is_null() {
        let _ = writeln!(std::io::stdout(), "  WH_MOUSE_LL: SetWindowsHookExA err=0x{:x}", GetLastError());
        return Duration::ZERO;
    }
    let cpu = msg_loop_par(stop, process_noop);
    UnhookWindowsHookEx(BENCH_HOOK);
    cpu
}

unsafe fn parallel_rawinput_wndproc(stop: &AtomicBool) -> Duration {
    let hwnd = match keymouse_rawinput::create_message_window(b"ParWndProc\0", Some(wnd_proc)) {
        Ok(h) => h,
        Err(e) => {
            let _ = writeln!(std::io::stdout(), "  ParWndProc: {}", e);
            return Duration::ZERO;
        }
    };
    if !keymouse_rawinput::register_raw_input_device(hwnd, 0x01, 0x02, RIDEV_INPUTSINK) {
        let _ = writeln!(std::io::stdout(), "  ParWndProc: RegisterRawInputDevices err=0x{:x}", GetLastError());
        return Duration::ZERO;
    }
    let cpu = msg_loop_par(stop, |msg| {
        TranslateMessage(msg);
        DispatchMessageA(msg);
    });
    let _ = PostThreadMessageA(GetCurrentThreadId(), WM_QUIT, 0, 0);
    cpu
}

unsafe fn parallel_rawinput_direct(stop: &AtomicBool) -> Duration {
    let hwnd = match keymouse_rawinput::create_message_window(b"ParDirect\0", Some(DefWindowProcA)) {
        Ok(h) => h,
        Err(e) => {
            let _ = writeln!(std::io::stdout(), "  ParDirect: {}", e);
            return Duration::ZERO;
        }
    };
    if !keymouse_rawinput::register_raw_input_device(hwnd, 0x01, 0x02, RIDEV_INPUTSINK) {
        let _ = writeln!(std::io::stdout(), "  ParDirect: RegisterRawInputDevices err=0x{:x}", GetLastError());
        return Duration::ZERO;
    }
    let cpu = msg_loop_par(stop, |msg| {
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

fn run_parallel(hz: u32, json: bool, auto: bool) {
    if auto {
        println!("Mouse listener CPU benchmark (parallel, auto @ {} Hz)", hz);
    } else {
        println!("Mouse listener CPU benchmark (parallel, manual input)");
    }
    println!("All 3 approaches run concurrently for {} s\n", BENCH_SECS);

    let stop = Arc::new(AtomicBool::new(false));

    let names = [
        "WH_MOUSE_LL (early return on WM_MOUSEMOVE)",
        "Raw Input via DispatchMessage → wnd_proc",
        "Raw Input direct in msg loop + stack buffer",
    ];

    let handles: Vec<_> = vec![
        parallel_wh_mouse_ll as unsafe fn(&AtomicBool) -> Duration,
        parallel_rawinput_wndproc,
        parallel_rawinput_direct,
    ].into_iter().map(|f| {
        let stop = Arc::clone(&stop);
        thread::spawn(move || unsafe { f(&stop) })
    }).collect();

    // Give threads time to set up
    thread::sleep(Duration::from_millis(200));

    if auto {
        let stim_interval_ns = 1_000_000_000 / hz.max(1) as u64;
        let wall_start = Instant::now();
        let mut stim_count = 0u64;
        loop {
            unsafe { send_mouse_event_parallel(); }
            stim_count += 1;
            let expected = Duration::from_nanos(stim_count * stim_interval_ns);
            let actual = wall_start.elapsed();
            if actual >= Duration::from_secs(BENCH_SECS) {
                break;
            }
            if actual < expected {
                std::thread::sleep(expected - actual);
            }
        }
    } else {
        std::thread::sleep(Duration::from_secs(BENCH_SECS));
    }

    stop.store(true, Ordering::Relaxed);

    let mut results = Vec::new();
    for (i, handle) in handles.into_iter().enumerate() {
        let name = names[i];
        let cpu = handle.join().unwrap_or(Duration::ZERO);
        results.push(BenchResult { name: name.to_string(), cpu });
        if cpu == Duration::ZERO {
            println!("  {:<40}  FAILED", name);
        } else {
            let wall = BENCH_SECS as f64;
            println!("  {:<40}  {:>8.3} ms total  ({:>5.1} µs/s)", name, cpu.as_secs_f64() * 1000.0, cpu.as_secs_f64() * 1_000_000.0 / wall);
        }
    }

    if json {
        println!("{{");
        println!("  \"results\": [");
        for (i, r) in results.iter().enumerate() {
            let comma = if i + 1 < results.len() { "," } else { "" };
            let wall = BENCH_SECS as f64;
            let cpu_ms = r.cpu.as_secs_f64() * 1000.0;
            let cpu_us_per_s = r.cpu.as_secs_f64() * 1_000_000.0 / wall;
            println!("    {{\"name\": \"{}\", \"cpu_ms\": {:.3}, \"cpu_us_per_s\": {:.1}}}{}", r.name, cpu_ms, cpu_us_per_s, comma);
        }
        println!("  ]");
        println!("}}");
    }
}

fn main() {
    let (auto, hz, json, only, sequential) = parse_args();

    if !sequential {
        run_parallel(hz, json, auto);
        return;
    }

    if auto {
        println!("Mouse listener CPU benchmark (sequential, auto @ {} Hz)", hz);
    } else {
        println!("Mouse listener CPU benchmark (sequential, manual)");
    }
    println!("Test duration: {} s per approach\n", BENCH_SECS);

    let approaches: [(&str, fn() -> Duration); 3] = [
        ("WH_MOUSE_LL (early return on WM_MOUSEMOVE)", bench_wh_mouse_ll),
        ("Raw Input via DispatchMessage → wnd_proc", bench_rawinput_wndproc),
        ("Raw Input direct in msg loop + stack buffer", bench_rawinput_direct),
    ];

    let mut results = Vec::new();

    for (i, (name, f)) in approaches.iter().enumerate() {
        if let Some(only_n) = only {
            if i + 1 != only_n { continue; }
        }
        println!("[{}/{}] {}  ({} @ {} Hz)", i + 1, approaches.len(), name, if auto { "auto" } else { "manual" }, hz);
        let r = run_bench(name, *f, auto, hz);
        results.push(r);
        println!();
    }

    if json {
        println!("{{");
        println!("  \"results\": [");
        for (i, r) in results.iter().enumerate() {
            let comma = if i + 1 < results.len() { "," } else { "" };
            let wall = BENCH_SECS as f64;
            let cpu_ms = r.cpu.as_secs_f64() * 1000.0;
            let cpu_us_per_s = r.cpu.as_secs_f64() * 1_000_000.0 / wall;
            println!("    {{\"name\": \"{}\", \"cpu_ms\": {:.3}, \"cpu_us_per_s\": {:.1}}}{}", r.name, cpu_ms, cpu_us_per_s, comma);
        }
        println!("  ]");
        println!("}}");
    } else {
        println!("{:─^60}", "");
        println!("Lower CPU time = better (less per-mouse-move overhead).");
    }
}
