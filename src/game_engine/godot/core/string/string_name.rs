//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/string/string_name.h>

use core::mem::MaybeUninit;

use arrayvec::ArrayVec;
use bytemuck::{Pod, Zeroable};

use crate::{
    game_engine::godot::{KnownSize, Ptr},
    Address64, Error, Process,
};

use super::String;

/// A built-in type for unique strings.
///
/// [`StringName`](https://docs.godotengine.org/en/4.2/classes/class_stringname.html)
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
pub struct StringName(Ptr<StringNameData>);

impl KnownSize for StringName {}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
struct StringNameData(Address64);

impl StringName {
    /// Reads the string from the target process.
    pub fn read<const N: usize>(self, process: &Process) -> Result<String<N>, Error> {
        let cow_data: Address64 = self.0.read_at_offset(0x10, process)?;

        // Only on 4.2 or before.
        let len = process
            .read::<u32>(cow_data + -0x4)?
            .checked_sub(1)
            .ok_or(Error {})?;
        let mut buf = [MaybeUninit::uninit(); N];
        let buf = buf.get_mut(..len as usize).ok_or(Error {})?;
        let buf = process.read_into_uninit_slice(cow_data, buf)?;

        let mut out = ArrayVec::new();
        out.extend(buf.iter().copied());

        Ok(String(out))
    }
}
