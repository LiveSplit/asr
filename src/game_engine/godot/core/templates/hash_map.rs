//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/hash_map.h>

use core::{iter, marker::PhantomData, num::NonZeroU32};

use crate::{
    game_engine::godot::{Ptr, SizeInTargetProcess},
    Address64, Error, Process,
};

use super::{
    hashfuncs::{fastmod, HASH_TABLE_SIZE_PRIMES, HASH_TABLE_SIZE_PRIMES_INV},
    Hash,
};

#[allow(unused)]
mod offsets {
    pub const ELEMENTS: u32 = 0x8;
    pub const HASHES: u32 = 0x10;
    pub const HEAD_ELEMENT: u32 = 0x18;
    pub const TAIL_ELEMENT: u32 = 0x20;
    pub const CAPACITY_INDEX: u32 = 0x28;
    pub const NUM_ELEMENTS: u32 = 0x2C;

    pub mod element {
        pub const NEXT: u32 = 0x00;
        pub const PREV: u32 = 0x08;
        pub const KEY: u32 = 0x10;
    }
}

impl<K, V> SizeInTargetProcess for HashMap<K, V> {
    const SIZE: u64 = 0x30;
}

const EMPTY_HASH: u32 = 0;

/// A hash map that maps keys to values. This is not publicly exposed as such in
/// Godot, because it's a template class. The closest equivalent is the general
/// [`Dictionary`](https://docs.godotengine.org/en/4.2/classes/class_dictionary.html).
///
/// Check the [`Ptr`] documentation to see all the methods you can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct HashMap<K, V>(PhantomData<fn() -> (K, V)>);

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
struct HashMapElement<K, V>(PhantomData<fn() -> (K, V)>);

impl<K: 'static, V: 'static> Ptr<HashMapElement<K, V>> {
    fn next(self, process: &Process) -> Result<Self, Error> {
        self.read_at_byte_offset(offsets::element::NEXT, process)
    }

    fn prev(self, process: &Process) -> Result<Self, Error> {
        self.read_at_byte_offset(offsets::element::PREV, process)
    }

    fn key(self) -> Ptr<K> {
        Ptr::new(self.addr() + offsets::element::KEY)
    }

    fn value(self) -> Ptr<V>
    where
        K: SizeInTargetProcess,
    {
        Ptr::new(self.addr() + offsets::element::KEY + K::SIZE)
    }
}

impl<K: 'static, V: 'static> Ptr<HashMap<K, V>> {
    /// Returns an iterator over the key-value pairs in this hash map.
    pub fn iter<'a>(&'a self, process: &'a Process) -> impl Iterator<Item = (Ptr<K>, Ptr<V>)> + 'a
    where
        K: SizeInTargetProcess,
    {
        let mut current: Ptr<HashMapElement<K, V>> = Ptr::new(
            self.read_at_byte_offset(offsets::HEAD_ELEMENT, process)
                .unwrap_or_default(),
        );
        iter::from_fn(move || {
            if current.is_null() {
                return None;
            }
            let pair = (current.key(), current.value());
            current = current.next(process).ok()?;
            Some(pair)
        })
    }

    /// Returns a backwards iterator over the key-value pairs in this hash map.
    pub fn iter_back<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl Iterator<Item = (Ptr<K>, Ptr<V>)> + 'a
    where
        K: SizeInTargetProcess,
    {
        let mut current: Ptr<HashMapElement<K, V>> = Ptr::new(
            self.read_at_byte_offset(offsets::TAIL_ELEMENT, process)
                .unwrap_or_default(),
        );
        iter::from_fn(move || {
            if current.is_null() {
                return None;
            }
            let pair = (current.key(), current.value());
            current = current.prev(process).ok()?;
            Some(pair)
        })
    }

    /// Returns the value associated with the given key, or [`None`] if the key
    /// is not in the hash map.
    pub fn get<Q>(self, key: &Q, process: &Process) -> Result<Option<Ptr<V>>, Error>
    where
        K: Hash<Q> + SizeInTargetProcess,
    {
        match self.lookup_pos(key, process)? {
            Some(element) => Ok(Some(element.value())),
            None => Ok(None),
        }
    }

    /// Returns the number of elements in this hash map.
    pub fn size(self, process: &Process) -> Result<u32, Error> {
        self.read_at_byte_offset(offsets::NUM_ELEMENTS, process)
    }

    fn get_capacity_index(self, process: &Process) -> Result<u32, Error> {
        self.read_at_byte_offset(offsets::CAPACITY_INDEX, process)
    }

    fn lookup_pos<Q>(
        self,
        key: &Q,
        process: &Process,
    ) -> Result<Option<Ptr<HashMapElement<K, V>>>, Error>
    where
        K: Hash<Q>,
    {
        let capacity_index = self.get_capacity_index(process)?;

        let capacity = *HASH_TABLE_SIZE_PRIMES
            .get(capacity_index as usize)
            .ok_or(Error {})?;

        let capacity_inv = *HASH_TABLE_SIZE_PRIMES_INV
            .get(capacity_index as usize)
            .ok_or(Error {})?;

        let hash = Self::hash(key);
        let mut pos = fastmod(hash, capacity_inv, capacity);
        let mut distance = 0;

        let [elements_ptr, hashes_ptr]: [Address64; 2] =
            self.read_at_byte_offset(offsets::ELEMENTS, process)?;

        for _ in 0..10000 {
            let current_hash: u32 =
                process.read(hashes_ptr + pos.checked_mul(4).ok_or(Error {})?)?;

            if current_hash == EMPTY_HASH {
                return Ok(None);
            }

            if distance > get_probe_length(pos, current_hash, capacity, capacity_inv) {
                return Ok(None);
            }

            if current_hash == hash {
                let element_ptr: Ptr<HashMapElement<K, V>> =
                    process.read(elements_ptr + pos.checked_mul(8).ok_or(Error {})?)?;
                let element_key = element_ptr.key().deref(process)?;
                if K::eq(&element_key, key, process) {
                    return Ok(Some(element_ptr));
                }
            }

            pos = fastmod(pos.wrapping_add(1), capacity_inv, capacity);
            distance += 1;
        }

        Err(Error {})
    }

    fn hash<Q>(key: &Q) -> u32
    where
        K: Hash<Q>,
    {
        let hash = K::hash_of_lookup_key(key);

        if hash == EMPTY_HASH {
            EMPTY_HASH + 1
        } else {
            hash
        }
    }
}

fn get_probe_length(p_pos: u32, p_hash: u32, p_capacity: NonZeroU32, p_capacity_inv: u64) -> u32 {
    let original_pos = fastmod(p_hash, p_capacity_inv, p_capacity);
    fastmod(
        p_pos
            .wrapping_sub(original_pos)
            .wrapping_add(p_capacity.get()),
        p_capacity_inv,
        p_capacity,
    )
}
