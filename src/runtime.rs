use bytemuck::Pod;
use core::{
    mem::{self, MaybeUninit},
    ops::Add,
    slice,
};

pub use self::sys::Address;
use self::sys::ProcessId;

#[cfg(feature = "flags")]
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct MemoryRangeFlags: u64 {
        /// The memory range is readable.
        const READ = 1 << 1;
        /// The memory range is writable.
        const WRITE = 1 << 2;
        /// The memory range is executable.
        const EXECUTE = 1 << 3;
        /// The memory range has a file path.
        const PATH = 1 << 4;
    }
}

mod sys {
    use core::num::NonZeroU64;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct Address(pub u64);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct NonZeroAddress(pub NonZeroU64);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct ProcessId(NonZeroU64);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct TimerState(u32);

    impl TimerState {
        /// The timer is not running.
        pub const NOT_RUNNING: Self = Self(0);
        /// The timer is running.
        pub const RUNNING: Self = Self(1);
        /// The timer started but got paused. This is separate from the game
        /// time being paused. Game time may even always be paused.
        pub const PAUSED: Self = Self(2);
        /// The timer has ended, but didn't get reset yet.
        pub const ENDED: Self = Self(3);
    }

    extern "C" {
        /// Gets the state that the timer currently is in.
        pub fn timer_get_state() -> TimerState;

        /// Starts the timer.
        pub fn timer_start();
        /// Splits the current segment.
        pub fn timer_split();
        /// Resets the timer.
        pub fn timer_reset();
        /// Sets a custom key value pair. This may be arbitrary information that
        /// the auto splitter wants to provide for visualization.
        pub fn timer_set_variable(
            key_ptr: *const u8,
            key_len: usize,
            value_ptr: *const u8,
            value_len: usize,
        );

        /// Sets the game time.
        pub fn timer_set_game_time(secs: i64, nanos: i32);
        /// Pauses the game time. This does not pause the timer, only the
        /// automatic flow of time for the game time.
        pub fn timer_pause_game_time();
        /// Resumes the game time. This does not resume the timer, only the
        /// automatic flow of time for the game time.
        pub fn timer_resume_game_time();

        /// Attaches to a process based on its name.
        pub fn process_attach(name_ptr: *const u8, name_len: usize) -> Option<ProcessId>;
        /// Detaches from a process.
        pub fn process_detach(process: ProcessId);
        /// Checks whether is a process is still open. You should detach from a
        /// process and stop using it if this returns `false`.
        pub fn process_is_open(process: ProcessId) -> bool;
        /// Reads memory from a process at the address given. This will write
        /// the memory to the buffer given. Returns `false` if this fails.
        pub fn process_read(
            process: ProcessId,
            address: Address,
            buf_ptr: *mut u8,
            buf_len: usize,
        ) -> bool;

        /// Gets the address of a module in a process.
        pub fn process_get_module_address(
            process: ProcessId,
            name_ptr: *const u8,
            name_len: usize,
        ) -> Option<NonZeroAddress>;
        /// Gets the size of a module in a process.
        pub fn process_get_module_size(
            process: ProcessId,
            name_ptr: *const u8,
            name_len: usize,
        ) -> Option<NonZeroU64>;

        /// Gets the number of memory ranges in a given process.
        pub fn process_get_memory_range_count(process: ProcessId) -> Option<NonZeroU64>;
        /// Gets the start address of a memory range by its index.
        pub fn process_get_memory_range_address(
            process: ProcessId,
            idx: u64,
        ) -> Option<NonZeroAddress>;
        /// Gets the size of a memory range by its index.
        pub fn process_get_memory_range_size(process: ProcessId, idx: u64) -> Option<NonZeroU64>;
        /// Gets the flags of a memory range by its index.
        #[cfg(feature = "flags")]
        pub fn process_get_memory_range_flags(process: ProcessId, idx: u64) -> Option<NonZeroU64>;

        /// Sets the tick rate of the runtime. This influences the amount of
        /// times the `update` function is called per second.
        pub fn runtime_set_tick_rate(ticks_per_second: f64);
        /// Prints a log message for debugging purposes.
        pub fn runtime_print_message(text_ptr: *const u8, text_len: usize);
        /// Stores the name of the operating system that the runtime is running
        /// on in the buffer given. Returns `false` if the buffer is too small.
        /// After this call, no matter whether it was successful or not, the
        /// `buf_len_ptr` will be set to the required buffer size. The name is
        /// guaranteed to be valid UTF-8 and is not nul-terminated.
        /// Example values: `windows`, `linux`, `macos`
        #[cfg(feature = "arrayvec")]
        pub fn runtime_get_os(buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool;
        /// Stores the name of the architecture that the runtime is running on
        /// in the buffer given. Returns `false` if the buffer is too small.
        /// After this call, no matter whether it was successful or not, the
        /// `buf_len_ptr` will be set to the required buffer size. The name is
        /// guaranteed to be valid UTF-8 and is not nul-terminated.
        /// Example values: `x86`, `x86_64`, `arm`, `aarch64`
        #[cfg(feature = "arrayvec")]
        pub fn runtime_get_arch(buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool;

        /// Adds a new setting that the user can modify. This will return either
        /// the specified default value or the value that the user has set.
        pub fn user_settings_add_bool(
            key_ptr: *const u8,
            key_len: usize,
            description_ptr: *const u8,
            description_len: usize,
            default_value: bool,
        ) -> bool;
    }
}

#[derive(Debug)]
pub struct Error;

#[derive(Debug)]
#[repr(transparent)]
pub struct Process(ProcessId);

impl Drop for Process {
    #[inline]
    fn drop(&mut self) {
        unsafe { sys::process_detach(self.0) }
    }
}

impl Process {
    #[inline]
    pub fn attach(name: &str) -> Option<Self> {
        let id = unsafe { sys::process_attach(name.as_ptr(), name.len()) };
        id.map(Self)
    }

    #[inline]
    pub fn get_module_address(&self, name: &str) -> Result<Address, Error> {
        unsafe {
            let address = sys::process_get_module_address(self.0, name.as_ptr(), name.len());
            if let Some(address) = address {
                Ok(Address(address.0.get()))
            } else {
                Err(Error)
            }
        }
    }

    #[inline]
    pub fn get_module_size(&self, name: &str) -> Result<u64, Error> {
        unsafe {
            let size = sys::process_get_module_size(self.0, name.as_ptr(), name.len());
            if let Some(size) = size {
                Ok(size.get())
            } else {
                Err(Error)
            }
        }
    }

    #[inline]
    pub fn memory_ranges(&self) -> impl DoubleEndedIterator<Item = MemoryRange<'_>> {
        let count = unsafe { sys::process_get_memory_range_count(self.0).map_or(0, |c| c.get()) };
        (0..count).map(|i| MemoryRange(self, i))
    }

    #[inline]
    pub fn read_into_buf(&self, address: Address, buf: &mut [u8]) -> Result<(), Error> {
        unsafe {
            if sys::process_read(self.0, address, buf.as_mut_ptr(), buf.len()) {
                Ok(())
            } else {
                Err(Error)
            }
        }
    }

    #[inline]
    pub fn read_into_uninit_buf<'buf>(
        &self,
        address: Address,
        buf: &'buf mut [MaybeUninit<u8>],
    ) -> Result<&'buf mut [u8], Error> {
        unsafe {
            if sys::process_read(self.0, address, buf.as_mut_ptr().cast(), buf.len()) {
                Ok(slice::from_raw_parts_mut(
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                ))
            } else {
                Err(Error)
            }
        }
    }

    #[inline]
    pub fn read<T: Pod>(&self, address: Address) -> Result<T, Error> {
        unsafe {
            let mut value = MaybeUninit::<T>::uninit();
            self.read_into_uninit_buf(
                address,
                slice::from_raw_parts_mut(value.as_mut_ptr().cast(), mem::size_of::<T>()),
            )?;
            Ok(value.assume_init())
        }
    }

    pub fn read_pointer_path64<T: Pod>(&self, mut address: u64, path: &[u64]) -> Result<T, Error> {
        let (&last, path) = path.split_last().ok_or(Error)?;
        for &offset in path {
            address = self.read(Address(address.wrapping_add(offset)))?;
        }
        self.read(Address(address.wrapping_add(last)))
    }

    pub fn read_pointer_path32<T: Pod>(&self, mut address: u32, path: &[u32]) -> Result<T, Error> {
        let (&last, path) = path.split_last().ok_or(Error)?;
        for &offset in path {
            address = self.read(Address(address.wrapping_add(offset) as u64))?;
        }
        self.read(Address(address.wrapping_add(last) as u64))
    }

    pub fn read_into_slice<T: Pod>(&self, address: Address, slice: &mut [T]) -> Result<(), Error> {
        self.read_into_buf(address, bytemuck::cast_slice_mut(slice))
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        unsafe { sys::process_is_open(self.0) }
    }
}

