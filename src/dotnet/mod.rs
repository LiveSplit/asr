//! Support for games using the Unity engine.
//! 
//! So far, this module supports games built with IL2CPP 2020

mod il2cpp_2020;
pub use il2cpp_2020::MonoModule as IL2CPP_2020;

mod il2cpp_2019;
pub use il2cpp_2019::MonoModule as IL2CPP_2019;

mod il2cpp_base;
pub use il2cpp_base::MonoModule as IL2CPP_BASE;

mod scenemanager;
pub use scenemanager::SceneManager;

use crate::signature::Signature;

// Consts used for all 64_bit IL2CPP games
const ASSEMBLIES_TRG_SIG: Signature<12> = Signature::new("48 FF C5 80 3C ?? 00 75 ?? 48 8B 1D");
const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> = Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");