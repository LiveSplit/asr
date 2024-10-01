//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/modules/gdscript/gdscript.h>

use crate::{
    game_engine::godot::{HashMap, Ptr, Script, ScriptInstance, StringName, Variant, Vector},
    Error, Process,
};

#[allow(unused)]
mod offsets {
    pub mod script_instance {
        // ObjectId
        pub const OWNER_ID: u64 = 0x8;
        // *const Object
        pub const OWNER: u64 = 0x10;
        // Ref<GDScript>
        pub const SCRIPT: u64 = 0x18;
        // Vector<Variant>
        pub const MEMBERS: u64 = 0x20;
    }

    pub mod script {
        // bool
        pub const TOOL: u64 = 0x178;
        // bool
        pub const VALID: u64 = 0x179;
        // bool
        pub const RELOADING: u64 = 0x17A;
        // Ref<GDScriptNativeClass>
        pub const NATIVE: u64 = 0x180;
        // Ref<GDScript>
        pub const BASE: u64 = 0x188;
        // *const GDScript
        pub const BASE_PTR: u64 = 0x190;
        // *const GDScript
        pub const OWNER_PTR: u64 = 0x198;
        // HashMap<StringName, MemberInfo>
        pub const MEMBER_INDICES: u64 = 0x1A0;
    }

    pub mod member_info {
        // i32
        pub const INDEX: u64 = 0x0;
        // StringName
        pub const SETTER: u64 = 0x8;
        // StringName
        pub const GETTER: u64 = 0x10;
        // GDScriptDataType
        pub const DATA_TYPE: u64 = 0x18;
    }
}

/// A script implemented in the GDScript programming language.
///
/// [`GDScript`](https://docs.godotengine.org/en/4.2/classes/class_gdscript.html)
///
/// Check the [`Ptr<GDScript>`] documentation to see all the methods you can
/// call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct GDScript;
extends!(GDScript: Script);

impl Ptr<GDScript> {
    /// Returns a [`HashMap`] that maps the name of each member to a
    /// [`MemberInfo`] object. This object contains information about the
    /// member, such as the index it occupies in the `members` array of a
    /// [`GDScriptInstance`]. This can then be used to read the actual values of
    /// the members, by indexing into the `members` array returned by
    /// [`Ptr<GDScriptInstance>::get_members`].
    pub fn get_member_indices(self) -> Ptr<HashMap<StringName, MemberInfo>> {
        Ptr::new(self.addr() + offsets::script::MEMBER_INDICES)
    }
}

/// An instance of a script implemented in the GDScript programming language.
/// This is not publicly exposed in Godot.
///
/// Check the [`Ptr<GDScriptInstance>`] documentation to see all the methods you
/// can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct GDScriptInstance;
extends!(GDScriptInstance: ScriptInstance);

impl Ptr<GDScriptInstance> {
    /// Returns the [`GDScript`] that this instance is an instance of. This can
    /// be used to query information about the script, such as the names of its
    /// members and their indices.
    pub fn get_script(self, process: &Process) -> Result<Ptr<GDScript>, Error> {
        self.read_at_byte_offset(offsets::script_instance::SCRIPT, process)
    }

    /// Returns the values of all the members of this script instance. To figure
    /// out the index of a member, use [`Ptr<GDScript>::get_member_indices`].
    pub fn get_members(self, process: &Process) -> Result<Vector<Variant>, Error> {
        self.read_at_byte_offset(offsets::script_instance::MEMBERS, process)
    }
}

/// Information about a member of a script implemented in the GDScript
/// programming language. This is not publicly exposed in Godot.
///
/// Check the [`Ptr<MemberInfo>`] documentation to see all the methods you can
/// call on it.
pub struct MemberInfo;

impl Ptr<MemberInfo> {
    /// Returns the index of the member in the `members` array of a
    /// [`GDScriptInstance`]. This can then be used to read the actual values of
    /// the members, by indexing into the `members` array returned by
    /// [`Ptr<GDScriptInstance>::get_members`].
    pub fn get_index(self, process: &Process) -> Result<i32, Error> {
        self.read_at_byte_offset(offsets::member_info::INDEX, process)
    }
}
