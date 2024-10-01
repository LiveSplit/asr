//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/string/ustring.h>

use arrayvec::{ArrayString, ArrayVec};

use crate::game_engine::godot::SizeInTargetProcess;

/// A built-in type for strings.
///
/// [`String`](https://docs.godotengine.org/en/4.2/classes/class_string.html)
#[derive(Clone)]
pub struct String<const N: usize>(pub(super) ArrayVec<u32, N>);

impl<const N: usize> SizeInTargetProcess for String<N> {
    const SIZE: u64 = 0x8;
}

impl<const N: usize> String<N> {
    /// Returns an iterator over the characters in this string.
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0
            .iter()
            .copied()
            .map(|c| char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER))
    }

    /// Converts this string to an [`ArrayString`]. If the string is too long to
    /// fit in the array, the excess characters are truncated.
    pub fn to_array_string<const UTF8_SIZE: usize>(&self) -> ArrayString<UTF8_SIZE> {
        let mut buf = ArrayString::<UTF8_SIZE>::new();
        for c in self.chars() {
            let _ = buf.try_push(c);
        }
        buf
    }

    /// Checks if this string matches the given string.
    pub fn matches_str(&self, text: &str) -> bool {
        self.chars().eq(text.chars())
    }
}
