#![no_std]

mod runtime;
pub mod time_util;
pub mod watcher;

#[cfg(feature = "gba")]
pub mod gba;

pub use self::runtime::*;
pub use time;
