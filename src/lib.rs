#![no_std]
#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::missing_const_for_fn,
    clippy::undocumented_unsafe_blocks,
    missing_docs,
    rust_2018_idioms
)]

//! Helper crate to write auto splitters for LiveSplit One's auto splitting runtime.
//!
//! # Example
//!
//! ```no_run
//! # use asr::Process;
//! #[no_mangle]
//! pub extern "C" fn update() {
//!     if let Some(process) = Process::attach("Notepad.exe") {
//!         asr::print_message("Hello World!");
//!         if let Ok(address) = process.get_module_address("Notepad.exe") {
//!             if let Ok(value) = process.read::<u32>(address) {
//!                 if value > 0 {
//!                     asr::timer::start();
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```

pub mod primitives;
mod runtime;
#[cfg(feature = "signature")]
pub mod signature;
pub mod string;
pub mod sync;
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
