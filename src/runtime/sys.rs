use core::num::NonZeroU64;

use crate::Address;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct NonZeroAddress(pub NonZeroU64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Process(NonZeroU64);

/// A process id is a unique identifier for a process. It is not guaranteed to
/// be the same across multiple runs of the same process. It is only guaranteed
/// to be unique for the duration of the process. This matches the operating
/// system's definition of a process id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ProcessId(pub u64);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct SettingsMap(NonZeroU64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct SettingValue(NonZeroU64);

extern "C" {
    /// Gets the state that the timer currently is in.
    pub fn timer_get_state() -> TimerState;

    /// Starts the timer.
    pub fn timer_start();
    /// Splits the current segment.
    pub fn timer_split();
    /// Skips the current split.
    pub fn timer_skip_split();
    /// Undoes the previous split.
    pub fn timer_undo_split();
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
    pub fn process_attach(name_ptr: *const u8, name_len: usize) -> Option<Process>;
    /// Attaches to a process based on its process id.
    pub fn process_attach_by_pid(pid: ProcessId) -> Option<Process>;
    /// Detaches from a process.
    pub fn process_detach(process: Process);
    /// Lists processes based on their name. Returns `false` if listing the
    /// processes failed. If it was successful, the buffer is now filled
    /// with the process ids. They are in no specific order. The
    /// `list_len_ptr` will be updated to the amount of process ids that
    /// were found. If this is larger than the original value provided, the
    /// buffer provided was too small and not all process ids could be
    /// stored. This is still considered successful and can optionally be
    /// treated as an error condition by the caller by checking if the
    /// length increased and potentially reallocating a larger buffer. If
    /// the length decreased after the call, the buffer was larger than
    /// needed and the remaining entries are untouched.
    pub fn process_list_by_name(
        name_ptr: *const u8,
        name_len: usize,
        list_ptr: *mut ProcessId,
        list_len_ptr: *mut usize,
    ) -> bool;
    /// Checks whether is a process is still open. You should detach from a
    /// process and stop using it if this returns `false`.
    pub fn process_is_open(process: Process) -> bool;
    /// Reads memory from a process at the address given. This will write
    /// the memory to the buffer given. Returns `false` if this fails.
    pub fn process_read(
        process: Process,
        address: Address,
        buf_ptr: *mut u8,
        buf_len: usize,
    ) -> bool;

    /// Gets the address of a module in a process.
    pub fn process_get_module_address(
        process: Process,
        name_ptr: *const u8,
        name_len: usize,
    ) -> Option<NonZeroAddress>;
    /// Gets the size of a module in a process.
    pub fn process_get_module_size(
        process: Process,
        name_ptr: *const u8,
        name_len: usize,
    ) -> Option<NonZeroU64>;

    #[cfg(feature = "alloc")]
    pub fn process_get_path(process: Process, buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool;

    /// Gets the number of memory ranges in a given process.
    pub fn process_get_memory_range_count(process: Process) -> Option<NonZeroU64>;
    /// Gets the start address of a memory range by its index.
    pub fn process_get_memory_range_address(process: Process, idx: u64) -> Option<NonZeroAddress>;
    /// Gets the size of a memory range by its index.
    pub fn process_get_memory_range_size(process: Process, idx: u64) -> Option<NonZeroU64>;
    /// Gets the flags of a memory range by its index.
    #[cfg(feature = "flags")]
    pub fn process_get_memory_range_flags(process: Process, idx: u64) -> Option<NonZeroU64>;

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

    /// Adds a new boolean setting that the user can modify. This will return
    /// either the specified default value or the value that the user has set.
    /// The key is used to store the setting and needs to be unique across all
    /// types of settings.
    pub fn user_settings_add_bool(
        key_ptr: *const u8,
        key_len: usize,
        description_ptr: *const u8,
        description_len: usize,
        default_value: bool,
    ) -> bool;
    /// Adds a new title to the user settings. This is used to group settings
    /// together. The heading level determines the size of the title. The top
    /// level titles use a heading level of 0. The key needs to be unique across
    /// all types of settings.
    pub fn user_settings_add_title(
        key_ptr: *const u8,
        key_len: usize,
        description_ptr: *const u8,
        description_len: usize,
        heading_level: u32,
    );
    /// Adds a tooltip to a setting based on its key. A tooltip is useful for
    /// explaining the purpose of a setting to the user.
    pub fn user_settings_set_tooltip(
        key_ptr: *const u8,
        key_len: usize,
        tooltip_ptr: *const u8,
        tooltip_len: usize,
    );

    /// Creates a new settings map. You own the settings map and are responsible
    /// for freeing it.
    pub fn settings_map_new() -> SettingsMap;
    /// Frees a settings map.
    pub fn settings_map_free(map: SettingsMap);
    /// Loads a copy of the currently set global settings map. Any changes to it
    /// are only perceived if it's stored back. You own the settings map and are
    /// responsible for freeing it.
    pub fn settings_map_load() -> SettingsMap;
    /// Stores a copy of the settings map as the new global settings map. This
    /// will overwrite the previous global settings map. You still retain
    /// ownership of the map, which means you still need to free it. There's a
    /// chance that the settings map was changed in the meantime, so those
    /// changes could get lost. Prefer using `settings_map_store_if_unchanged`
    /// if you want to avoid that.
    pub fn settings_map_store(map: SettingsMap);
    /// Stores a copy of the new settings map as the new global settings map if
    /// the map has not changed in the meantime. This is done by comparing the
    /// old map. You still retain ownership of both maps, which means you still
    /// need to free them. Returns `true` if the map was stored successfully.
    /// Returns `false` if the map was changed in the meantime.
    pub fn settings_map_store_if_unchanged(old_map: SettingsMap, new_map: SettingsMap) -> bool;
    /// Copies a settings map. No changes inside the copy affect the original
    /// settings map. You own the new settings map and are responsible for
    /// freeing it.
    pub fn settings_map_copy(map: SettingsMap) -> SettingsMap;
    /// Inserts a copy of the setting value into the settings map based on the
    /// key. If the key already exists, it will be overwritten. You still retain
    /// ownership of the setting value, which means you still need to free it.
    pub fn settings_map_insert(
        map: SettingsMap,
        key_ptr: *const u8,
        key_len: usize,
        value: SettingValue,
    );
    /// Gets a copy of the setting value from the settings map based on the key.
    /// Returns `None` if the key does not exist. Any changes to it are only
    /// perceived if it's stored back. You own the setting value and are
    /// responsible for freeing it.
    pub fn settings_map_get(
        map: SettingsMap,
        key_ptr: *const u8,
        key_len: usize,
    ) -> Option<SettingValue>;

    /// Creates a new boolean setting value. You own the setting value and are
    /// responsible for freeing it.
    pub fn setting_value_new_bool(value: bool) -> SettingValue;
    /// Frees a setting value.
    pub fn setting_value_free(value: SettingValue);
    /// Gets the value of a boolean setting value by storing it into the pointer
    /// provided. Returns `false` if the setting value is not a boolean. No
    /// value is stored into the pointer in that case. No matter what happens,
    /// you still retain ownership of the setting value, which means you still
    /// need to free it.
    pub fn setting_value_get_bool(value: SettingValue, value_ptr: *mut bool) -> bool;
}
