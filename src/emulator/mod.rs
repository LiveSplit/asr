//! Support for attaching to various emulators.

#[cfg(feature = "gba")]
pub mod gba;
#[cfg(feature = "gcn")]
pub mod gcn;
#[cfg(feature = "genesis")]
pub mod genesis;
#[cfg(feature = "ps1")]
pub mod ps1;
#[cfg(feature = "ps2")]
pub mod ps2;
#[cfg(feature = "wii")]
pub mod wii;
