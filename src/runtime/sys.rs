use core::num::NonZeroU64;

use crate::Address;

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

    #[cfg(feature = "alloc")]
    pub fn process_get_path(process: ProcessId, buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool;

    /// Gets the number of memory ranges in a given process.
    pub fn process_get_memory_range_count(process: ProcessId) -> Option<NonZeroU64>;
    /// Gets the start address of a memory range by its index.
    pub fn process_get_memory_range_address(process: ProcessId, idx: u64)
        -> Option<NonZeroAddress>;
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
    pub fn runtime_get_os(buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool;
    /// Stores the name of the architecture that the runtime is running on
    /// in the buffer given. Returns `false` if the buffer is too small.
    /// After this call, no matter whether it was successful or not, the
    /// `buf_len_ptr` will be set to the required buffer size. The name is
    /// guaranteed to be valid UTF-8 and is not nul-terminated.
    /// Example values: `x86`, `x86_64`, `arm`, `aarch64`
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
