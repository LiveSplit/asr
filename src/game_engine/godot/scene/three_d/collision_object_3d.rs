//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/3d/collision_object_3d.h>

use super::Node3D;

/// Abstract base class for 3D physics objects.
///
/// [`CollisionObject3D`](https://docs.godotengine.org/en/4.2/classes/class_collisionobject3d.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CollisionObject3D;
extends!(CollisionObject3D: Node3D);
