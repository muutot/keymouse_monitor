use std::sync::{Arc, atomic::AtomicUsize};

use parking_lot::RwLock;
use tokio::sync::watch;

use crate::data::MonitorData;

mod common;
mod keyboard;

#[cfg(windows)]
mod native;
#[cfg(windows)]
mod rawinput;
mod rdev;

pub fn start(kind: &str, data: Arc<RwLock<MonitorData>>, change_tx: watch::Sender<()>, client_count: Arc<AtomicUsize>) {
    match kind {
        #[cfg(windows)]
        "native" => native::start(data, change_tx, client_count),
        #[cfg(windows)]
        "rawinput" => rawinput::start(data, change_tx, client_count),
        _ => rdev::start(data, change_tx, client_count),
    }
}
