//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/3d/node_3d.h>

use crate::{
    game_engine::godot::{Node, Ptr},
    Error, Process,
};

mod offsets {
    // Transform3D
    pub const GLOBAL_TRANSFORM: u64 = 0x3C8;
    // Transform3D
    pub const LOCAL_TRANSFORM: u64 = 0x3F8;
}

/// Most basic 3D game object, parent of all 3D-related nodes.
///
/// [`Node3D`](https://docs.godotengine.org/en/4.2/classes/class_node3d.html)
///
/// Check the [`Ptr<Node3D>`] documentation to see all the methods you can call
/// on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Node3D;
extends!(Node3D: Node);

impl Ptr<Node3D> {
    /// World3D space (global) Transform3D of this node.
    ///
    /// [`Node3D.global_transform`](https://docs.godotengine.org/en/4.2/classes/class_node3d.html#class-node3d-property-global-transform)
    pub fn get_global_transform(self, process: &Process) -> Result<[[f32; 3]; 4], Error> {
        self.read_at_byte_offset(offsets::GLOBAL_TRANSFORM, process)
    }

    /// Local Transform3D of this node. This is not exposed in Godot.
    pub fn get_local_transform(self, process: &Process) -> Result<[[f32; 3]; 4], Error> {
        self.read_at_byte_offset(offsets::LOCAL_TRANSFORM, process)
    }
}
