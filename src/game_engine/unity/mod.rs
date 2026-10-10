//! Support for games using the Unity engine.
//!
//! # Example
//!
//! ```no_run
//! # async fn example(process: asr::Process) {
//! use asr::{
//!     future::retry,
//!     game_engine::unity::il2cpp::Module,
//!     Address, Address64,
//! };
//!
//! // We first attach to the IL2CPP module.
//! let module = Module::wait_attach_auto_detect(&process).await;
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
//! let current_time_offset = timer_class.wait_get_field_offset(&process, &module, "currentTime").await;
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
//! the field in the class exactly (or alternatively renamed with the `#[rename
//! = "..."]` attribute) and needs to be of a type that can be read from a
//! process. Fields can be marked as static with the `#[static_field]`
//! attribute.
//!
//! ```ignore
//! #[derive(Class)]
//! struct Timer {
//!     #[rename = "currentLevelTime"]
//!     level_time: f32,
//!     #[static_field]
//!     foo: bool,
//! }
//! ```
//!
//! This will bind to a .NET class of the following shape:
//!
//! ```csharp
//! class Timer
//! {
//!     float currentLevelTime;
//!     static bool foo;
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
//!
//! If only static fields are present, the `read` method does not take an
//! instance argument.

// References:
// https://github.com/just-ero/asl-help/tree/4c87822df0125b027d1af75e8e348c485817592d/src/Unity
// https://github.com/Unity-Technologies/mono
// https://github.com/CryZe/lunistice-auto-splitter/blob/b8c01031991783f7b41044099ee69edd54514dba/asr-dotnet/src/lib.rs

pub mod il2cpp;
mod managed;
pub use managed::{DictionaryOffsets, HashSetOffsets, ListOffsets, ManagedString};
pub mod mono;
pub mod scene_manager;

use crate::{signature::Signature, Address, Process};

const CSTR: usize = 128;

#[derive(Copy, Clone, PartialEq, Hash, Debug)]
#[non_exhaustive]
enum BinaryFormat {
    PE,
    ELF,
    MachO,
}

/// Finds the Unity version string in a player, `2021.3.11f1` for
/// example, and returns its three numbers. The fourth part of a file
/// version has no equal in the string, so it is 0. A NUL heads the
/// string, and only a run of the shape `major.minor.patch` followed by
/// the letter of the release counts, so a date or a build number in the
/// same module is passed over. A player can hold older version strings
/// too, so this returns the newest one.
fn version_string(process: &Process, module: (Address, u64)) -> Option<(u16, u16, u16, u16)> {
    const FOUR_DIGITS: Signature<6> = Signature::new("00 3? 3? 3? 3? 2E");
    const ONE_DIGIT: Signature<4> = Signature::new("00 3? 2E 3?");

    FOUR_DIGITS
        .scan_iter(process, module)
        .chain(ONE_DIGIT.scan_iter(process, module))
        .filter_map(|at| {
            let text = process.read::<[u8; 16]>(at + 1).ok()?;
            parse_version(&text)
        })
        .max()
}

/// Parses `major.minor.patch` followed by a release letter out of the
/// head of `text`.
fn parse_version(text: &[u8]) -> Option<(u16, u16, u16, u16)> {
    // Reads a number of up to four digits and returns what follows it.
    fn number(text: &[u8]) -> Option<(u16, &[u8])> {
        let digits = text.iter().take_while(|byte| byte.is_ascii_digit()).count();
        if digits == 0 || digits > 4 {
            return None;
        }
        let value = text[..digits]
            .iter()
            .fold(0_u16, |value, byte| value * 10 + (byte - b'0') as u16);
        Some((value, &text[digits..]))
    }

    let (major, rest) = number(text)?;
    let (minor, rest) = number(rest.strip_prefix(b".")?)?;
    let (patch, rest) = number(rest.strip_prefix(b".")?)?;
    matches!(rest.first(), Some(b'a' | b'b' | b'f' | b'p' | b'x'))
        .then_some((major, minor, patch, 0))
}

/// If the field name is an auto-property, extract the backing field name.
fn get_backing_name(name: &str) -> Option<&str> {
    let start = name.find('<')?;
    let end = name[start + 1..].find('>')?;
    Some(&name[start + 1..start + 1 + end])
}
