use arrayvec::ArrayVec;
use bytemuck::CheckedBitPattern;
use core::mem::{size_of, MaybeUninit};

use super::ManagedString;
use crate::{Address, Address32, Address64, Error, PointerSize, Process};

/// How many bytes a managed object's two header words occupy. The readers
/// skip them; nothing in them is read.
const fn object_header(pointer_size: PointerSize) -> u64 {
    2 * pointer_size as u64
}

/// Reads a managed string through the reference stored at the given address.
pub fn read_string<const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    at: Address,
) -> Result<ManagedString<N>, Error> {
    let object = process
        .read_pointer(at, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    read_string_object(process, pointer_size, object)
}

/// Reads a managed string at its object address, the form a reference
/// array's elements hand back. The layout is runtime ABI, shared by both
/// runtimes at both widths: the character count as an i32 past the object
/// header, the UTF-16 characters inline behind it. The count is the string's
/// length, so a nul character inside the string is kept like any other.
pub fn read_string_object<const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    object: Address,
) -> Result<ManagedString<N>, Error> {
    if object.is_null() {
        return Err(Error {});
    }

    let header = object_header(pointer_size);
    let count = process.read::<i32>(object + header)?;

    // The buffer size is the bound past which a claimed count is nonsense: a
    // torn or garbage read claims billions, and something has to refuse
    // before reading it. Refusal, never truncation.
    let count = usize::try_from(count)
        .ok()
        .filter(|&count| count <= N)
        .ok_or(Error {})?;

    let mut units = [0_u16; N];
    let characters = &mut bytemuck::cast_slice_mut::<u16, u8>(&mut units)[..2 * count];
    process.read_into_slice(object + header + 4, characters)?;

    ManagedString::from_units(&units[..count]).ok_or(Error {})
}

/// Where a list keeps its backing array and live count, resolved once from
/// the list's class hierarchy and held by the caller, so the per-tick read
/// costs reads rather than a metadata walk.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListOffsets {
    pub(crate) items: u32,
    pub(crate) size: u32,
}

/// The most bytes one entry may span: past this a claimed layout is
/// garbage, and at most this many are read per chunk.
pub(crate) const ENTRY_SCRATCH: usize = 1024;

/// How one dictionary entry lays out, measured from the entry's own start:
/// the stored hash, the chain link, and the key and value slots.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntryLayout {
    pub(crate) stride: u32,
    pub(crate) hash: u32,
    pub(crate) next: u32,
    pub(crate) key: u32,
    pub(crate) value: u32,
}

/// Where a dictionary keeps its backing storage and live counts, and how
/// its entries lay out, resolved once from the dictionary's class hierarchy
/// and held by the caller, so the per-tick read costs reads rather than a
/// metadata walk. The entry layout depends on the concrete key and value
/// types, so offsets must only be reused for the same dictionary type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DictionaryOffsets {
    pub(crate) shape: DictionaryShape,
}

/// The corlib generations lay dictionaries out two ways: one entries array
/// of structs with buckets beside it, or, in the oldest corlib, four
/// arrays side by side whose links carry a live flag in their stored
/// hashes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DictionaryShape {
    Entries {
        entries: u32,
        count: u32,
        free_count: u32,
        layout: EntryLayout,
    },
    Parallel {
        link_slots: u32,
        key_slots: u32,
        value_slots: u32,
        touched: u32,
        count: u32,
        link_hash: u32,
    },
}

/// The flag the parallel shape sets in a link's stored hash while its slot
/// is live.
pub(crate) const LIVE_FLAG: u32 = 0x8000_0000;

/// One parallel link spans two ints.
pub(crate) const LINK_STRIDE: u32 = 8;

