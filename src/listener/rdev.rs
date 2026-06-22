use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::RwLock;
use rdev::Event;
use rdev::EventType;
use tokio::sync::watch;

use crate::data::MonitorData;
use crate::maps;

pub fn start(data: Arc<RwLock<MonitorData>>, change_tx: watch::Sender<()>, client_count: Arc<AtomicUsize>) {
    std::thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event: Event| {
            if matches!(event.event_type, EventType::MouseMove { .. }) {
                return;
            }

            let key_name: Option<Cow<'static, str>> = match &event.event_type {
                EventType::KeyRelease(key) => maps::key_to_string(key),
                EventType::ButtonPress(button) => maps::button_to_string(button),
                EventType::Wheel { delta_x, delta_y } => {
                    if *delta_y > 0 {
                        Some(Cow::Borrowed("mouse_scroll_up"))
                    } else if *delta_y < 0 {
                        Some(Cow::Borrowed("mouse_scroll_down"))
                    } else if *delta_x > 0 {
                        Some(Cow::Borrowed("scroll_right_dir"))
                    } else if *delta_x < 0 {
                        Some(Cow::Borrowed("scroll_left_dir"))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(ref name) = key_name {
                data.write().increase_count(name);
                if client_count.load(Ordering::Relaxed) > 0 {
                    change_tx.send_modify(|_| ());
                }
            }
        }) {
            eprintln!("rdev listener error: {:?}", e);
        }
    });

    println!("rdev listener started.");
}
