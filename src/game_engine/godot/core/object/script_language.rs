//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/object/script_language.h>

/// A class stored as a resource.
///
/// [`Script`](https://docs.godotengine.org/en/4.2/classes/class_script.html)
///
/// You need to cast this to a [`GDScript`](crate::game_engine::godot::GDScript)
/// or [`CSharpScript`](crate::game_engine::godot::CSharpScript) to do anything
/// meaningful with it. Make sure to verify the script language before casting.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Script;
