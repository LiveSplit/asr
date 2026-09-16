//! The walk over a managed runtime's metadata, shared by the runtimes that lay
//! their classes and fields out the same way.
//!
//! What the runtimes genuinely disagree on is behind [`Runtime`]: where the
//! images live, where an image keeps its classes, how a class counts its
//! fields, where its statics sit, and how a live object names its class. Below
//! that, the walk is written once.

mod cursor;
mod pointer;
mod runtime;
mod walk;

pub use cursor::{Assemblies, Classes};
pub use pointer::PointerPath;
pub use runtime::{Il2CppRuntime, MonoRuntime, Runtime};
pub use walk::Walk;

use crate::{string::ArrayCString, Address, PointerSize, Process};

/// The offsets the shared walk reads, copied out of whichever runtime's own
/// offsets built it, so the walk reads plain numbers without knowing whose
/// sections they came from.
pub struct WalkOffsets {
    pub assembly: AssemblyOffsets,
    pub class: ClassOffsets,
    pub field: FieldOffsets,
}

/// Where an assembly keeps its image, and where its name is: on the assembly
/// itself, or through the image, whichever the offsets carry.
pub struct AssemblyOffsets {
    pub name_in_image: Option<u16>,
    pub name_in_assembly: Option<u16>,
    pub image: u16,
}

/// Where a class keeps its names, its parent, its field array, and, when it
/// was measured, the class it is nested in.
pub struct ClassOffsets {
    pub name: u16,
    pub namespace: u16,
    pub parent: u16,
    pub declaring: Option<u16>,
    pub fields: u16,
}

/// Where a field entry keeps its name and offset, and the size of one entry in
/// a class's field array, whatever each runtime's own offsets call it.
pub struct FieldOffsets {
    pub name: u16,
    pub offset: u16,
    pub stride: u16,
}

/// The names a walk stops climbing at when either answers, which is engine
/// policy rather than anything the runtime says: the engine's own classes
/// carry fields a game never declares.
pub struct ClimbStop {
    pub class: &'static str,
    pub namespace: &'static str,
}

impl ClimbStop {
    /// Unity's own base classes.
    pub const UNITY: Self = Self {
        class: "Object",
        namespace: "UnityEngine",
    };
}

/// A class, named by where the runtime keeps it.
#[derive(Copy, Clone)]
pub struct ClassRef {
    pub address: Address,
}

impl ClassRef {
    pub const fn new(address: Address) -> Self {
        Self { address }
    }
}

/// A field, named by where the runtime keeps it.
#[derive(Copy, Clone)]
pub struct FieldRef {
    pub address: Address,
}

impl FieldRef {
    pub const fn new(address: Address) -> Self {
        Self { address }
    }
}

/// An image, named by where the runtime keeps it.
#[derive(Copy, Clone)]
pub struct ImageRef {
    pub address: Address,
}

impl ImageRef {
    pub const fn new(address: Address) -> Self {
        Self { address }
    }
}

/// The address of a pointer-sized slot in an array of them.
pub fn slot(base: Address, pointer_size: PointerSize, index: u64) -> Address {
    base + (pointer_size as u64).wrapping_mul(index)
}

/// Reads a name the runtime stores behind a pointer.
pub fn read_name<const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    at: Address,
) -> Option<ArrayCString<N>> {
    process
        .read_pointer(at, pointer_size)
        .and_then(|address| process.read(address))
        .ok()
}
