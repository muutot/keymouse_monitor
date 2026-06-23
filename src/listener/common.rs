use std::borrow::Cow;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

use parking_lot::RwLock;
use rdev::EventType;
use tokio::sync::watch;

use crate::{data::MonitorData, maps};

pub struct CallbackData {
    pub data: Arc<RwLock<MonitorData>>,
    pub change_tx: watch::Sender<()>,
    pub client_count: Arc<AtomicUsize>,
}

pub fn process_event(event_type: &EventType, cb: &CallbackData) {
    let key_name: Option<Cow<'static, str>> = match event_type {
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
        cb.data.write().increase_count(name);
        if cb.client_count.load(Ordering::Relaxed) > 0 {
            cb.change_tx.send_modify(|_| ());
        }
    }
}

