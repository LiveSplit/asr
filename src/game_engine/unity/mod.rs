//! Support for games using the Unity engine.
//!
//! # Example
//!
//! ```no_run
//! # async fn example(process: asr::Process) {
//! use asr::{
//!     future::retry,
//!     game_engine::unity::il2cpp::{Module, Version},
//!     Address, Address64,
//! };
//!
//! // We first attach to the Mono module. Here we know that the game is using IL2CPP 2020.
//! let module = Module::wait_attach(&process, Version::V2020).await;
//! // We access the .NET DLL that the game code is in.
//! let image = module.wait_get_default_image(&process).await;
//!
//! // We access a class called "Timer" in that DLL.
//! let timer_class = image.wait_get_class(&process, &module, "Timer").await;
//! // We access a static field called "_instance" representing the singleton
//! // instance of the class.
//! let instance = timer_class.wait_get_static_instance(&process, &module, "_instance").await;
//!
//! // Once we have the address of the instance, we want to access one of its
//! // fields, so we get the offset of the "currentTime" field.
//! let current_time_offset = timer_class.wait_get_field(&process, &module, "currentTime").await;
//!
//! // Now we can add it to the address of the instance and read the current time.
//! if let Ok(current_time) = process.read::<f32>(instance + current_time_offset) {
//!    // Use the current time.
//! }
//! # }
//! ```
//! Alternatively you can use the `Class` derive macro to generate the bindings
//! for you. This allows reading the contents of an instance of the class
//! described by the struct from a process. Each field must match the name of
//! the field in the class exactly and needs to be of a type that can be read
//! from a process.
//!
//! ```ignore
//! #[derive(Class)]
//! struct Timer {
//!     currentLevelTime: f32,
//!     timerStopped: bool,
//! }
//! ```
//!
//! This will bind to a .NET class of the following shape:
//!
//! ```csharp
//! class Timer
//! {
//!     float currentLevelTime;
//!     bool timerStopped;
//!     // ...
//! }
//! ```
//!
//! The class can then be bound to the process like so:
//!
//! ```ignore
//! let timer_class = Timer::bind(&process, &module, &image).await;
//! ```
//!
//! Once you have an instance, you can read the instance from the process like
//! so:
//!
//! ```ignore
//! if let Ok(timer) = timer_class.read(&process, timer_instance) {
//!     // Do something with the instance.
//! }
//! ```

// References:
// https://github.com/just-ero/asl-help/tree/4c87822df0125b027d1af75e8e348c485817592d/src/Unity
// https://github.com/Unity-Technologies/mono
// https://github.com/CryZe/lunistice-auto-splitter/blob/b8c01031991783f7b41044099ee69edd54514dba/asr-dotnet/src/lib.rs

pub mod il2cpp;
pub mod mono;

mod scene;
pub use self::scene::*;
