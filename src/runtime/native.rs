//! Native execution support for auto splitters.

use core::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
    time::Duration,
};
use std::{cell::Cell, collections::HashMap, sync::Arc, thread};

use indexmap::IndexMap;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

use super::sys::native::{SettingValueData, SettingsListData, SettingsMapData};

/// The severity of a message produced by the native runtime.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LogLevel {
    /// A trace message.
    Trace,
    /// A debug message.
    Debug,
    /// An informational message.
    Info,
    /// A warning.
    Warning,
    /// An error.
    Error,
}

/// The platform-independent services an auto splitter needs from its host.
///
/// Process interaction, memory reading, and platform information are provided
/// directly by `asr` on native targets. Timer interaction and logging are
/// supplied by the embedding application through this trait.
pub trait Runtime: 'static {
    /// Returns the current state of the timer.
    fn timer_state(&self) -> super::timer::TimerState;
    /// Returns the index of the current split, or [`None`] outside an attempt.
    fn current_split_index(&self) -> Option<u64>;
    /// Returns whether the segment was split or skipped.
    fn segment_splitted(&self, index: u64) -> Option<bool>;
    /// Starts the timer.
    fn start(&mut self);
    /// Splits the current segment.
    fn split(&mut self);
    /// Skips the current split.
    fn skip_split(&mut self);
    /// Undoes the previous split.
    fn undo_split(&mut self);
    /// Resets the timer.
    fn reset(&mut self);
    /// Sets the game time.
    fn set_game_time(&mut self, time: time::Duration);
    /// Pauses game time.
    fn pause_game_time(&mut self);
    /// Resumes game time.
    fn resume_game_time(&mut self);
    /// Sets a custom timer variable.
    fn set_variable(&mut self, key: &str, value: &str);
    /// Logs a message produced by the auto splitter.
    fn log(&mut self, message: fmt::Arguments<'_>);
    /// Logs a message produced by the native support code.
    fn log_runtime(&mut self, message: fmt::Arguments<'_>, level: LogLevel) {
        let _ = level;
        self.log(message);
    }
}

/// A serializable setting value used by a native [`Runner`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SettingValue {
    /// A key-value map.
    Map(SettingsMap),
    /// A list.
    List(SettingsList),
    /// A boolean.
    Bool(bool),
    /// A 64-bit signed integer.
    I64(i64),
    /// A 64-bit floating-point number.
    F64(f64),
    /// A string.
    String(Arc<str>),
}

/// Settings stored for a native auto splitter.
pub type SettingsMap = IndexMap<Arc<str>, SettingValue>;

/// A list nested inside a setting value.
pub type SettingsList = Vec<SettingValue>;

/// A settings widget registered by an auto splitter.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsWidget {
    /// The stable key used in the settings map.
    pub key: Arc<str>,
    /// The description shown to the user.
    pub description: Arc<str>,
    /// An optional tooltip.
    pub tooltip: Option<Arc<str>>,
    /// The kind of widget.
    pub kind: SettingsWidgetKind,
}

/// The kind of a settings widget.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SettingsWidgetKind {
    /// A title used to group settings.
    Title {
        /// The heading level, starting at zero.
        heading_level: u32,
    },
    /// A boolean setting.
    Bool {
        /// The value used when the setting is absent.
        default_value: bool,
    },
    /// A choice setting.
    Choice {
        /// The value used when the setting is absent.
        default_option_key: Arc<str>,
        /// The available choices.
        options: Vec<ChoiceOption>,
    },
    /// A file selection setting.
    FileSelect {
        /// Filters suggested to the frontend.
        filters: Vec<FileFilter>,
    },
}

/// An option in a choice setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceOption {
    /// The stable value stored for this option.
    pub key: Arc<str>,
    /// The description shown to the user.
    pub description: Arc<str>,
}

/// A filter for a file selection setting.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileFilter {
    /// A file-name glob pattern.
    Name {
        /// An optional description.
        description: Option<Arc<str>>,
        /// The glob pattern.
        pattern: Arc<str>,
    },
    /// A MIME type, potentially containing a wildcard.
    MimeType(Arc<str>),
}

