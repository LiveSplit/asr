//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/object/script_instance.h>

/// An instance of a [`Script`](super::Script).
///
/// You need to cast this to a
/// [`GDScriptInstance`](crate::game_engine::godot::GDScriptInstance) or
/// [`CSharpScriptInstance`](crate::game_engine::godot::CSharpScriptInstance) to
/// do anything meaningful with it. Make sure to verify the script language
/// before casting.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct ScriptInstance;
