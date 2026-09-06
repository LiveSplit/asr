#![allow(clippy::undocumented_unsafe_blocks)]

use core::{num::NonZeroU64, slice, str};
use std::{
    cell::RefCell,
    io,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use indexmap::IndexMap;
use proc_maps::{MapRange, Pid};
use read_process_memory::{CopyAddress, ProcessHandle};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::Address;

use super::{
    NonZeroAddress, Process, ProcessId, SettingValue, SettingValueType, SettingsList, SettingsMap,
    TimerState,
};
use crate::runtime::{
    native::with_active, ChoiceOption, FileFilter, LogLevel, SettingsWidget, SettingsWidgetKind,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SettingsMapData(pub(crate) Arc<IndexMap<Arc<str>, SettingValueData>>);

impl Default for SettingsMapData {
    fn default() -> Self {
        Self(Arc::new(IndexMap::new()))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SettingsListData(pub(crate) Arc<Vec<SettingValueData>>);

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SettingValueData {
    Map(SettingsMapData),
    List(SettingsListData),
    Bool(bool),
    I64(i64),
    F64(f64),
    String(Arc<str>),
}

impl From<crate::runtime::SettingsMap> for SettingsMapData {
    fn from(value: crate::runtime::SettingsMap) -> Self {
        Self(Arc::new(
            value
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
        ))
    }
}

impl From<SettingsMapData> for crate::runtime::SettingsMap {
    fn from(value: SettingsMapData) -> Self {
        value
            .0
            .iter()
            .map(|(key, value)| (key.clone(), value.clone().into()))
            .collect()
    }
}

impl From<crate::runtime::SettingValue> for SettingValueData {
    fn from(value: crate::runtime::SettingValue) -> Self {
        match value {
            crate::runtime::SettingValue::Map(value) => Self::Map(value.into()),
            crate::runtime::SettingValue::List(value) => Self::List(SettingsListData(Arc::new(
                value.into_iter().map(Into::into).collect(),
            ))),
            crate::runtime::SettingValue::Bool(value) => Self::Bool(value),
            crate::runtime::SettingValue::I64(value) => Self::I64(value),
            crate::runtime::SettingValue::F64(value) => Self::F64(value),
            crate::runtime::SettingValue::String(value) => Self::String(value),
        }
    }
}

impl From<SettingValueData> for crate::runtime::SettingValue {
    fn from(value: SettingValueData) -> Self {
        match value {
            SettingValueData::Map(value) => Self::Map(value.into()),
            SettingValueData::List(value) => {
                Self::List(value.0.iter().cloned().map(Into::into).collect::<Vec<_>>())
            }
            SettingValueData::Bool(value) => Self::Bool(value),
            SettingValueData::I64(value) => Self::I64(value),
            SettingValueData::F64(value) => Self::F64(value),
            SettingValueData::String(value) => Self::String(value),
        }
    }
}

struct SendSync<T>(T);

// SAFETY: `read-process-memory`'s handles are plain operating-system process
// handles. Its missing Send/Sync implementations are a known limitation of
// that crate, not a thread-affinity requirement of the underlying handles.
unsafe impl<T> Send for SendSync<T> {}
// SAFETY: See the Send implementation above.
unsafe impl<T> Sync for SendSync<T> {}

pub(crate) struct NativeProcess {
    handle: SendSync<ProcessHandle>,
    pid: u32,
    memory_ranges: Vec<MapRange>,
    next_memory_range_check: Instant,
    next_open_check: Instant,
    path: Option<Arc<str>>,
}

impl NativeProcess {
    fn with_name(name: &str, process_list: &mut ProcessList) -> Option<Self> {
        process_list.refresh();
        let process = process_list
            .processes_by_name(name)
            .max_by_key(|process| (process.start_time(), process.pid().as_u32()))?;
        Self::from_process(process)
    }

    fn with_pid(pid: u32, process_list: &mut ProcessList) -> Option<Self> {
        process_list.refresh();
        Self::from_process(process_list.system.process(sysinfo::Pid::from_u32(pid))?)
    }

    fn from_process(process: &sysinfo::Process) -> Option<Self> {
        let pid = process.pid().as_u32();
        let handle = (pid as Pid).try_into().ok()?;
        let now = Instant::now();
        Some(Self {
            handle: SendSync(handle),
            pid,
            memory_ranges: Vec::new(),
            next_memory_range_check: now,
            next_open_check: now + Duration::from_secs(1),
            path: process
                .exe()
                .map(Path::to_string_lossy)
                .map(|path| Arc::from(path.as_ref())),
        })
    }

    fn is_open(&mut self, process_list: &mut ProcessList) -> bool {
        let now = Instant::now();
        let pid = sysinfo::Pid::from_u32(self.pid);
        if now >= self.next_open_check {
            process_list.system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                true,
                ProcessRefreshKind::nothing(),
            );
            self.next_open_check = now + Duration::from_secs(1);
        }
        process_list.system.process(pid).is_some()
    }

    fn refresh_memory_ranges(&mut self) -> io::Result<()> {
        let now = Instant::now();
        if now >= self.next_memory_range_check {
            self.memory_ranges = proc_maps::get_process_maps(self.pid as Pid)?;
            self.next_memory_range_check = now + Duration::from_secs(1);
        }
        Ok(())
    }

    fn module_ranges<'a>(&'a self, module: &'a str) -> impl Iterator<Item = &'a MapRange> {
        self.memory_ranges
            .iter()
            .filter(move |range| range.filename().is_some_and(|path| path.ends_with(module)))
    }
}

pub(crate) struct ProcessList {
    pub(super) system: System,
    next_check: Instant,
}

thread_local! {
    static PROCESS_LIST: RefCell<ProcessList> = RefCell::new(ProcessList::new());
}

fn with_process_list<T>(f: impl FnOnce(&mut ProcessList) -> T) -> T {
    PROCESS_LIST.with(|process_list| f(&mut process_list.borrow_mut()))
}

impl ProcessList {
    pub(crate) fn new() -> Self {
        Self {
            system: System::new_with_specifics(crate::runtime::native::process_refresh_kind()),
            next_check: Instant::now() + Duration::from_secs(1),
        }
    }

    fn refresh(&mut self) {
        if Instant::now() >= self.next_check {
            crate::runtime::native::refresh_all(&mut self.system);
            self.next_check = Instant::now() + Duration::from_secs(1);
        }
    }

    fn processes_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a sysinfo::Process> {
        let name = name.as_bytes();
        #[cfg(target_os = "linux")]
        let name = &name[..name.len().min(15)];
        self.system
            .processes()
            .values()
            .filter(move |process| process.name().as_encoded_bytes() == name)
    }
}

const fn handle(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("native asr generated a zero handle")
}

fn process_handle(process: NativeProcess) -> Process {
    let process = Box::into_raw(Box::new(process)) as usize as u64;
    Process(handle(process))
}

const unsafe fn process_ref(process: Process) -> &'static NativeProcess {
    // SAFETY: Native process handles are pointers created by `process_handle`
    // and remain allocated until `process_detach` is called.
    unsafe { &*(process.0.get() as usize as *const NativeProcess) }
}

unsafe fn process_mut(process: Process) -> &'static mut NativeProcess {
    // SAFETY: Native process handles are uniquely owned by the public Process
    // wrapper. Calls through that wrapper are synchronous.
    unsafe { &mut *(process.0.get() as usize as *mut NativeProcess) }
}

const unsafe fn text<'a>(ptr: *const u8, len: usize) -> &'a str {
    // SAFETY: Every caller is an implementation of the import API and receives
    // pointers from the safe wrapper, which always passes valid UTF-8 strings.
    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(ptr, len)) }
}

