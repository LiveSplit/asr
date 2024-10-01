//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/modules/mono/csharp_script.h>

use crate::{
    game_engine::godot::{HashMap, PropertyInfo, Ptr, Script, ScriptInstance, StringName},
    Error, Process,
};

#[allow(unused)]
mod offsets {
    pub mod script_instance {
        // *const Object
        pub const OWNER: u64 = 0x8;
        // bool
        pub const BASE_REF_COUNTED: u64 = 0x10;
        // bool
        pub const REF_DYING: u64 = 0x11;
        // bool
        pub const UNSAFE_REFERENCED: u64 = 0x12;
        // bool
        pub const PREDELETE_NOTIFIED: u64 = 0x13;
        // Ref<CSharpScript>
        pub const SCRIPT: u64 = 0x18;
        // MonoGCHandleData
        pub const GCHANDLE: u64 = 0x20;
    }

    pub mod script {
        use crate::game_engine::godot::{HashSet, SizeInTargetProcess, String, Vector};

        // bool
        pub const TOOL: u64 = 0x178;
        // bool
        pub const GLOBAL_CLASS: u64 = 0x179;
        // bool
        pub const ABSTRACT_CLASS: u64 = 0x17A;
        // bool
        pub const VALID: u64 = 0x17B;
        // bool
        pub const RELOAD_INVALIDATED: u64 = 0x17C;
        // Ref<CSharpScript>
        pub const BASE_SCRIPT: u64 = 0x180;
        // HashSet<*const Object>
        pub const INSTANCES: u64 = 0x188;
        // String
        pub const SOURCE: u64 = (INSTANCES + HashSet::<()>::SIZE).next_multiple_of(8);
        // String
        pub const CLASS_NAME: u64 = (SOURCE + String::<0>::SIZE).next_multiple_of(8);
        // String
        pub const ICON_PATH: u64 = (CLASS_NAME + String::<0>::SIZE).next_multiple_of(8);
        // SelfList<CSharpScript> (4 pointers)
        pub const SCRIPT_LIST: u64 = (ICON_PATH + String::<0>::SIZE).next_multiple_of(8);
        // Dictionary (1 pointer)
        pub const RPC_CONFIG: u64 = (SCRIPT_LIST + 4 * 8).next_multiple_of(8);
        // Vector<EventSignalInfo>
        pub const EVENT_SIGNALS: u64 = (RPC_CONFIG + 8).next_multiple_of(8);
        // Vector<CSharpMethodInfo>
        pub const METHODS: u64 = (EVENT_SIGNALS + Vector::<()>::SIZE).next_multiple_of(8);
        // HashMap<StringName, PropertyInfo>
        pub const MEMBER_INFO: u64 = (METHODS + Vector::<()>::SIZE).next_multiple_of(8);
    }
}

/// A script implemented in the C# programming language, saved with the `.cs`
/// extension (Mono-enabled builds only).
///
/// [`CSharpScript`](https://docs.godotengine.org/en/4.2/classes/class_csharpscript.html)
///
/// Check the [`Ptr<CSharpScript>`] documentation to see all the methods you can
/// call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CSharpScript;
extends!(CSharpScript: Script);

impl Ptr<CSharpScript> {
    /// Returns a [`HashMap`] that maps the name of each member to a
    /// [`PropertyInfo`] object. This object contains information about the
    /// member, such as its type. Notably this is not the type on the C# side,
    /// but a [`VariantType`](crate::game_engine::godot::VariantType). Unlike
    /// with [`GDScript`](crate::game_engine::godot::GDScript), there is
    /// currently no way to figure out where the member is stored in memory.
    pub fn get_member_info(self) -> Ptr<HashMap<StringName, PropertyInfo>> {
        Ptr::new(self.addr() + offsets::script::MEMBER_INFO)
    }
}

/// An instance of a script implemented in the C# programming language. This is
/// not publicly exposed in Godot.
///
/// Check the [`Ptr<CSharpScriptInstance>`] documentation to see all the methods
/// you can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CSharpScriptInstance;
extends!(CSharpScriptInstance: ScriptInstance);

impl Ptr<CSharpScriptInstance> {
    /// Returns the [`CSharpScript`] that this instance is an instance of. This
    /// can be used to query information about the script, such as the names of
    /// its members and their
    /// [`VariantType`](crate::game_engine::godot::VariantType)s.
    pub fn get_script(self, process: &Process) -> Result<Ptr<CSharpScript>, Error> {
        self.read_at_byte_offset(offsets::script_instance::SCRIPT, process)
    }

    /// Returns the [`CSharpGCHandle`], which allows you to access the members of
    /// the script instance.
    pub fn get_gc_handle(self, process: &Process) -> Result<Ptr<CSharpGCHandle>, Error> {
        self.read_at_byte_offset(offsets::script_instance::GCHANDLE, process)
    }
}

/// A handle to a C# object. This is not publicly exposed in Godot.
///
/// Check the [`Ptr<CSharpGCHandle>`] documentation to see all the methods you
/// can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CSharpGCHandle;

impl Ptr<CSharpGCHandle> {
    /// Returns a pointer to the start of the raw data of the instance. This is
    /// where all the members are stored. You can use the `.Net Info`
    /// functionality in Cheat Engine to figure out the offset of a member from
    /// this pointer. Note that the garbage collector can move objects around in
    /// memory, so this pointer should be queried in each tick of the auto
    /// splitter.
    pub fn get_instance_data(self, process: &Process) -> Result<Ptr<()>, Error> {
        self.read_at_byte_offset(0x0, process)
    }
}
