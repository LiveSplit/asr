//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/main/canvas_item.h>

use crate::{game_engine::godot::Ptr, Error, Process};

use super::Node;

/// Abstract base class for everything in 2D space.
///
/// [`CanvasItem`](https://docs.godotengine.org/en/4.2/classes/class_canvasitem.html)
///
/// Check the [`Ptr<CanvasItem>`] documentation to see all the methods you can
/// call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CanvasItem;
extends!(CanvasItem: Node);

impl Ptr<CanvasItem> {
    /// Returns the global transform matrix of this item, i.e. the combined
    /// transform up to the topmost **CanvasItem** node. The topmost item is a
    /// **CanvasItem** that either has no parent, has non-**CanvasItem** parent
    /// or it has `top_level` enabled.
    ///
    /// [`CanvasItem.get_global_transform`](https://docs.godotengine.org/en/4.2/classes/class_canvasitem.html#class-canvasitem-method-get-global-transform)
    pub fn get_global_transform(self, process: &Process) -> Result<[f32; 6], Error> {
        self.read_at_offset(0x450, process)
    }
}
