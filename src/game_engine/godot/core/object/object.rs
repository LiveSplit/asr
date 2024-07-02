//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/object/object.h>

use crate::{
    game_engine::godot::{Ptr, VTable},
    Error, Process,
};

/// Base class for all other classes in the engine.
///
/// [`Object`](https://docs.godotengine.org/en/4.2/classes/class_object.html)
///
/// Check the [`Ptr<Object>`] documentation to see all the methods you can call
/// on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Object;

impl Ptr<Object> {
    /// Returns a pointer to the object's virtual method table.
    pub fn get_vtable(self, process: &Process) -> Result<Ptr<VTable>, Error> {
        process.read(self.addr())
    }
}
