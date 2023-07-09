//! This module provides utilities for creating durations.

/// From a frame count and a fixed frame rate, returns an accurate duration.
pub fn frame_count<const FRAME_RATE: u64>(frame_count: u64) -> time::Duration {
    let secs = frame_count / FRAME_RATE;
    let nanos = (frame_count % FRAME_RATE) * 1_000_000_000 / FRAME_RATE;
    time::Duration::new(secs as _, nanos as _)
}

#[cfg(target_os = "wasi")]
mod instant {
    use core::{mem::MaybeUninit, ops::Add, time::Duration};

    use wasi::Timestamp;

    fn current_time() -> Timestamp {
        // SAFETY: This is copied from std, so it should be fine.
        // https://github.com/rust-lang/rust/blob/dd5d7c729d4e8a59708df64002e09dbcbc4005ba/library/std/src/sys/wasi/time.rs#L15
        unsafe {
            let mut rp0 = MaybeUninit::<Timestamp>::uninit();
            let ret = wasi::wasi_snapshot_preview1::clock_time_get(
                wasi::CLOCKID_MONOTONIC.raw() as _,
                1, // precision... seems ignored though?
                rp0.as_mut_ptr() as _,
            );
            assert_eq!(ret, wasi::ERRNO_SUCCESS.raw() as _);
            rp0.assume_init()
        }
    }

    /// A version of the standard library's `Instant` using WASI that doesn't
    /// need the standard library.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(transparent)]
    pub struct Instant(pub(crate) Timestamp);

    impl Instant {
        /// Returns an instant corresponding to "now".
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use asr::time_util::Instant;
        ///
        /// let now = Instant::now();
        /// ```
        pub fn now() -> Self {
            Self(current_time())
        }

        /// Returns the amount of time elapsed from another instant to this one,
        /// or zero duration if that instant is later than this one.
        pub const fn duration_since(&self, other: Self) -> Duration {
            let nanos = self.0.saturating_sub(other.0);
            Duration::new(nanos / 1_000_000_000, (nanos % 1_000_000_000) as _)
        }

        /// Returns the amount of time elapsed since this instant.
        pub fn elapsed(&self) -> Duration {
            Self::now().duration_since(*self)
        }
    }

    impl Add<Duration> for Instant {
        type Output = Self;

        fn add(self, rhs: Duration) -> Self::Output {
            Self(self.0 + rhs.as_nanos() as u64)
        }
    }
}
#[cfg(target_os = "wasi")]
pub use self::instant::Instant;