pub struct MemoryRange<'a>(&'a Process, u64);

impl MemoryRange<'_> {
    #[inline]
    pub fn address(&self) -> Result<Address, Error> {
        unsafe {
            let address = sys::process_get_memory_range_address(self.0 .0, self.1);
            if let Some(address) = address {
                Ok(Address(address.0.get()))
            } else {
                Err(Error)
            }
        }
    }

    #[inline]
    pub fn size(&self) -> Result<u64, Error> {
        unsafe {
            let size = sys::process_get_memory_range_size(self.0 .0, self.1);
            if let Some(size) = size {
                Ok(size.get())
            } else {
                Err(Error)
            }
        }
    }

    #[cfg(feature = "flags")]
    #[inline]
    pub fn flags(&self) -> Result<MemoryRangeFlags, Error> {
        unsafe {
            let flags = sys::process_get_memory_range_flags(self.0 .0, self.1);
            if let Some(flags) = flags {
                Ok(MemoryRangeFlags::from_bits_truncate(flags.get()))
            } else {
                Err(Error)
            }
        }
    }
}

impl From<u32> for Address {
    fn from(addr: u32) -> Self {
        Self(addr as u64)
    }
}

impl From<u64> for Address {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl Add<u32> for Address {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs as u64)
    }
}

impl Add<u64> for Address {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

pub mod timer {
    use super::sys;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum TimerState {
        NotRunning,
        Running,
        Paused,
        Ended,
    }

