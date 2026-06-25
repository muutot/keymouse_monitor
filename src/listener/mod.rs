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

pub enum ListenerKind {
    #[cfg(windows)]
    Native,
    #[cfg(windows)]
    RawInput,
    Rdev,
}

impl ListenerKind {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            #[cfg(windows)]
            "native" => Self::Native,
            #[cfg(windows)]
            "rawinput" => Self::RawInput,
            _ => Self::Rdev,
        }
    }
}

pub fn start(kind: ListenerKind, data: Arc<RwLock<MonitorData>>, change_tx: watch::Sender<()>, client_count: Arc<AtomicUsize>) {
    match kind {
        #[cfg(windows)]
        ListenerKind::Native => native::start(data, change_tx, client_count),
        #[cfg(windows)]
        ListenerKind::RawInput => rawinput::start(data, change_tx, client_count),
        ListenerKind::Rdev => rdev::start(data, change_tx, client_count),
    }
}
