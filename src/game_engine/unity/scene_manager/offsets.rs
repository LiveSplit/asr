use crate::PointerSize;

pub struct Offsets {
    pub scene_count: u8,
    pub active_scene: u8,
    pub dont_destroy_on_load_scene: u8,
    pub asset_path: u8,
    pub build_index: u8,
    pub root_storage_container: u8,
    pub game_object: u8,
    pub game_object_name: u8,
    pub game_object_activeself: u8,
    pub game_object_activeinhierarchy: u8,
    /// a handle to the scripting object
    /// MonoObjectHandle for Mono for e.g.
    pub scripting_object_handle: u8,
    pub children_pointer: u8,
}

impl Offsets {
    // FIXME: This needs to support differentiation on unity version / binary format
    pub(super) const fn new(pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => Some(&Self {
                scene_count: 0x18,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
                asset_path: 0x10,
                build_index: 0x98,
                root_storage_container: 0xB0,
                game_object: 0x30,
                game_object_name: 0x60,
                // NOTE: I'm not sure what version of unity the above offsets were
                // gotten with, so I can't get the appropriate offsets for the active
                // fields
                game_object_activeself: 0x0,
                game_object_activeinhierarchy: 0x1,
                scripting_object_handle: 0x18,
                children_pointer: 0x70,
            }),
            PointerSize::Bit32 => Some(&Self {
                scene_count: 0x10,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
                asset_path: 0xC,
                build_index: 0x70,
                root_storage_container: 0x88,
                game_object: 0x1C,
                game_object_name: 0x3C,
                // NOTE: I'm not sure what version of unity the above offsets were
                // gotten with, so I can't get the appropriate offsets for the active
                // fields
                game_object_activeself: 0x0,
                game_object_activeinhierarchy: 0x1,
                scripting_object_handle: 0x18,
                children_pointer: 0x50,
            }),
            _ => None,
        }
    }
}
