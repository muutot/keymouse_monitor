#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod window;
#[cfg(windows)]
mod device;
#[cfg(windows)]
mod rawdata;

#[cfg(windows)]
pub use window::*;
#[cfg(windows)]
pub use device::*;
#[cfg(windows)]
pub use rawdata::*;
