//! A fake host for tests: definitions of the wasm imports the runtime layer
//! links against, backed by in-memory images so readers can run on the host.

use core::{cell::RefCell, num::NonZeroU64};

use std::vec::Vec;

use crate::Process;

std::thread_local! {
    static MEMORY: RefCell<Vec<(u64, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
}

/// Runs a test against a process whose memory holds the given regions, each an
/// address and the bytes starting there. Reads outside every region fail.
pub fn with_process<R>(regions: &[(u64, &[u8])], test: impl FnOnce(&Process) -> R) -> R {
    MEMORY.with(|memory| {
        *memory.borrow_mut() = regions
            .iter()
            .map(|&(address, bytes)| (address, bytes.to_vec()))
            .collect();
    });
    let process = Process::attach("mock").expect("the mock always attaches");
    test(&process)
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
