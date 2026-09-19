//! A fake host for tests: definitions of the wasm imports the runtime layer
//! links against, backed by in-memory images so readers can run on the host.

use core::{
    cell::RefCell,
    future::Future,
    num::NonZeroU64,
    task::{Context, Poll, Waker},
};

use std::{
    string::{String, ToString},
    vec::Vec,
};

use crate::Process;

std::thread_local! {
    static MEMORY: RefCell<Vec<(u64, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    static MODULES: RefCell<Vec<(String, u64, u64)>> = const { RefCell::new(Vec::new()) };
}

/// Runs a test against a process whose memory holds the given regions, each an
/// address and the bytes starting there. Reads outside every region fail.
pub fn with_process<R>(regions: &[(u64, &[u8])], test: impl FnOnce(&Process) -> R) -> R {
    with_modules(regions, &[], test)
}

/// Runs a test against a process whose memory holds the given regions and
/// whose loaded modules are the given names, each with an address and a size.
/// The first module is the executable of the process.
pub fn with_modules<R>(
    regions: &[(u64, &[u8])],
    modules: &[(&str, u64, u64)],
    test: impl FnOnce(&Process) -> R,
) -> R {
    MEMORY.with(|memory| {
        *memory.borrow_mut() = regions
            .iter()
            .map(|&(address, bytes)| (address, bytes.to_vec()))
            .collect();
    });
    MODULES.with(|held| {
        *held.borrow_mut() = modules
            .iter()
            .map(|&(name, address, size)| (name.to_string(), address, size))
            .collect();
    });
    let process = Process::attach("mock").expect("the mock always attaches");
    test(&process)
}

/// A `UnityPlayer.dll` image of 0x1000 bytes: a PE32+ header for x64 whose
/// resource directory holds one version resource carrying the given file
/// version as its four parts.
pub fn unity_player_image(unity: (u16, u16, u16, u16)) -> Vec<u8> {
    fn put(image: &mut [u8], at: usize, bytes: &[u8]) {
        image[at..at + bytes.len()].copy_from_slice(bytes);
    }
    let mut i = std::vec![0; 0x1000];
    put(&mut i, 0x00, b"MZ");
    put(&mut i, 0x3C, &0x80_u32.to_le_bytes());
    put(&mut i, 0x80, b"PE\0\0");
    put(&mut i, 0x84, &0x8664_u16.to_le_bytes()); // machine: x64
    put(&mut i, 0x94, &0xF0_u16.to_le_bytes()); // size of optional header
    put(&mut i, 0x98, &0x20B_u16.to_le_bytes()); // PE32+
    put(&mut i, 0x98 + 0x38, &0x1000_u32.to_le_bytes()); // size of image
    put(&mut i, 0x98 + 0x80, &0x400_u32.to_le_bytes()); // resource directory
    put(&mut i, 0x98 + 0x84, &0x100_u32.to_le_bytes());
    let entry = |i: &mut [u8], at: usize, id: u32, offset: u32| {
        put(i, at, &id.to_le_bytes());
        put(i, at + 4, &offset.to_le_bytes());
    };
    // root: one id entry, RT_VERSION (0x10), a directory at +0x20
    put(&mut i, 0x400 + 0xE, &1_u16.to_le_bytes());
    entry(&mut i, 0x410, 0x10, 0x8000_0020);
    // type directory: one directory entry at +0x40
    put(&mut i, 0x420 + 0xE, &1_u16.to_le_bytes());
    entry(&mut i, 0x430, 1, 0x8000_0040);
    // language directory: one data entry at +0x60
    put(&mut i, 0x440 + 0xE, &1_u16.to_le_bytes());
    entry(&mut i, 0x450, 0x409, 0x60);
    // the data entry names VS_VERSIONINFO at 0x600
    put(&mut i, 0x460, &0x600_u32.to_le_bytes());
    // VS_FIXEDFILEINFO sits 0x28 in
    put(&mut i, 0x628, &0xFEEF_04BD_u32.to_le_bytes());
    put(&mut i, 0x630, &unity.1.to_le_bytes());
    put(&mut i, 0x632, &unity.0.to_le_bytes());
    put(&mut i, 0x634, &unity.3.to_le_bytes());
    put(&mut i, 0x636, &unity.2.to_le_bytes());
    i
}

fn module(name_ptr: *const u8, name_len: usize) -> Option<(u64, u64)> {
    // SAFETY: The runtime layer passes a pointer to name_len bytes of UTF-8.
    let name =
        unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(name_ptr, name_len)) };
    MODULES.with(|held| {
        held.borrow()
            .iter()
            .find(|(held_name, _, _)| held_name == name)
            .map(|&(_, address, size)| (address, size))
    })
}

