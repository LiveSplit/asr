//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/main/window.h>

use super::Viewport;

/// Base class for all windows, dialogs, and popups.
///
/// [`Window`](https://docs.godotengine.org/en/4.2/classes/class_window.html)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Window;
extends!(Window: Viewport);
