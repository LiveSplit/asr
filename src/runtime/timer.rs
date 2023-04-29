//! This module provides functions for interacting with the timer.

use super::sys;

/// The state of the timer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimerState {
    /// The timer is not running.
    NotRunning,
    /// The timer is running.
    Running,
    /// The timer started but got paused. This is separate from the game time
    /// being paused. Game time may even always be paused.
    Paused,
    /// The timer has ended, but didn't get reset yet.
    Ended,
    /// The timer is in an unknown state.
    Unknown,
}

/// Starts the timer.
#[inline]
pub fn start() {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::timer_start() }
}

/// Splits the current segment.
#[inline]
pub fn split() {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::timer_split() }
}

/// Resets the timer.
#[inline]
pub fn reset() {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::timer_reset() }
}

/// Pauses the game time. This does not pause the timer, only the
/// automatic flow of time for the game time.
#[inline]
pub fn pause_game_time() {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::timer_pause_game_time() }
}

/// Resumes the game time. This does not resume the timer, only the
/// automatic flow of time for the game time.
#[inline]
pub fn resume_game_time() {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::timer_resume_game_time() }
}

/// Sets a custom key value pair. This may be arbitrary information that the
/// auto splitter wants to provide for visualization.
#[inline]
pub fn set_variable(key: &str, value: &str) {
    // SAFETY: We provide a valid pointer and length to both the key and value
    // that are both UTF-8 encoded.
    unsafe { sys::timer_set_variable(key.as_ptr(), key.len(), value.as_ptr(), value.len()) }
}

/// Sets a custom key value pair where the value is an integer. This may be
/// arbitrary information that the auto splitter wants to provide for
/// visualization.
#[cfg(feature = "integer-vars")]
pub fn set_variable_int(key: &str, value: impl itoa::Integer) {
    let mut buf = itoa::Buffer::new();
    set_variable(key, buf.format(value));
}

/// Sets a custom key value pair where the value is a floating point number.
/// This may be arbitrary information that the auto splitter wants to provide
/// for visualization.
#[cfg(feature = "float-vars")]
pub fn set_variable_float(key: &str, value: impl ryu::Float) {
    let mut buf = ryu::Buffer::new();
    set_variable(key, buf.format(value));
}

/// Gets the state that the timer currently is in.
#[inline]
pub fn state() -> TimerState {
    // SAFETY: It is always safe to call this function.
    unsafe {
        match sys::timer_get_state() {
            sys::TimerState::NOT_RUNNING => TimerState::NotRunning,
            sys::TimerState::PAUSED => TimerState::Paused,
            sys::TimerState::RUNNING => TimerState::Running,
            sys::TimerState::ENDED => TimerState::Ended,
            _ => TimerState::Unknown,
        }
    }
}

/// Sets the game time.
#[inline]
pub fn set_game_time(time: time::Duration) {
    // SAFETY: It is always safe to call this function.
    unsafe { sys::timer_set_game_time(time.whole_seconds(), time.subsec_nanoseconds()) }
}
