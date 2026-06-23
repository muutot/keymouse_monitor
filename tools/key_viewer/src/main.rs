use rdev::{listen, Event, EventType, Key};

#[cfg(windows)]
mod rawinput;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_rawinput = args.iter().any(|a| a == "--rawinput" || a == "-r");

    if use_rawinput {
        #[cfg(windows)]
        {
            rawinput::start();
            return;
        }
        #[cfg(not(windows))]
        {
            eprintln!("--rawinput is only supported on Windows");
            std::process::exit(1);
        }
    }

    println!("Key Viewer - 按键虚拟键码检测工具");
    println!("按下任意按键查看信息，按 Ctrl+C 退出");
    println!("使用 --rawinput 或 -r 参数启动 Raw Input 模式\n");

    println!(
        "{:<7} {:<30} {:<22} {:<10} 事件名",
        "类型", "rdev Key 枚举值", "映射名称", "VK 码"
    );
    println!("{}", "-".repeat(100));

    if let Err(e) = listen(move |event: Event| {
        if matches!(event.event_type, EventType::MouseMove { .. }) {
            return;
        }
        let (action, key_info, mapped, vk, name) = format_event(&event);
        println!("{:<7} {:<30} {:<22} {:<10} {}", action, key_info, mapped, vk, name);
    }) {
        eprintln!("监听器错误: {:?}", e);
    }
}

fn format_event(event: &Event) -> (String, String, String, String, String) {
    let name = event.name.clone().unwrap_or_default();
    match &event.event_type {
        EventType::KeyPress(key) | EventType::KeyRelease(key) => {
            let action = if matches!(event.event_type, EventType::KeyPress(_)) {
                "按下"
            } else {
                "释放"
            };
            let key_info = format!("{:?}", key);
            let mapped = keymouse_common::maps::key_to_string(key)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| "(无映射)".to_string());
            let vk = if let Key::Unknown(code) = key {
                format!("{}", code)
            } else {
                String::new()
            };
            (action.to_string(), key_info, mapped, vk, name)
        }
        EventType::ButtonPress(button) | EventType::ButtonRelease(button) => {
            let action = if matches!(event.event_type, EventType::ButtonPress(_)) {
                "按下"
            } else {
                "释放"
            };
            let key_info = format!("{:?}", button);
            let mapped = keymouse_common::maps::button_to_string(button)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| "(无映射)".to_string());
            let vk = if let rdev::Button::Unknown(code) = button {
                format!("{}", code)
            } else {
                String::new()
            };
            (action.to_string(), key_info, mapped, vk, name)
        }
        EventType::Wheel { delta_x, delta_y } => {
            (String::new(), format!("dx={} dy={}", delta_x, delta_y), String::new(), String::new(), name)
        }
        _ => (String::new(), String::new(), String::new(), String::new(), name),
    }
}
