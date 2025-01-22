use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::time_util::Instant;

/// A type that provides futures that resolve in fixed intervals.
///
/// # Example
///
/// ```no_run
/// let mut interval = interval(Duration::from_secs(1));
/// loop {
///     interval.tick().await;
///     print_message("A second has passed!");
/// }
/// ```
pub struct Interval {
    next: u64,
    duration: u64,
}

impl Interval {
    /// Returns a future that resolves in the next interval.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut interval = interval(Duration::from_secs(1));
    /// loop {
    ///     interval.tick().await;
    ///     print_message("A second has passed!");
    /// }
    /// ```
    pub fn tick(&mut self) -> Sleep {
        let next = self.next;
        self.next += self.duration;
        Sleep(next)
    }
}

/// A type that provides futures that resolve in fixed intervals.
///
/// # Example
///
/// ```no_run
/// let mut interval = interval(Duration::from_secs(1));
/// loop {
///     interval.tick().await;
///     print_message("A second has passed!");
/// }
/// ```
pub fn interval(duration: Duration) -> Interval {
    let duration = duration.as_nanos() as u64;
    Interval {
        next: Instant::now().0 + duration,
        duration,
    }
}

/// A future that yields back to the runtime for a certain amount of time and
/// then resolves once the time has passed.
///
/// # Example
///
/// ```no_run
/// sleep(Duration::from_secs(1)).await;
/// print_message("A second has passed!");
/// ```
#[must_use = "You need to await this future."]
pub struct Sleep(u64);

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now().0 < self.0 {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/// A future that yields back to the runtime for a certain amount of time and
/// then resolves once the time has passed.
///
/// # Example
///
/// ```no_run
/// sleep(Duration::from_secs(1)).await;
/// print_message("A second has passed!");
/// ```
pub fn sleep(duration: Duration) -> Sleep {
    Sleep(Instant::now().0 + duration.as_nanos() as u64)
}

/// A future that resolves to [`None`] after a certain amount of time, if the
/// provided future has not resolved yet.
///
/// # Example
///
/// ```no_run
/// let future = async {
///    // do some work
/// };
///
/// let result = timeout(Duration::from_secs(1), future).await;
/// if let Some(result) = result {
///    // do something with the result
/// } else {
///   // the future timed out
/// }
/// ```
#[must_use = "You need to await this future."]
pub struct Timeout<F> {
    sleep: Sleep,
    future: F,
}

impl<F: Future> Future for Timeout<F> {
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We are simply projecting the Pin to the inner futures.
        unsafe {
            let this = self.get_unchecked_mut();
            if let Poll::Ready(()) = Pin::new_unchecked(&mut this.sleep).poll(cx) {
                return Poll::Ready(None);
            }
            Pin::new_unchecked(&mut this.future).poll(cx).map(Some)
        }
    }
}

/// A future that resolves to [`None`] after a certain amount of time, if the
/// provided future has not resolved yet.
///
/// # Example
///
/// ```no_run
/// let future = async {
///    // do some work
/// };
///
/// let result = timeout(Duration::from_secs(1), future).await;
/// if let Some(result) = result {
///    // do something with the result
/// } else {
///   // the future timed out
/// }
/// ```
pub fn timeout<F: Future>(duration: Duration, future: F) -> Timeout<F> {
    Timeout {
        sleep: sleep(duration),
        future,
    }
}