unsafe fn copy_text(value: &str, ptr: *mut u8, len_ptr: *mut usize) -> bool {
    // SAFETY: The caller provides a valid length pointer.
    let capacity = unsafe { *len_ptr };
    // SAFETY: The caller provides a valid length pointer.
    unsafe { *len_ptr = value.len() };
    if capacity < value.len() {
        return false;
    }
    if !value.is_empty() {
        // SAFETY: The wrapper provided a writable buffer with at least
        // `capacity` bytes, which was checked above.
        unsafe { core::ptr::copy_nonoverlapping(value.as_ptr(), ptr, value.len()) };
    }
    true
}

pub unsafe fn timer_get_state() -> TimerState {
    with_active(|runtime, _| match runtime.timer_state() {
        crate::runtime::timer::TimerState::NotRunning => TimerState::NOT_RUNNING,
        crate::runtime::timer::TimerState::Running => TimerState::RUNNING,
        crate::runtime::timer::TimerState::Paused => TimerState::PAUSED,
        crate::runtime::timer::TimerState::Ended => TimerState::ENDED,
        crate::runtime::timer::TimerState::Unknown => TimerState(u32::MAX),
    })
}

pub unsafe fn timer_start() {
    with_active(|runtime, _| runtime.start());
}
pub unsafe fn timer_split() {
    with_active(|runtime, _| runtime.split());
}
pub unsafe fn timer_skip_split() {
    with_active(|runtime, _| runtime.skip_split());
}
pub unsafe fn timer_undo_split() {
    with_active(|runtime, _| runtime.undo_split());
}
pub unsafe fn timer_reset() {
    with_active(|runtime, _| runtime.reset());
}
pub unsafe fn timer_current_split_index() -> i64 {
    with_active(|runtime, _| {
        runtime
            .current_split_index()
            .and_then(|index| index.try_into().ok())
            .unwrap_or(-1)
    })
}
pub unsafe fn timer_segment_splitted(index: u64) -> i32 {
    with_active(|runtime, _| {
        runtime
            .segment_splitted(index)
            .map_or(-1, |value| value as i32)
    })
}
pub unsafe fn timer_set_variable(
    key_ptr: *const u8,
    key_len: usize,
    value_ptr: *const u8,
    value_len: usize,
) {
    let key = unsafe { text(key_ptr, key_len) };
    let value = unsafe { text(value_ptr, value_len) };
    with_active(|runtime, _| runtime.set_variable(key, value));
}
pub unsafe fn timer_set_game_time(secs: i64, nanos: i32) {
    with_active(|runtime, _| runtime.set_game_time(time::Duration::new(secs, nanos)));
}
pub unsafe fn timer_pause_game_time() {
    with_active(|runtime, _| runtime.pause_game_time());
}
pub unsafe fn timer_resume_game_time() {
    with_active(|runtime, _| runtime.resume_game_time());
}

