#![no_std]

mod runtime;
pub mod settings;
#[cfg(feature = "signature")]
pub mod signature;
pub mod time_util;
pub mod watcher;

#[cfg(feature = "gba")]
pub mod gba;

pub use self::runtime::*;
pub use time;

#[cfg(feature = "derive")]
pub use asr_derive::Settings;
