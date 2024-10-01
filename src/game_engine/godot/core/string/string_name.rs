//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/string/string_name.h>

use core::mem::MaybeUninit;

use arrayvec::ArrayVec;
use bytemuck::{Pod, Zeroable};

use crate::{
    game_engine::godot::{Hash, Ptr, SizeInTargetProcess},
    Address64, Error, Process,
};

use super::String;

#[allow(unused)]
mod offsets {
    pub mod data {
        use super::super::{SizeInTargetProcess, String};

        pub const REFCOUNT: u64 = 0x00;
        pub const STATIC_COUNT: u64 = 0x04;
        pub const CNAME: u64 = 0x08;
        pub const NAME: u64 = 0x10;
        pub const IDX: u64 = NAME + String::<0>::SIZE;
        pub const HASH: u64 = IDX + 0x4;
    }
}

/// A built-in type for unique strings.
///
/// [`StringName`](https://docs.godotengine.org/en/4.2/classes/class_stringname.html)
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
pub struct StringName {
    data: Ptr<Data>,
}

impl<const N: usize> Hash<[u8; N]> for StringName {
    fn hash_of_lookup_key(lookup_key: &[u8; N]) -> u32 {
        // String::hash
        let mut hashv: u32 = 5381;

        for c in lossy_chars(lookup_key) {
            hashv = hashv.wrapping_mul(33).wrapping_add(c as u32);
        }

        hashv
    }

    fn eq(&self, lookup_key: &[u8; N], process: &Process) -> bool {
        let Ok(name) = self.read::<N>(process) else {
            return false;
        };
        name.chars().eq(lossy_chars(lookup_key))
    }
}

fn lossy_chars(lookup_key: &[u8]) -> impl Iterator<Item = char> + '_ {
    lookup_key.utf8_chunks().flat_map(|chunk| {
        chunk.valid().chars().chain(if chunk.invalid().is_empty() {
            None
        } else {
            Some(char::REPLACEMENT_CHARACTER)
        })
    })
}

impl SizeInTargetProcess for StringName {
    const SIZE: u64 = 0x8;
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
struct Data(Address64);

impl StringName {
    /// Reads the string from the target process.
    pub fn read<const N: usize>(self, process: &Process) -> Result<String<N>, Error> {
        // FIXME: This skips cname entirely atm.

        // FIXME: Use CowData
        let cow_data: Address64 = self
            .data
            .read_at_byte_offset(offsets::data::NAME, process)?;

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
