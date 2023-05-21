//! Support for games using the Unity engine.

#[cfg(feature = "mono")]
mod mono;
#[cfg(feature = "mono")]
pub use mono::MonoModule as Mono;
#[cfg(feature = "mono")]
pub use mono::MonoVersion as MonoVersion;

#[cfg(feature = "il2cpp")]
mod il2cpp;
#[cfg(feature = "il2cpp")]
pub use il2cpp::Il2CppModule as Il2Cpp;
#[cfg(feature = "il2cpp")]
pub use il2cpp::Il2CppVersion as Il2CppVersion;

#[cfg(any(feature = "mono", feature = "il2cpp"))]
mod scenemanager;
#[cfg(any(feature = "mono", feature = "il2cpp"))]
pub use scenemanager::SceneManager as SceneManager;