pub unsafe fn process_attach(name_ptr: *const u8, name_len: usize) -> Option<Process> {
    let name = unsafe { text(name_ptr, name_len) };
    with_process_list(|process_list| {
        NativeProcess::with_name(name, process_list).map(process_handle)
    })
}

pub unsafe fn process_attach_by_pid(pid: ProcessId) -> Option<Process> {
    with_process_list(|process_list| {
        NativeProcess::with_pid(pid.0.try_into().ok()?, process_list).map(process_handle)
    })
}

pub unsafe fn process_detach(process: Process) {
    // SAFETY: The handle was allocated by `process_handle`, is uniquely owned,
    // and this function is called exactly once by Process::drop.
    unsafe { drop(Box::from_raw(process.0.get() as usize as *mut NativeProcess)) };
}

pub unsafe fn process_list_by_name(
    name_ptr: *const u8,
    name_len: usize,
    list_ptr: *mut ProcessId,
    list_len_ptr: *mut usize,
) -> bool {
    let name = unsafe { text(name_ptr, name_len) };
    with_process_list(|process_list| {
        process_list.refresh();
        // SAFETY: The wrapper provides a valid length pointer.
        let capacity = unsafe { *list_len_ptr };
        let mut count = 0;
        for process in process_list.processes_by_name(name) {
            if count < capacity {
                // SAFETY: `count` is below the buffer capacity.
                unsafe {
                    list_ptr
                        .add(count)
                        .write(ProcessId(process.pid().as_u32().into()))
                };
            }
            count = count.saturating_add(1);
        }
        // SAFETY: The wrapper provides a valid length pointer.
        unsafe { *list_len_ptr = count };
        true
    })
}

pub unsafe fn process_is_open(process: Process) -> bool {
    let process = unsafe { process_mut(process) };
    with_process_list(|process_list| process.is_open(process_list))
}

pub unsafe fn process_read(
    process: Process,
    address: Address,
    buf_ptr: *mut u8,
    buf_len: usize,
) -> bool {
    let process = unsafe { process_ref(process) };
    // SAFETY: The safe wrapper provides a valid output buffer.
    process
        .handle
        .0
        .copy_address(address.value() as usize, unsafe {
            slice::from_raw_parts_mut(buf_ptr, buf_len)
        })
        .is_ok()
}