pub(super) struct State {
    pub(super) settings_maps: HashMap<u64, SettingsMapData>,
    pub(super) settings_lists: HashMap<u64, SettingsListData>,
    pub(super) setting_values: HashMap<u64, SettingValueData>,
    pub(super) settings: SettingsMapData,
    pub(super) settings_widgets: Vec<SettingsWidget>,
    pub(super) tick_rate: Duration,
    next_handle: u64,
}

impl State {
    fn new(settings: SettingsMap) -> Self {
        Self {
            settings_maps: HashMap::new(),
            settings_lists: HashMap::new(),
            setting_values: HashMap::new(),
            settings: settings.into(),
            settings_widgets: Vec::new(),
            tick_rate: Duration::from_secs_f64(1.0 / 120.0),
            next_handle: 1,
        }
    }

    pub(super) fn insert_handle<T>(next: &mut u64, map: &mut HashMap<u64, T>, value: T) -> u64 {
        let handle = *next;
        *next = next
            .checked_add(1)
            .expect("native asr handle space exhausted");
        map.insert(handle, value);
        handle
    }

    pub(super) fn insert_map(&mut self, value: SettingsMapData) -> u64 {
        Self::insert_handle(&mut self.next_handle, &mut self.settings_maps, value)
    }

    pub(super) fn insert_list(&mut self, value: SettingsListData) -> u64 {
        Self::insert_handle(&mut self.next_handle, &mut self.settings_lists, value)
    }

    pub(super) fn insert_value(&mut self, value: SettingValueData) -> u64 {
        Self::insert_handle(&mut self.next_handle, &mut self.setting_values, value)
    }
}

#[derive(Clone, Copy)]
struct Active {
    runtime: *mut dyn Runtime,
    state: *mut State,
}

thread_local! {
    static ACTIVE: Cell<Option<Active>> = const { Cell::new(None) };
}

pub(super) fn with_active<T>(f: impl FnOnce(&mut dyn Runtime, &mut State) -> T) -> T {
    ACTIVE.with(|active| {
        let active = active
            .get()
            .expect("native asr APIs must be called from inside Runner::tick or Runner::run");
        // SAFETY: Runner installs both pointers only for the duration of the
        // call. Nested installation is rejected, and native execution is
        // confined to this thread.
        unsafe { f(&mut *active.runtime, &mut *active.state) }
    })
}

struct ActiveGuard;

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(None));
    }
}

/// Executes an auto splitter natively with a host-provided [`Runtime`].
pub struct Runner<R> {
    runtime: R,
    state: State,
}

impl<R: Runtime> Runner<R> {
    /// Creates a runner with empty settings.
    pub fn new(runtime: R) -> Self {
        Self::with_settings(runtime, SettingsMap::new())
    }

    /// Creates a runner with previously stored settings.
    pub fn with_settings(runtime: R, settings: SettingsMap) -> Self {
        Self {
            runtime,
            state: State::new(settings),
        }
    }

    /// Executes one synchronous auto-splitter tick.
    pub fn tick(&mut self, update: impl FnOnce()) {
        self.enter(update)
    }

