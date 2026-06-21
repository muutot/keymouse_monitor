use std::sync::Arc;
use std::sync::Mutex;

use rdev::Event;
use rdev::EventType;

use crate::data::MonitorData;
use crate::database::Database;
use crate::maps;

pub fn start(
    data: Arc<Mutex<MonitorData>>,
    db: Arc<Mutex<Database>>,
    save_threshold: u64,
) {
    std::thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event: Event| {
            let key_name = match &event.event_type {
                EventType::KeyRelease(key) => maps::key_to_string(key),
                EventType::ButtonPress(button) => maps::button_to_string(button),
                EventType::Wheel { delta_x, delta_y } => {
                    if *delta_y > 0 {
                        Some("mouse_scroll_up")
                    } else if *delta_y < 0 {
                        Some("mouse_scroll_down")
                    } else if *delta_x > 0 {
                        Some("mouse_scroll_right")
                    } else if *delta_x < 0 {
                        Some("mouse_scroll_left")
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(name) = key_name {
                let mut guard = data.lock().unwrap();
                guard.increase_count(name);
                if guard.total_since_save >= save_threshold {
                    guard.save_to_db(&db.lock().unwrap());
                }
            }
        }) {
            eprintln!("Listener error: {:?}", e);
        }
    });

    println!("Keyboard and mouse listeners started.");
}
