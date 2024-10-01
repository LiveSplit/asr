//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/3d/physics_body_3d.h>

use crate::{game_engine::godot::Ptr, Error, Process};

use super::CollisionObject3D;

mod offsets {
    // Vector3
    pub const VELOCITY: u64 = 0x5D8;
}

/// Abstract base class for 3D game objects affected by physics.
///
/// [`PhysicsBody3D`](https://docs.godotengine.org/en/4.2/classes/class_physicsbody3d.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct PhysicsBody3D;
extends!(PhysicsBody3D: CollisionObject3D);

/// A 3D physics body specialized for characters moved by script.
///
/// [`CharacterBody3D`](https://docs.godotengine.org/en/4.2/classes/class_characterbody3d.html)
///
/// Check the [`Ptr<CharacterBody3D>`] documentation to see all the methods you
/// can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CharacterBody3D;
extends!(CharacterBody3D: PhysicsBody3D);

impl Ptr<CharacterBody3D> {
    /// Current velocity vector (typically meters per second), used and modified
    /// during calls to `move_and_slide`.
    ///
    /// [`CharacterBody3D.velocity`](https://docs.godotengine.org/en/4.2/classes/class_characterbody3d.html#class-characterbody3d-property-velocity)
    pub fn get_velocity(self, process: &Process) -> Result<[f32; 3], Error> {
        self.read_at_byte_offset(offsets::VELOCITY, process)
    }
}
