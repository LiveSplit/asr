//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/object/object.h>

use crate::{
    game_engine::godot::{Ptr, VTable, VariantType},
    Error, Process,
};

use super::ScriptInstance;

#[allow(unused)]
mod offsets {
    // *const VTable
    pub const VTABLE_PTR: u64 = 0x0;
    // *const ObjectGDExtension
    pub const EXTENSION: u64 = 0x8;
    // GDExtensionClassInstancePtr
    pub const EXTENSION_INSTANCE: u64 = 0x10;
    // HashMap<StringName, SignalData>
    pub const SIGNAL_MAP: u64 = 0x18;
    // List<Connection>
    pub const CONNECTIONS: u64 = 0x48;
    // bool
    pub const BLOCK_SIGNALS: u64 = 0x50;
    // i32
    pub const PREDELETE_OK: u64 = 0x54;
    // ObjectID
    pub const INSTANCE_ID: u64 = 0x58;
    // bool
    pub const CAN_TRANSLATE: u64 = 0x60;
    // bool
    pub const EMITTING: u64 = 0x61;
    // *const ScriptInstance
    pub const SCRIPT_INSTANCE: u64 = 0x68;
    // Variant
    pub const SCRIPT: u64 = 0x70;
    // HashMap<StringName, Variant>
    pub const METADATA: u64 = 0x88;
    // HashMap<StringName, Variant*>
    pub const METADATA_PROPERTIES: u64 = 0xb8;
    // *const StringName
    pub const CLASS_NAME_PTR: u64 = 0xe8;
}

/// Information about a property of a script. This is not publicly exposed in
/// Godot.
///
/// Check the [`Ptr<PropertyInfo>`] documentation to see all the methods you can
/// call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct PropertyInfo;

impl Ptr<PropertyInfo> {
    /// Returns the type of the property as a [`VariantType`].
    pub fn get_variant_type(self, process: &Process) -> Result<VariantType, Error> {
        self.read_at_byte_offset(0x0, process)
    }
}

/// Base class for all other classes in the engine.
///
/// [`Object`](https://docs.godotengine.org/en/4.2/classes/class_object.html)
///
/// Check the [`Ptr<Object>`] documentation to see all the methods you can call
/// on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Object;

impl Ptr<Object> {
    /// Returns a pointer to the object's virtual method table.
    pub fn get_vtable(self, process: &Process) -> Result<Ptr<VTable>, Error> {
        self.read_at_byte_offset(offsets::VTABLE_PTR, process)
    }

    /// Returns the object's Script instance, or [`None`] if no script is
    /// attached.
    ///
    /// [`Object.get_script`](https://docs.godotengine.org/en/4.2/classes/class_object.html#class-object-method-get-script)
    pub fn get_script_instance(
        self,
        process: &Process,
    ) -> Result<Option<Ptr<ScriptInstance>>, Error> {
        let ptr: Ptr<ScriptInstance> =
            self.read_at_byte_offset(offsets::SCRIPT_INSTANCE, process)?;
        Ok(if ptr.is_null() { None } else { Some(ptr) })
    }
}
