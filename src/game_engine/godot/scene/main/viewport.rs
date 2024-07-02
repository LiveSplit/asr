//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/main/viewport.h>

use super::Node;

/// Abstract base class for viewports. Encapsulates drawing and interaction with
/// a game world.
///
/// [`Viewport`](https://docs.godotengine.org/en/4.2/classes/class_viewport.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Viewport;
extends!(Viewport: Node);
