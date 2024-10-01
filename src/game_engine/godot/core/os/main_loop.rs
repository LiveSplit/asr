//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/os/main_loop.h>

use crate::game_engine::godot::Object;

/// Abstract base class for the game's main loop.
///
/// [`MainLoop`](https://docs.godotengine.org/en/4.2/classes/class_mainloop.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct MainLoop;
extends!(MainLoop: Object);
