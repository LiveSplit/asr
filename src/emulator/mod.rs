//! Support for attaching to various emulators.

#[cfg(feature = "gba")]
pub mod gba;
#[cfg(feature = "gcn")]
pub mod gcn;
#[cfg(feature = "genesis")]
pub mod genesis;
#[cfg(feature = "ps1")]
pub mod ps1;