/// Reads a managed list's live elements through the reference stored at the
/// given address, with the offsets a resolution handed out earlier. The
/// count is judged by the buffer, the backing array's capacity is not, and
/// a count past the backing's own length is a torn resize and refuses.
pub fn read_list<T: CheckedBitPattern, const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    offsets: ListOffsets,
    at: Address,
) -> Result<ArrayVec<T, N>, Error> {
    let object = process
        .read_pointer(at, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    let size = process.read::<i32>(object + offsets.size)?;
    let size = usize::try_from(size)
        .ok()
        .filter(|&size| size <= N)
        .ok_or(Error {})?;

    let items = process
        .read_pointer(object + offsets.items, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    let header = object_header(pointer_size);
    let backing = process
        .read_pointer(items + header + pointer_size as u64, pointer_size)?
        .value();
    if usize::try_from(backing)
        .ok()
        .filter(|&backing| size <= backing)
        .is_none()
    {
        return Err(Error {});
    }

    let mut elements = [const { MaybeUninit::<T>::uninit() }; N];
    let elements = process.read_into_uninit_slice(
        items + header + 2 * pointer_size as u64,
        &mut elements[..size],
    )?;

    let mut out = ArrayVec::new();
    out.try_extend_from_slice(elements).map_err(|_| Error {})?;
    Ok(out)
}

/// The most entries a dictionary may claim: a torn header must not buy a
/// scan proportional to whatever it says.
const MOST_ENTRIES: u32 = 1 << 20;

/// The freed-entry marks: some runtimes write the mark over the stored
/// hash, others link the chain word below the empty value.
fn freed(hash: u32, next: i32) -> bool {
    hash == u32::MAX || next < -1
}

/// What a member may span before it runs into the member behind it, or the
/// entry's end.
fn room(members: &[u32], stride: u32, member: u32) -> u32 {
    members
        .iter()
        .copied()
        .filter(|&other| other > member)
        .min()
        .unwrap_or(stride)
        - member
}

/// Walks the counted entries in chunks of the scratch, handing each live
/// one's bytes on and skipping the freed by their marks.
fn scan_entries(
    process: &Process,
    elements: Address,
    total: usize,
    stride: usize,
    hash: u32,
    next: u32,
    mut live: impl FnMut(&[u8]) -> Result<(), Error>,
) -> Result<(), Error> {
    let per_chunk = ENTRY_SCRATCH / stride;
    let mut scratch = [0; ENTRY_SCRATCH];

    let mut index = 0;
    while index < total {
        let taken = per_chunk.min(total - index);
        let bytes = &mut scratch[..taken * stride];
        process.read_into_slice(elements + (index * stride) as u64, bytes)?;

        for entry in bytes.chunks_exact(stride) {
            let word = |member: u32| {
                let member = member as usize;
                entry[member..member + 4].try_into().expect("four bytes")
            };
            if !freed(
                u32::from_le_bytes(word(hash)),
                i32::from_le_bytes(word(next)),
            ) {
                live(entry)?;
            }
        }

        index += taken;
    }

    Ok(())
}

/// Dereferences a backing array whose length has to reach the counted
/// entries, answering where its elements start.
fn array_elements(
    process: &Process,
    pointer_size: PointerSize,
    object: Address,
    field: u32,
    reach: u32,
) -> Result<Address, Error> {
    let array = process
        .read_pointer(object + field, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    let header = object_header(pointer_size);
    let length = process
        .read_pointer(array + header + pointer_size as u64, pointer_size)?
        .value();
    if u64::from(reach) > length {
        return Err(Error {});
    }

    Ok(array + header + 2 * pointer_size as u64)
}

/// Walks the parallel links in chunks, handing on each index whose stored
/// hash carries the live flag.
fn scan_links(
    process: &Process,
    links: Address,
    touched: usize,
    link_hash: u32,
    mut live: impl FnMut(usize) -> Result<(), Error>,
) -> Result<(), Error> {
    let stride = LINK_STRIDE as usize;
    let per_chunk = ENTRY_SCRATCH / stride;
    let mut scratch = [0; ENTRY_SCRATCH];

    let mut index = 0;
    while index < touched {
        let taken = per_chunk.min(touched - index);
        let bytes = &mut scratch[..taken * stride];
        process.read_into_slice(links + (index * stride) as u64, bytes)?;

        for (offset, link) in bytes.chunks_exact(stride).enumerate() {
            let member = link_hash as usize;
            let hash = u32::from_le_bytes(link[member..member + 4].try_into().expect("four bytes"));
            if hash & LIVE_FLAG != 0 {
                live(index + offset)?;
            }
        }

        index += taken;
    }

    Ok(())
}

/// Reads a managed dictionary's live pairs through the reference stored at
/// the given address, with the offsets a resolution handed out earlier.
/// The buffer judges the live pairs, never the counted entries or the
/// backing capacity; freed entries are skipped by their marks, and a live
/// tally that cannot balance against the counts fails rather than
/// answering wrong pairs. The key and value types are the caller's claims,
/// refused where a claim outgrows its member's room.
pub fn read_dictionary<K: CheckedBitPattern, V: CheckedBitPattern, const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    offsets: DictionaryOffsets,
    at: Address,
) -> Result<ArrayVec<(K, V), N>, Error> {
    let object = process
        .read_pointer(at, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    let mut out = ArrayVec::new();
    match offsets.shape {
        DictionaryShape::Entries {
            entries,
            count,
            free_count,
            layout,
        } => {
            let members = [layout.hash, layout.next, layout.key, layout.value];
            if size_of::<K>() as u32 > room(&members, layout.stride, layout.key)
                || size_of::<V>() as u32 > room(&members, layout.stride, layout.value)
            {
                return Err(Error {});
            }

            let counted = process.read::<i32>(object + count)?;
            let free = process.read::<i32>(object + free_count)?;
            let (counted, free) = match (u32::try_from(counted), u32::try_from(free)) {
                (Ok(counted), Ok(free)) if free <= counted && counted <= MOST_ENTRIES => {
                    (counted, free)
                }
                _ => return Err(Error {}),
            };
            let live = (counted - free) as usize;
            if live > N {
                return Err(Error {});
            }

            // A dictionary constructed without capacity has no backing arrays
            // until its first insertion. That is the canonical empty
            // representation, not an unreadable dictionary.
            if counted == 0 {
                return Ok(out);
            }

            let elements = array_elements(process, pointer_size, object, entries, counted)?;
            scan_entries(
                process,
                elements,
                counted as usize,
                layout.stride as usize,
                layout.hash,
                layout.next,
                |entry| {
                    let at =
                        |member: u32, len: usize| &entry[member as usize..member as usize + len];
                    let key =
                        bytemuck::checked::try_pod_read_unaligned(at(layout.key, size_of::<K>()))
                            .map_err(|_| Error {})?;
                    let value =
                        bytemuck::checked::try_pod_read_unaligned(at(layout.value, size_of::<V>()))
                            .map_err(|_| Error {})?;
                    out.try_push((key, value)).map_err(|_| Error {})
                },
            )?;

            if out.len() != live {
                return Err(Error {});
            }
        }
        DictionaryShape::Parallel {
            link_slots,
            key_slots,
            value_slots,
            touched,
            count,
            link_hash,
        } => {
            let reached = process.read::<i32>(object + touched)?;
            let counted = process.read::<i32>(object + count)?;
            let (reached, counted) = match (u32::try_from(reached), u32::try_from(counted)) {
                (Ok(reached), Ok(counted)) if counted <= reached && reached <= MOST_ENTRIES => {
                    (reached, counted)
                }
                _ => return Err(Error {}),
            };
            if counted as usize > N {
                return Err(Error {});
            }

            let links = array_elements(process, pointer_size, object, link_slots, reached)?;
            let keys = array_elements(process, pointer_size, object, key_slots, reached)?;
            let values = array_elements(process, pointer_size, object, value_slots, reached)?;

            scan_links(process, links, reached as usize, link_hash, |index| {
                let key = process.read(keys + (index * size_of::<K>()) as u64)?;
                let value = process.read(values + (index * size_of::<V>()) as u64)?;
                out.try_push((key, value)).map_err(|_| Error {})
            })?;

            if out.len() != counted as usize {
                return Err(Error {});
            }
        }
    }

    Ok(out)
}

/// How one hash set slot lays out, measured from the slot's own start: the
/// entry layout without a key.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlotLayout {
    pub(crate) stride: u32,
    pub(crate) hash: u32,
    pub(crate) next: u32,
    pub(crate) value: u32,
}

/// Where a hash set keeps its backing storage, live count, and high-water
/// mark, and how its slots lay out, resolved once from the set's class
/// hierarchy and held by the caller, so the per-tick read costs reads rather
/// than a metadata walk. The slot layout depends on the concrete value type,
/// so offsets must only be reused for the same hash set type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HashSetOffsets {
    pub(crate) shape: SetShape,
}

/// The hash set's two corlib layouts, mirroring the dictionary's.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SetShape {
    Slots {
        slots: u32,
        count: u32,
        last_index: u32,
        layout: SlotLayout,
    },
    Parallel {
        links: u32,
        slots: u32,
        touched: u32,
        count: u32,
        link_hash: u32,
    },
}

