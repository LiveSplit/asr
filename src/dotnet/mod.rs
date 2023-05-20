//! Support for games using the Unity engine.

#[cfg(feature = "mono")]
mod mono;
#[cfg(feature = "mono")]
pub use mono::MonoModule as Mono;
#[cfg(feature = "mono")]
pub use mono::MonoVersion;