use arrayvec::ArrayVec;
use bytemuck::CheckedBitPattern;
use core::mem::MaybeUninit;

use super::ManagedString;
use crate::{Address, Error, PointerSize, Process};

/// How many bytes a managed object's two header words occupy. The readers
/// skip them; nothing in them is read.
const fn object_header(pointer_size: PointerSize) -> u64 {
    2 * pointer_size as u64
}

/// Reads a managed string through the reference stored at the given address.
/// The layout is runtime ABI, shared by both runtimes at both widths: the
/// character count as an i32 past the object header, the UTF-16 characters
/// inline behind it. The count is the string's length, so a nul character
/// inside the string is kept like any other.
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
