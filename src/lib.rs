#![no_std]

mod runtime;
#[cfg(feature = "signature")]
pub mod signature;
pub mod string;
pub mod time_util;
pub mod watcher;

#[cfg(feature = "gba")]
pub mod gba;

pub use self::runtime::*;
pub use time;

#[cfg(feature = "derive")]
pub use asr_derive::Settings;

#[cfg(feature = "itoa")]
pub use itoa;

#[cfg(feature = "ryu")]
pub use ryu;

#[cfg(feature = "arrayvec")]
pub use arrayvec;