pub unsafe fn process_get_module_address(
    process: Process,
    name_ptr: *const u8,
    name_len: usize,
) -> Option<NonZeroAddress> {
    let name = unsafe { text(name_ptr, name_len) };
    let process = unsafe { process_mut(process) };
    process.refresh_memory_ranges().ok()?;
    let address = process.module_ranges(name).next()?.start() as u64;
    Some(NonZeroAddress(NonZeroU64::new(address)?))
}

pub unsafe fn process_get_module_size(
    process: Process,
    name_ptr: *const u8,
    name_len: usize,
) -> Option<NonZeroU64> {
    let name = unsafe { text(name_ptr, name_len) };
    let process = unsafe { process_mut(process) };
    process.refresh_memory_ranges().ok()?;
    NonZeroU64::new(
        process
            .module_ranges(name)
            .map(|range| range.size() as u64)
            .sum(),
    )
}

#[cfg(any(feature = "alloc", not(target_family = "wasm")))]
pub unsafe fn process_get_module_path(
    process: Process,
    name_ptr: *const u8,
    name_len: usize,
    buf_ptr: *mut u8,
    buf_len_ptr: *mut usize,
) -> bool {
    let name = unsafe { text(name_ptr, name_len) };
    let process = unsafe { process_mut(process) };
    if process.refresh_memory_ranges().is_err() {
        unsafe { *buf_len_ptr = 0 };
        return false;
    }
    let path: Option<String> = process
        .module_ranges(name)
        .next()
        .and_then(MapRange::filename)
        .map(|path| Path::new(path).to_string_lossy().into_owned());
    let Some(path) = path else {
        unsafe { *buf_len_ptr = 0 };
        return false;
    };
    unsafe { copy_text(&path, buf_ptr, buf_len_ptr) }
}

#[cfg(any(feature = "alloc", not(target_family = "wasm")))]
pub unsafe fn process_get_path(
    process: Process,
    buf_ptr: *mut u8,
    buf_len_ptr: *mut usize,
) -> bool {
    let process = unsafe { process_ref(process) };
    let Some(path) = process.path.as_deref() else {
        unsafe { *buf_len_ptr = 0 };
        return false;
    };
    unsafe { copy_text(path, buf_ptr, buf_len_ptr) }
}

pub unsafe fn process_get_memory_range_count(process: Process) -> Option<NonZeroU64> {
    let process = unsafe { process_mut(process) };
    process.refresh_memory_ranges().ok()?;
    NonZeroU64::new(process.memory_ranges.len() as u64)
}

pub unsafe fn process_get_memory_range_address(
    process: Process,
    index: u64,
) -> Option<NonZeroAddress> {
    let process = unsafe { process_mut(process) };
    process.refresh_memory_ranges().ok()?;
    let address = process
        .memory_ranges
        .get(usize::try_from(index).ok()?)?
        .start() as u64;
    Some(NonZeroAddress(NonZeroU64::new(address)?))
}

pub unsafe fn process_get_memory_range_size(process: Process, index: u64) -> Option<NonZeroU64> {
    let process = unsafe { process_mut(process) };
    process.refresh_memory_ranges().ok()?;
    NonZeroU64::new(
        process
            .memory_ranges
            .get(usize::try_from(index).ok()?)?
            .size() as u64,
    )
}

#[cfg(feature = "flags")]
pub unsafe fn process_get_memory_range_flags(process: Process, index: u64) -> Option<NonZeroU64> {
    let process = unsafe { process_mut(process) };
    process.refresh_memory_ranges().ok()?;
    let range = process.memory_ranges.get(usize::try_from(index).ok()?)?;
    let mut flags = 1;
    flags |= (range.is_read() as u64) << 1;
    flags |= (range.is_write() as u64) << 2;
    flags |= (range.is_exec() as u64) << 3;
    flags |= (range.filename().is_some() as u64) << 4;
    NonZeroU64::new(flags)
}

pub unsafe fn runtime_set_tick_rate(ticks_per_second: f64) {
    with_active(|runtime, state| {
        if ticks_per_second.is_finite() && ticks_per_second > 0.0 {
            state.tick_rate = Duration::from_secs_f64(ticks_per_second.recip());
            runtime.log_runtime(
                format_args!("New tick rate: {ticks_per_second}"),
                LogLevel::Debug,
            );
        } else {
            runtime.log_runtime(format_args!("Invalid tick rate"), LogLevel::Warning);
        }
    });
}

