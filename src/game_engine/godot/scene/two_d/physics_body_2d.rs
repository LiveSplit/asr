//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/2d/physics_body_2d.h>

use crate::{game_engine::godot::Ptr, Error, Process};

use super::CollisionObject2D;

mod offsets {
    // Vector2
    pub const VELOCITY: u64 = 0x5C4;
}

/// Abstract base class for 2D game objects affected by physics.
///
/// [`PhysicsBody2D`](https://docs.godotengine.org/en/4.2/classes/class_physicsbody2d.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct PhysicsBody2D;
extends!(PhysicsBody2D: CollisionObject2D);

/// A 2D physics body specialized for characters moved by script.
///
/// [`CharacterBody2D`](https://docs.godotengine.org/en/4.2/classes/class_characterbody2d.html)
///
/// Check the [`Ptr<CharacterBody2D>`] documentation to see all the methods you
/// can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CharacterBody2D;
extends!(CharacterBody2D: PhysicsBody2D);

impl Ptr<CharacterBody2D> {
    /// Current velocity vector in pixels per second, used and modified during
    /// calls to `move_and_slide`.
    ///
    /// [`CharacterBody2D.velocity`](https://docs.godotengine.org/en/4.2/classes/class_characterbody2d.html#class-characterbody2d-property-velocity)
    pub fn get_velocity(self, process: &Process) -> Result<[f32; 2], Error> {
        self.read_at_byte_offset(offsets::VELOCITY, process)
    }
}
