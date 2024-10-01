//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/2d/collision_object_2d.h>

use super::Node2D;

/// Abstract base class for 2D physics objects.
///
/// [`CollisionObject2D`](https://docs.godotengine.org/en/4.2/classes/class_collisionobject2d.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CollisionObject2D;
extends!(CollisionObject2D: Node2D);