pub unsafe fn runtime_print_message(text_ptr: *const u8, text_len: usize) {
    let message = unsafe { text(text_ptr, text_len) };
    with_active(|runtime, _| runtime.log(format_args!("{message}")));
}

pub unsafe fn runtime_get_os(buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool {
    unsafe { copy_text(std::env::consts::OS, buf_ptr, buf_len_ptr) }
}

pub unsafe fn runtime_get_arch(buf_ptr: *mut u8, buf_len_ptr: *mut usize) -> bool {
    unsafe { copy_text(std::env::consts::ARCH, buf_ptr, buf_len_ptr) }
}

pub unsafe fn user_settings_add_bool(
    key_ptr: *const u8,
    key_len: usize,
    description_ptr: *const u8,
    description_len: usize,
    default_value: bool,
) -> bool {
    let key: Arc<str> = Arc::from(unsafe { text(key_ptr, key_len) });
    let description = Arc::from(unsafe { text(description_ptr, description_len) });
    with_active(|_, state| {
        let value = match state.settings.0.get(&key) {
            Some(SettingValueData::Bool(value)) => *value,
            _ => default_value,
        };
        state.settings_widgets.push(SettingsWidget {
            key,
            description,
            tooltip: None,
            kind: SettingsWidgetKind::Bool { default_value },
        });
        value
    })
}

pub unsafe fn user_settings_add_title(
    key_ptr: *const u8,
    key_len: usize,
    description_ptr: *const u8,
    description_len: usize,
    heading_level: u32,
) {
    let key = Arc::from(unsafe { text(key_ptr, key_len) });
    let description = Arc::from(unsafe { text(description_ptr, description_len) });
    with_active(|_, state| {
        state.settings_widgets.push(SettingsWidget {
            key,
            description,
            tooltip: None,
            kind: SettingsWidgetKind::Title { heading_level },
        });
    });
}

pub unsafe fn user_settings_add_choice(
    key_ptr: *const u8,
    key_len: usize,
    description_ptr: *const u8,
    description_len: usize,
    default_option_key_ptr: *const u8,
    default_option_key_len: usize,
) {
    let key = Arc::from(unsafe { text(key_ptr, key_len) });
    let description = Arc::from(unsafe { text(description_ptr, description_len) });
    let default_option_key =
        Arc::from(unsafe { text(default_option_key_ptr, default_option_key_len) });
    with_active(|_, state| {
        state.settings_widgets.push(SettingsWidget {
            key,
            description,
            tooltip: None,
            kind: SettingsWidgetKind::Choice {
                default_option_key,
                options: Vec::new(),
            },
        });
    });
}

pub unsafe fn user_settings_add_choice_option(
    key_ptr: *const u8,
    key_len: usize,
    option_key_ptr: *const u8,
    option_key_len: usize,
    option_description_ptr: *const u8,
    option_description_len: usize,
) -> bool {
    let key = unsafe { text(key_ptr, key_len) };
    let option_key: Arc<str> = Arc::from(unsafe { text(option_key_ptr, option_key_len) });
    let description = Arc::from(unsafe { text(option_description_ptr, option_description_len) });
    with_active(|_, state| {
        let Some(widget) = state
            .settings_widgets
            .iter_mut()
            .find(|widget| widget.key.as_ref() == key)
        else {
            return false;
        };
        let SettingsWidgetKind::Choice {
            default_option_key,
            options,
        } = &mut widget.kind
        else {
            return false;
        };
        let selected = match state.settings.0.get(key) {
            Some(SettingValueData::String(selected)) => selected == &option_key,
            _ => default_option_key == &option_key,
        };
        options.push(ChoiceOption {
            key: option_key,
            description,
        });
        selected
    })
}

pub unsafe fn user_settings_add_file_select(
    key_ptr: *const u8,
    key_len: usize,
    description_ptr: *const u8,
    description_len: usize,
) {
    let key = Arc::from(unsafe { text(key_ptr, key_len) });
    let description = Arc::from(unsafe { text(description_ptr, description_len) });
    with_active(|_, state| {
        state.settings_widgets.push(SettingsWidget {
            key,
            description,
            tooltip: None,
            kind: SettingsWidgetKind::FileSelect {
                filters: Vec::new(),
            },
        });
    });
}

pub unsafe fn user_settings_add_file_select_name_filter(
    key_ptr: *const u8,
    key_len: usize,
    description_ptr: *const u8,
    description_len: usize,
    pattern_ptr: *const u8,
    pattern_len: usize,
) {
    let key = unsafe { text(key_ptr, key_len) };
    let description = if description_ptr.is_null() {
        None
    } else {
        Some(Arc::from(unsafe { text(description_ptr, description_len) }))
    };
    let pattern = Arc::from(unsafe { text(pattern_ptr, pattern_len) });
    with_active(|_, state| {
        let Some(SettingsWidget {
            kind: SettingsWidgetKind::FileSelect { filters },
            ..
        }) = state
            .settings_widgets
            .iter_mut()
            .find(|widget| widget.key.as_ref() == key)
        else {
            return;
        };
        filters.push(FileFilter::Name {
            description,
            pattern,
        });
    });
}

pub unsafe fn user_settings_add_file_select_mime_filter(
    key_ptr: *const u8,
    key_len: usize,
    mime_type_ptr: *const u8,
    mime_type_len: usize,
) {
    let key = unsafe { text(key_ptr, key_len) };
    let mime_type = Arc::from(unsafe { text(mime_type_ptr, mime_type_len) });
    with_active(|_, state| {
        let Some(SettingsWidget {
            kind: SettingsWidgetKind::FileSelect { filters },
            ..
        }) = state
            .settings_widgets
            .iter_mut()
            .find(|widget| widget.key.as_ref() == key)
        else {
            return;
        };
        filters.push(FileFilter::MimeType(mime_type));
    });
}

pub unsafe fn user_settings_set_tooltip(
    key_ptr: *const u8,
    key_len: usize,
    tooltip_ptr: *const u8,
    tooltip_len: usize,
) {
    let key = unsafe { text(key_ptr, key_len) };
    let tooltip = Arc::from(unsafe { text(tooltip_ptr, tooltip_len) });
    with_active(|_, state| {
        if let Some(widget) = state
            .settings_widgets
            .iter_mut()
            .find(|widget| widget.key.as_ref() == key)
        {
            widget.tooltip = Some(tooltip);
        }
    });
}

pub unsafe fn settings_map_new() -> SettingsMap {
    with_active(|_, state| SettingsMap(handle(state.insert_map(SettingsMapData::default()))))
}
pub unsafe fn settings_map_free(map: SettingsMap) {
    with_active(|_, state| {
        state.settings_maps.remove(&map.0.get());
    });
}
pub unsafe fn settings_map_load() -> SettingsMap {
    with_active(|_, state| SettingsMap(handle(state.insert_map(state.settings.clone()))))
}
pub unsafe fn settings_map_store(map: SettingsMap) {
    with_active(|_, state| {
        if let Some(map) = state.settings_maps.get(&map.0.get()) {
            state.settings = map.clone();
        }
    });
}
pub unsafe fn settings_map_store_if_unchanged(old_map: SettingsMap, new_map: SettingsMap) -> bool {
    with_active(|_, state| {
        let Some(old_map) = state.settings_maps.get(&old_map.0.get()) else {
            return false;
        };
        if !Arc::ptr_eq(&old_map.0, &state.settings.0) {
            return false;
        }
        let Some(new_map) = state.settings_maps.get(&new_map.0.get()) else {
            return false;
        };
        state.settings = new_map.clone();
        true
    })
}
pub unsafe fn settings_map_copy(map: SettingsMap) -> SettingsMap {
    with_active(|_, state| {
        let map = state.settings_maps[&map.0.get()].clone();
        SettingsMap(handle(state.insert_map(map)))
    })
}
pub unsafe fn settings_map_insert(
    map: SettingsMap,
    key_ptr: *const u8,
    key_len: usize,
    value: SettingValue,
) {
    let key = Arc::from(unsafe { text(key_ptr, key_len) });
    with_active(|_, state| {
        let Some(value) = state.setting_values.get(&value.0.get()).cloned() else {
            return;
        };
        if let Some(map) = state.settings_maps.get_mut(&map.0.get()) {
            Arc::make_mut(&mut map.0).insert(key, value);
        }
    });
}
pub unsafe fn settings_map_get(
    map: SettingsMap,
    key_ptr: *const u8,
    key_len: usize,
) -> Option<SettingValue> {
    let key = unsafe { text(key_ptr, key_len) };
    with_active(|_, state| {
        let value = state.settings_maps.get(&map.0.get())?.0.get(key)?.clone();
        Some(SettingValue(handle(state.insert_value(value))))
    })
}
pub unsafe fn settings_map_len(map: SettingsMap) -> u64 {
    with_active(|_, state| {
        state
            .settings_maps
            .get(&map.0.get())
            .map_or(0, |map| map.0.len() as u64)
    })
}
pub unsafe fn settings_map_get_key_by_index(
    map: SettingsMap,
    index: u64,
    buf_ptr: *mut u8,
    buf_len_ptr: *mut usize,
) -> bool {
    with_active(|_, state| {
        let Some(key) = state
            .settings_maps
            .get(&map.0.get())
            .and_then(|map| map.0.get_index(usize::try_from(index).ok()?))
            .map(|(key, _)| key.as_ref())
        else {
            unsafe { *buf_len_ptr = 0 };
            return false;
        };
        unsafe { copy_text(key, buf_ptr, buf_len_ptr) }
    })
}
pub unsafe fn settings_map_get_value_by_index(
    map: SettingsMap,
    index: u64,
) -> Option<SettingValue> {
    with_active(|_, state| {
        let value = state
            .settings_maps
            .get(&map.0.get())?
            .0
            .get_index(usize::try_from(index).ok()?)?
            .1
            .clone();
        Some(SettingValue(handle(state.insert_value(value))))
    })
}

pub unsafe fn settings_list_new() -> SettingsList {
    with_active(|_, state| SettingsList(handle(state.insert_list(SettingsListData::default()))))
}
pub unsafe fn settings_list_free(list: SettingsList) {
    with_active(|_, state| {
        state.settings_lists.remove(&list.0.get());
    });
}
pub unsafe fn settings_list_copy(list: SettingsList) -> SettingsList {
    with_active(|_, state| {
        let list = state.settings_lists[&list.0.get()].clone();
        SettingsList(handle(state.insert_list(list)))
    })
}
pub unsafe fn settings_list_len(list: SettingsList) -> u64 {
    with_active(|_, state| {
        state
            .settings_lists
            .get(&list.0.get())
            .map_or(0, |list| list.0.len() as u64)
    })
}
pub unsafe fn settings_list_get(list: SettingsList, index: u64) -> Option<SettingValue> {
    with_active(|_, state| {
        let value = state
            .settings_lists
            .get(&list.0.get())?
            .0
            .get(usize::try_from(index).ok()?)?
            .clone();
        Some(SettingValue(handle(state.insert_value(value))))
    })
}
pub unsafe fn settings_list_push(list: SettingsList, value: SettingValue) {
    with_active(|_, state| {
        let Some(value) = state.setting_values.get(&value.0.get()).cloned() else {
            return;
        };
        if let Some(list) = state.settings_lists.get_mut(&list.0.get()) {
            Arc::make_mut(&mut list.0).push(value);
        }
    });
}
pub unsafe fn settings_list_insert(list: SettingsList, index: u64, value: SettingValue) -> bool {
    with_active(|_, state| {
        let Some(value) = state.setting_values.get(&value.0.get()).cloned() else {
            return false;
        };
        let Some(list) = state.settings_lists.get_mut(&list.0.get()) else {
            return false;
        };
        let Ok(index) = usize::try_from(index) else {
            return false;
        };
        let list = Arc::make_mut(&mut list.0);
        if index > list.len() {
            return false;
        }
        list.insert(index, value);
        true
    })
}

pub unsafe fn setting_value_new_map(map: SettingsMap) -> SettingValue {
    with_active(|_, state| {
        let value = SettingValueData::Map(state.settings_maps[&map.0.get()].clone());
        SettingValue(handle(state.insert_value(value)))
    })
}
pub unsafe fn setting_value_new_list(list: SettingsList) -> SettingValue {
    with_active(|_, state| {
        let value = SettingValueData::List(state.settings_lists[&list.0.get()].clone());
        SettingValue(handle(state.insert_value(value)))
    })
}
pub unsafe fn setting_value_new_bool(value: bool) -> SettingValue {
    with_active(|_, state| SettingValue(handle(state.insert_value(SettingValueData::Bool(value)))))
}
pub unsafe fn setting_value_new_i64(value: i64) -> SettingValue {
    with_active(|_, state| SettingValue(handle(state.insert_value(SettingValueData::I64(value)))))
}
pub unsafe fn setting_value_new_f64(value: f64) -> SettingValue {
    with_active(|_, state| SettingValue(handle(state.insert_value(SettingValueData::F64(value)))))
}
pub unsafe fn setting_value_new_string(value_ptr: *const u8, value_len: usize) -> SettingValue {
    let value = Arc::from(unsafe { text(value_ptr, value_len) });
    with_active(|_, state| {
        SettingValue(handle(state.insert_value(SettingValueData::String(value))))
    })
}
pub unsafe fn setting_value_free(value: SettingValue) {
    with_active(|_, state| {
        state.setting_values.remove(&value.0.get());
    });
}
pub unsafe fn setting_value_copy(value: SettingValue) -> SettingValue {
    with_active(|_, state| {
        let value = state.setting_values[&value.0.get()].clone();
        SettingValue(handle(state.insert_value(value)))
    })
}
pub unsafe fn setting_value_get_type(value: SettingValue) -> SettingValueType {
    with_active(|_, state| match state.setting_values.get(&value.0.get()) {
        Some(SettingValueData::Map(_)) => SettingValueType::MAP,
        Some(SettingValueData::List(_)) => SettingValueType::LIST,
        Some(SettingValueData::Bool(_)) => SettingValueType::BOOL,
        Some(SettingValueData::I64(_)) => SettingValueType::I64,
        Some(SettingValueData::F64(_)) => SettingValueType::F64,
        Some(SettingValueData::String(_)) => SettingValueType::STRING,
        None => SettingValueType(u32::MAX),
    })
}
pub unsafe fn setting_value_get_map(value: SettingValue, out: *mut SettingsMap) -> bool {
    with_active(|_, state| {
        let Some(SettingValueData::Map(map)) = state.setting_values.get(&value.0.get()).cloned()
        else {
            return false;
        };
        // SAFETY: The wrapper provides a valid output pointer.
        unsafe { out.write(SettingsMap(handle(state.insert_map(map)))) };
        true
    })
}
pub unsafe fn setting_value_get_list(value: SettingValue, out: *mut SettingsList) -> bool {
    with_active(|_, state| {
        let Some(SettingValueData::List(list)) = state.setting_values.get(&value.0.get()).cloned()
        else {
            return false;
        };
        // SAFETY: The wrapper provides a valid output pointer.
        unsafe { out.write(SettingsList(handle(state.insert_list(list)))) };
        true
    })
}
pub unsafe fn setting_value_get_bool(value: SettingValue, out: *mut bool) -> bool {
    with_active(|_, state| {
        let Some(SettingValueData::Bool(value)) = state.setting_values.get(&value.0.get()) else {
            return false;
        };
        unsafe { out.write(*value) };
        true
    })
}
pub unsafe fn setting_value_get_i64(value: SettingValue, out: *mut i64) -> bool {
    with_active(|_, state| {
        let Some(SettingValueData::I64(value)) = state.setting_values.get(&value.0.get()) else {
            return false;
        };
        unsafe { out.write(*value) };
        true
    })
}
pub unsafe fn setting_value_get_f64(value: SettingValue, out: *mut f64) -> bool {
    with_active(|_, state| {
        let Some(SettingValueData::F64(value)) = state.setting_values.get(&value.0.get()) else {
            return false;
        };
        unsafe { out.write(*value) };
        true
    })
}
pub unsafe fn setting_value_get_string(
    value: SettingValue,
    buf_ptr: *mut u8,
    buf_len_ptr: *mut usize,
) -> bool {
    with_active(|_, state| {
        let Some(SettingValueData::String(value)) = state.setting_values.get(&value.0.get()) else {
            unsafe { *buf_len_ptr = 0 };
            return false;
        };
        unsafe { copy_text(value, buf_ptr, buf_len_ptr) }
    })
}
