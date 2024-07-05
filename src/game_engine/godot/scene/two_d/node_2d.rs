//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/2d/node_2d.h>

use crate::{
    game_engine::godot::{CanvasItem, Ptr},
    Error, Process,
};

/// A 2D game object, inherited by all 2D-related nodes. Has a position,
/// rotation, scale, and Z index.
///
/// [`Node2D`](https://docs.godotengine.org/en/4.2/classes/class_node2d.html)
///
/// Check the [`Ptr<Node2D>`] documentation to see all the methods you can call
/// on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Node2D;
extends!(Node2D: CanvasItem);

impl Ptr<Node2D> {
    /// Position, relative to the node's parent.
    ///
    /// [`Node2D.get_position`](https://docs.godotengine.org/en/4.2/classes/class_node2d.html#class-node2d-property-position)
    pub fn get_position(self, process: &Process) -> Result<[f32; 2], Error> {
        self.read_at_byte_offset(0x48C, process)
    }

    /// Rotation in radians, relative to the node's parent.
    ///
    /// [`Node2D.get_rotation`](https://docs.godotengine.org/en/4.2/classes/class_node2d.html#class-node2d-property-rotation)
    pub fn get_rotation(self, process: &Process) -> Result<f32, Error> {
        self.read_at_byte_offset(0x494, process)
    }

    /// The node's scale. Unscaled value: `[1.0, 1.0]`.
    ///
    /// [`Node2D.get_scale`](https://docs.godotengine.org/en/4.2/classes/class_node2d.html#class-node2d-property-scale)
    pub fn get_scale(self, process: &Process) -> Result<[f32; 2], Error> {
        self.read_at_byte_offset(0x498, process)
    }
}