/// The host's log. Tests have no need to see it.
#[no_mangle]
const extern "C" fn runtime_print_message(_text_ptr: *const u8, _text_len: usize) {}

#[no_mangle]
extern "C" fn process_get_module_address(
    _process: u64,
    name_ptr: *const u8,
    name_len: usize,
) -> Option<NonZeroU64> {
    NonZeroU64::new(module(name_ptr, name_len)?.0)
}

#[no_mangle]
extern "C" fn process_get_module_size(
    _process: u64,
    name_ptr: *const u8,
    name_len: usize,
) -> Option<NonZeroU64> {
    NonZeroU64::new(module(name_ptr, name_len)?.1)
}

/// Polls a future a single time. The mock host answers everything
/// synchronously, so a future either resolves on its first poll or sits on a
/// condition the fixture never satisfies.
pub fn poll_once<F: Future>(future: F) -> Poll<F::Output> {
    core::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop()))
}

/// The path of the executable, which is the first module. A null buffer
/// asks for the length.
#[no_mangle]
extern "C" fn process_get_path(_process: u64, buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool {
    MODULES.with(|held| {
        let held = held.borrow();
        let Some((name, _, _)) = held.first() else {
            return false;
        };
        let path = std::format!("/mnt/c/game/{name}");
        // SAFETY: The runtime layer passes a valid pointer to the length, and
        // either a null buffer or a buffer of that length.
        unsafe {
            let len = *buf_len_ptr;
            *buf_len_ptr = path.len();
            if buf_ptr.is_null() {
                return true;
            }
            if len < path.len() {
                return false;
            }
            core::ptr::copy_nonoverlapping(path.as_ptr(), buf_ptr, path.len());
        }
        true
    })
}

#[no_mangle]
extern "C" fn process_attach(_name_ptr: *const u8, _name_len: usize) -> Option<NonZeroU64> {
    NonZeroU64::new(1)
}

#[no_mangle]
extern "C" fn process_detach(_process: u64) {}

#[no_mangle]
extern "C" fn process_get_memory_range_count(_process: u64) -> Option<NonZeroU64> {
    MEMORY.with(|memory| NonZeroU64::new(memory.borrow().len() as u64))
}

#[no_mangle]
extern "C" fn process_get_memory_range_address(_process: u64, idx: u64) -> Option<NonZeroU64> {
    MEMORY.with(|memory| {
        let memory = memory.borrow();
        NonZeroU64::new(memory.get(idx as usize)?.0)
    })
}

#[no_mangle]
extern "C" fn process_get_memory_range_size(_process: u64, idx: u64) -> Option<NonZeroU64> {
    MEMORY.with(|memory| {
        let memory = memory.borrow();
        NonZeroU64::new(memory.get(idx as usize)?.1.len() as u64)
    })
}

#[no_mangle]
extern "C" fn process_read(_process: u64, address: u64, buf_ptr: *mut u8, buf_len: usize) -> bool {
    MEMORY.with(|memory| {
        memory.borrow().iter().any(|(start, bytes)| {
            let Some(offset) = address.checked_sub(*start) else {
                return false;
            };
            let Ok(offset) = usize::try_from(offset) else {
                return false;
            };
            if !offset
                .checked_add(buf_len)
                .is_some_and(|end| end <= bytes.len())
            {
                return false;
            }
            // SAFETY: The runtime layer passes a buffer valid for buf_len
            // bytes, and the range is checked to lie inside the region.
            unsafe {
                core::ptr::copy_nonoverlapping(bytes.as_ptr().add(offset), buf_ptr, buf_len);
            }
            true
        })
    })
}

#[cfg(test)]
mod tests {
    use super::with_process;
    use crate::Address;

    #[test]
    fn ranges_mirror_the_regions() {
        with_process(&[(0x1000, &[1, 2]), (0x4000, &[3, 4, 5])], |process| {
            let mut ranges = process.memory_ranges();
            let range = ranges.next().unwrap();
            assert_eq!(range.address().unwrap(), Address::new(0x1000));
            assert_eq!(range.size().unwrap(), 2);
            let range = ranges.next().unwrap();
            assert_eq!(range.address().unwrap(), Address::new(0x4000));
            assert_eq!(range.size().unwrap(), 3);
            assert!(ranges.next().is_none());
        });
    }

    #[test]
    fn reads_come_from_the_regions() {
        with_process(&[(0x1000, &[1, 2, 3, 4])], |process| {
            assert_eq!(process.read::<u32>(0x1000_u64).unwrap(), 0x04030201);
            assert_eq!(process.read::<[u8; 2]>(0x1002_u64).unwrap(), [3, 4]);
            assert!(process.read::<u32>(0x0FFF_u64).is_err());
            assert!(process.read::<u32>(0x1001_u64).is_err());
        });
    }
}