    #[inline]
    pub fn start() {
        unsafe { sys::timer_start() }
    }

    #[inline]
    pub fn split() {
        unsafe { sys::timer_split() }
    }

    #[inline]
    pub fn reset() {
        unsafe { sys::timer_reset() }
    }

    #[inline]
    pub fn pause_game_time() {
        unsafe { sys::timer_pause_game_time() }
    }

    #[inline]
    pub fn resume_game_time() {
        unsafe { sys::timer_resume_game_time() }
    }

    #[inline]
    pub fn set_variable(key: &str, value: &str) {
        unsafe { sys::timer_set_variable(key.as_ptr(), key.len(), value.as_ptr(), value.len()) }
    }

    #[inline]
    pub fn state() -> TimerState {
        unsafe {
            match sys::timer_get_state() {
                sys::TimerState::NOT_RUNNING => TimerState::NotRunning,
                sys::TimerState::PAUSED => TimerState::Paused,
                sys::TimerState::RUNNING => TimerState::Running,
                sys::TimerState::ENDED => TimerState::Ended,
                _ => core::hint::unreachable_unchecked(),
            }
        }
    }

    #[inline]
    pub fn set_game_time(time: time::Duration) {
        unsafe { sys::timer_set_game_time(time.whole_seconds(), time.subsec_nanoseconds()) }
    }

    #[cfg(feature = "integer-vars")]
    pub fn set_variable_int(key: &str, value: impl itoa::Integer) {
        let mut buf = itoa::Buffer::new();
        set_variable(key, buf.format(value));
    }

    #[cfg(feature = "float-vars")]
    pub fn set_variable_float(key: &str, value: impl ryu::Float) {
        let mut buf = ryu::Buffer::new();
        set_variable(key, buf.format(value));
    }
}

pub mod user_settings {
    use super::sys;

    #[inline]
    pub fn add_bool(key: &str, description: &str, default_value: bool) -> bool {
        unsafe {
            sys::user_settings_add_bool(
                key.as_ptr(),
                key.len(),
                description.as_ptr(),
                description.len(),
                default_value,
            )
        }
    }
}

pub trait Setting {
    fn register(key: &str, description: &str, default_value: Self) -> Self;
}

impl Setting for bool {
    fn register(key: &str, description: &str, default_value: Self) -> Self {
        user_settings::add_bool(key, description, default_value)
    }
}

#[inline]
pub fn set_tick_rate(ticks_per_second: f64) {
    unsafe { sys::runtime_set_tick_rate(ticks_per_second) }
}

#[inline]
pub fn print_message(text: &str) {
    unsafe { sys::runtime_print_message(text.as_ptr(), text.len()) }
}

#[inline]
#[cfg(feature = "strings")]
pub fn get_os() -> Result<arrayvec::ArrayString<16>, Error> {
    let mut buf = arrayvec::ArrayString::<16>::new();
    unsafe {
        let mut len = buf.capacity();
        let success = sys::runtime_get_os(buf.as_mut_ptr(), &mut len);
        if !success {
            return Err(Error);
        }
        buf.set_len(len);
    }
    Ok(buf)
}

#[inline]
#[cfg(feature = "strings")]
pub fn get_arch() -> Result<arrayvec::ArrayString<16>, Error> {
    let mut buf = arrayvec::ArrayString::<16>::new();
    unsafe {
        let mut len = buf.capacity();
        let success = sys::runtime_get_arch(buf.as_mut_ptr(), &mut len);
        if !success {
            return Err(Error);
        }
        buf.set_len(len);
    }
    Ok(buf)
}
