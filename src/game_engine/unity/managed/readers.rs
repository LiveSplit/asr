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
