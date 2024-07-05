//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/list.h>

use core::marker::PhantomData;

use crate::game_engine::godot::SizeInTargetProcess;

impl<T> SizeInTargetProcess for List<T> {
    const SIZE: u64 = 0x8;
}

/// A linked list of elements. This is not publicly exposed in Godot.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct List<T>(PhantomData<fn() -> T>);
