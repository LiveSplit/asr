pub use memory_range::*;
pub use process::*;

mod memory_range;
mod process;
mod sys;

#[cfg(all(test, not(target_family = "wasm")))]
pub(crate) mod mock;
pub mod settings;
pub mod timer;

/// An error returned by a runtime function.
#[derive(Debug)]
#[non_exhaustive]
pub struct Error {}

/// Sets the tick rate of the runtime. This influences how many times per second
/// the `update` function is called. The default tick rate is 120 ticks per
/// second.
#[inline]
pub fn set_tick_rate(ticks_per_second: f64) {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::runtime_set_tick_rate(ticks_per_second) }
}

/// Prints a log message for debugging purposes.
#[inline]
pub fn print_message(text: &str) {
    // SAFETY: We provide a valid pointer and length to text that is UTF-8 encoded.
    unsafe { sys::runtime_print_message(text.as_ptr(), text.len()) }
}

/// Prints a log message for debugging purposes by formatting the given message
/// into a stack allocated buffer with the given capacity. This is useful for
/// printing dynamic messages without needing an allocator. However the message
/// may be truncated if it is too long.
///
/// # Example
///
/// ```no_run
/// asr::print_limited::<128>(&format_args!("Hello, {}!", "world"));
/// ```
#[inline(never)]
pub fn print_limited<const CAP: usize>(message: &dyn core::fmt::Display) {
    let mut buf = arrayvec::ArrayString::<CAP>::new();
    let _ = core::fmt::Write::write_fmt(&mut buf, format_args!("{message}"));
    print_message(&buf);
}

/// Queries the name of the operating system that the runtime is running on. Due
/// to emulation this may not be the same as the operating system that an
/// individual process is targeting.
///
/// Example values: `windows`, `linux`, `macos`
#[inline]
pub fn get_os() -> Result<arrayvec::ArrayString<16>, Error> {
    let mut buf = arrayvec::ArrayString::<16>::new();
    // SAFETY: We provide a valid pointer and length to the buffer. We check
    // whether the buffer was successfully filled and set the length of the
    // buffer accordingly. The buffer is guaranteed to be valid UTF-8.
    unsafe {
        let mut len = buf.capacity();
        let success = sys::runtime_get_os(buf.as_mut_ptr(), &mut len);
        if !success {
            return Err(Error {});
        }
        buf.set_len(len);
    }
    Ok(buf)
}

/// Queries the architecture that the runtime is running on. Due to emulation
/// this may not be the same as the architecture that an individual process is
/// targeting. For example 64-bit operating systems usually can run 32-bit
/// processes. Also modern operating systems running on aarch64 often have
/// backwards compatibility with x86_64 processes.
///
/// Example values: `x86`, `x86_64`, `arm`, `aarch64`
#[inline]
pub fn get_arch() -> Result<arrayvec::ArrayString<16>, Error> {
    let mut buf = arrayvec::ArrayString::<16>::new();
    // SAFETY: We provide a valid pointer and length to the buffer. We check
    // whether the buffer was successfully filled and set the length of the
    // buffer accordingly. The buffer is guaranteed to be valid UTF-8.
    unsafe {
        let mut len = buf.capacity();
        let success = sys::runtime_get_arch(buf.as_mut_ptr(), &mut len);
        if !success {
            return Err(Error {});
        }
        buf.set_len(len);
    }
    Ok(buf)
}

/// Queries the WASI path of the currently loaded splits file (`.lss`).
///
/// Returns [`None`] if the host has no file (unsaved run, debugger, etc.).
/// The path is usable with [`std::fs`] under WASI (`/mnt/...`). It may change
/// after Save As; call this when you need the current path rather than
/// caching it at startup unless you only care about the file that was loaded.
///
/// Requires the `alloc` feature.
#[cfg(feature = "alloc")]
#[inline]
pub fn get_splits_path() -> Option<alloc::string::String> {
    // SAFETY: The first call uses a null buffer and zero length to learn the
    // required size. If that fails with length 0, the host has no file. Otherwise
    // we allocate a buffer of that length and call again. On success the buffer
    // is filled with valid UTF-8 that is not nul-terminated.
    unsafe {
        let mut len = 0;
        let success = sys::runtime_get_splits_path(core::ptr::null_mut(), &mut len);
        if len == 0 && !success {
            return None;
        }
        let mut buf = alloc::vec::Vec::with_capacity(len);
        let success = sys::runtime_get_splits_path(buf.as_mut_ptr(), &mut len);
        if !success {
            return None;
        }
        buf.set_len(len);
        Some(alloc::string::String::from_utf8_unchecked(buf))
    }
}