/// Reads a managed hash set's live values through the reference stored at
/// the given address, with the offsets a resolution handed out earlier.
/// The walk runs to the high-water mark rather than the live count, since
/// freed slots sit inside it; the buffer judges the live values, and the
/// live tally has to balance against the count exactly. The value type is
/// the caller's claim, refused where it outgrows the slot's room.
pub fn read_hash_set<T: CheckedBitPattern, const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    offsets: HashSetOffsets,
    at: Address,
) -> Result<ArrayVec<T, N>, Error> {
    let object = process
        .read_pointer(at, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    let mut out = ArrayVec::new();
    match offsets.shape {
        SetShape::Slots {
            slots,
            count,
            last_index,
            layout,
        } => {
            let members = [layout.hash, layout.next, layout.value];
            if size_of::<T>() as u32 > room(&members, layout.stride, layout.value) {
                return Err(Error {});
            }

            let counted = process.read::<i32>(object + count)?;
            let last = process.read::<i32>(object + last_index)?;
            let (counted, last) = match (u32::try_from(counted), u32::try_from(last)) {
                (Ok(counted), Ok(last)) if counted <= last && last <= MOST_ENTRIES => {
                    (counted, last)
                }
                _ => return Err(Error {}),
            };
            if counted as usize > N {
                return Err(Error {});
            }

            // A hash set constructed without capacity has no backing arrays
            // until its first insertion. That is the canonical empty
            // representation, not an unreadable hash set.
            if last == 0 {
                return Ok(out);
            }

            let elements = array_elements(process, pointer_size, object, slots, last)?;
            scan_entries(
                process,
                elements,
                last as usize,
                layout.stride as usize,
                layout.hash,
                layout.next,
                |slot| {
                    let bytes =
                        &slot[layout.value as usize..layout.value as usize + size_of::<T>()];
                    let value =
                        bytemuck::checked::try_pod_read_unaligned(bytes).map_err(|_| Error {})?;
                    out.try_push(value).map_err(|_| Error {})
                },
            )?;

            if out.len() != counted as usize {
                return Err(Error {});
            }
        }
        SetShape::Parallel {
            links,
            slots,
            touched,
            count,
            link_hash,
        } => {
            let reached = process.read::<i32>(object + touched)?;
            let counted = process.read::<i32>(object + count)?;
            let (reached, counted) = match (u32::try_from(reached), u32::try_from(counted)) {
                (Ok(reached), Ok(counted)) if counted <= reached && reached <= MOST_ENTRIES => {
                    (reached, counted)
                }
                _ => return Err(Error {}),
            };
            if counted as usize > N {
                return Err(Error {});
            }

            let links = array_elements(process, pointer_size, object, links, reached)?;
            let values = array_elements(process, pointer_size, object, slots, reached)?;

            scan_links(process, links, reached as usize, link_hash, |index| {
                let value = process.read(values + (index * size_of::<T>()) as u64)?;
                out.try_push(value).map_err(|_| Error {})
            })?;

            if out.len() != counted as usize {
                return Err(Error {});
            }
        }
    }

    Ok(out)
}

/// Reads a managed array of value elements through the reference stored at
/// the given address. The layout is runtime ABI: past the object header sit
/// the bounds word, the length, and the elements inline.
pub fn read_array<T: CheckedBitPattern, const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    at: Address,
) -> Result<ArrayVec<T, N>, Error> {
    let object = process
        .read_pointer(at, pointer_size)
        .ok()
        .filter(|address| !address.is_null())
        .ok_or(Error {})?;

    let header = object_header(pointer_size);

    // The length judges at pointer width before any narrowing, so garbage
    // that a narrower read would truncate small still refuses. Only IL2CPP's
    // length is truly pointer-sized; 64-bit mono stores a u32 whose zeroed
    // padding reads the same value through the wide slot.
    let length = process
        .read_pointer(object + header + pointer_size as u64, pointer_size)?
        .value();
    let length = usize::try_from(length)
        .ok()
        .filter(|&length| length <= N)
        .ok_or(Error {})?;

    let mut elements = [const { MaybeUninit::<T>::uninit() }; N];
    let elements = process.read_into_uninit_slice(
        object + header + 2 * pointer_size as u64,
        &mut elements[..length],
    )?;

    let mut array = ArrayVec::new();
    array
        .try_extend_from_slice(elements)
        .map_err(|_| Error {})?;
    Ok(array)
}