    /// Runs an asynchronous auto splitter to completion.
    ///
    /// A pending future is polled again after the configured tick interval.
    pub fn run(&mut self, future: impl Future<Output = ()>) {
        let mut future = core::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(()) =
                self.enter(|| Future::poll(Pin::as_mut(&mut future), &mut context))
            {
                return;
            }
            thread::sleep(self.state.tick_rate);
        }
    }

    /// Returns the current interval between ticks.
    pub const fn tick_rate(&self) -> Duration {
        self.state.tick_rate
    }

    /// Returns the settings currently stored by the auto splitter.
    pub fn settings(&self) -> SettingsMap {
        self.state.settings.clone().into()
    }

    /// Replaces the settings currently stored by the auto splitter.
    pub fn set_settings(&mut self, settings: SettingsMap) {
        self.state.settings = settings.into();
    }

    /// Returns the settings widgets registered so far.
    pub fn settings_widgets(&self) -> &[SettingsWidget] {
        &self.state.settings_widgets
    }

    /// Accesses the embedding runtime.
    pub const fn runtime(&self) -> &R {
        &self.runtime
    }

    /// Mutably accesses the embedding runtime.
    pub const fn runtime_mut(&mut self) -> &mut R {
        &mut self.runtime
    }

    /// Consumes the runner and returns the embedding runtime.
    pub fn into_runtime(self) -> R {
        self.runtime
    }

    fn enter<T>(&mut self, f: impl FnOnce() -> T) -> T {
        ACTIVE.with(|active| {
            assert!(
                active.get().is_none(),
                "native asr runners cannot be nested on the same thread"
            );
            let runtime: *mut dyn Runtime = &mut self.runtime;
            active.set(Some(Active {
                runtime,
                state: &mut self.state,
            }));
        });
        let _guard = ActiveGuard;
        f()
    }
}

pub(super) fn process_refresh_kind() -> RefreshKind {
    RefreshKind::nothing()
        .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet))
}

pub(super) fn refresh_all(system: &mut System) {
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{future::next_tick, settings, timer, Address, Process, ProcessId};

    #[derive(Default)]
    struct TestRuntime {
        started: bool,
        messages: Vec<String>,
    }

    impl Runtime for TestRuntime {
        fn timer_state(&self) -> timer::TimerState {
            if self.started {
                timer::TimerState::Running
            } else {
                timer::TimerState::NotRunning
            }
        }

        fn current_split_index(&self) -> Option<u64> {
            self.started.then_some(0)
        }

        fn segment_splitted(&self, _: u64) -> Option<bool> {
            None
        }

        fn start(&mut self) {
            self.started = true;
        }

        fn split(&mut self) {}
        fn skip_split(&mut self) {}
        fn undo_split(&mut self) {}

        fn reset(&mut self) {
            self.started = false;
        }

        fn set_game_time(&mut self, _: time::Duration) {}
        fn pause_game_time(&mut self) {}
        fn resume_game_time(&mut self) {}
        fn set_variable(&mut self, _: &str, _: &str) {}

        fn log(&mut self, message: fmt::Arguments<'_>) {
            self.messages.push(message.to_string());
        }
    }

    #[test]
    fn timer_calls_are_forwarded() {
        let mut runner = Runner::new(TestRuntime::default());
        runner.tick(|| {
            assert_eq!(timer::state(), timer::TimerState::NotRunning);
            timer::start();
            assert_eq!(timer::state(), timer::TimerState::Running);
            crate::print_message("hello");
        });
        assert!(runner.runtime().started);
        assert_eq!(runner.runtime().messages, ["hello"]);
    }

    #[test]
    fn settings_survive_across_ticks() {
        let mut runner = Runner::new(TestRuntime::default());
        runner.tick(|| {
            let map = settings::Map::new();
            map.insert("enabled", true);
            map.store();
        });
        runner.tick(|| {
            let map = settings::Map::load();
            assert_eq!(
                map.get("enabled").and_then(|value| value.get_bool()),
                Some(true)
            );
            assert!(settings::gui::add_bool("enabled", "Enabled", false));
        });
        assert_eq!(runner.settings_widgets().len(), 1);
    }

    #[test]
    fn asynchronous_runner_polls_at_tick_boundaries() {
        let mut runner = Runner::new(TestRuntime::default());
        runner.run(async {
            next_tick().await;
            timer::start();
        });
        assert!(runner.runtime().started);
    }

    #[test]
    fn can_read_the_current_process() {
        let process = Process::attach_by_pid(ProcessId(std::process::id().into())).expect("attach");
        let value = 0x1234_5678_9ABC_DEF0_u64;
        let address = Address::new((&value as *const u64) as u64);
        assert_eq!(process.read::<u64>(address).expect("read"), value);
        assert!(process.is_open());
        assert!(!process.get_path().expect("path").is_empty());
    }
}
