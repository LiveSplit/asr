//! Support for games using the Unity engine.

#[cfg(feature = "mono")]
pub mod mono;

#[cfg(feature = "il2cpp")]
pub mod il2cpp;

#[cfg(any(feature = "mono", feature = "il2cpp"))]
pub mod scene;