/// Reads a managed array of reference elements through the reference
/// stored at the given address, handing back the elements' object
/// addresses. The element slots are pointers at the target's own width,
/// which the reader owns rather than taking as a claim. Null elements
/// are data and stay at their positions; a null array reference fails.
pub fn read_reference_array<const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    at: Address,
) -> Result<ArrayVec<Address, N>, Error> {
    match pointer_size {
        PointerSize::Bit64 => Ok(read_array::<Address64, N>(process, pointer_size, at)?
            .iter()
            .copied()
            .map(Into::into)
            .collect()),
        PointerSize::Bit32 => Ok(read_array::<Address32, N>(process, pointer_size, at)?
            .iter()
            .copied()
            .map(Into::into)
            .collect()),
        // No managed runtime is 16-bit.
        PointerSize::Bit16 => Err(Error {}),
    }
}

/// Reads a managed list of reference elements through the reference
/// stored at the given address, with the offsets a resolution handed out
/// earlier, handing back the elements' object addresses the way
/// [`read_reference_array`] does. The count and backing checks are
/// [`read_list`]'s.
pub fn read_reference_list<const N: usize>(
    process: &Process,
    pointer_size: PointerSize,
    offsets: ListOffsets,
    at: Address,
) -> Result<ArrayVec<Address, N>, Error> {
    match pointer_size {
        PointerSize::Bit64 => Ok(
            read_list::<Address64, N>(process, pointer_size, offsets, at)?
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        PointerSize::Bit32 => Ok(
            read_list::<Address32, N>(process, pointer_size, offsets, at)?
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        // No managed runtime is 16-bit.
        PointerSize::Bit16 => Err(Error {}),
    }
}
