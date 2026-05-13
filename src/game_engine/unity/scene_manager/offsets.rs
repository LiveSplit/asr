use crate::PointerSize;

pub(super) struct Offsets {
    pub(super) scene_count: u8,
    pub(super) active_scene: u8,
    pub(super) dont_destroy_on_load_scene: u8,
    pub(super) asset_path: u8,
    pub(super) build_index: u8,
    pub(super) root_storage_container: u8,
    pub(super) game_object: u8,
    pub(super) game_object_name: u8,
    pub(super) game_object_activeself: u8,
    pub(super) game_object_activeinhierarchy: u8,
    pub(super) klass: u8,
    pub(super) children_pointer: u8,
}

impl Offsets {
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
                game_object_activeself: 0x5E,
                game_object_activeinhierarchy: 0x5F,
                klass: 0x28,
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
                game_object_activeself: 0x32,
                game_object_activeinhierarchy: 0x33,
                klass: 0x18,
                children_pointer: 0x50,
            }),
            _ => None,
        }
    }
}
