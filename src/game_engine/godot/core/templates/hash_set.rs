//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/hash_set.h>

use core::marker::PhantomData;

use crate::game_engine::godot::SizeInTargetProcess;

impl<K> SizeInTargetProcess for HashSet<K> {
    const SIZE: u64 = 40;
}

/// A hash set that uniquely stores each element. This is not publicly exposed
/// in Godot.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct HashSet<K>(PhantomData<fn() -> K>);
