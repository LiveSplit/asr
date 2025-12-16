//! Futures support for writing auto splitters with asynchronous code.
//!
//! If you want to write an auto splitter that uses asynchronous code, you can
//! use the [`async_main`](crate::async_main) macro to define an asynchronous
//! `main` function instead of defining an `update` function as the entrypoint
//! for your auto splitter.
//!
//! Similar to using an `update` function, it is important to constantly yield
//! back to the runtime to communicate that the auto splitter is still alive.
//! All asynchronous code that you await automatically yields back to the
//! runtime. However, if you want to write synchronous code, such as the main
//! loop handling of a process on every tick, you can use the [`next_tick`]
//! function to yield back to the runtime and continue on the next tick.
//!
//! The main low level abstraction is the [`retry`] function, which wraps any
//! code that you want to retry until it succeeds, yielding back to the runtime
//! between each try.
//!
//! So if you wanted to attach to a Process you could for example write:
//!
//! ```no_run
//! # use asr::{Process, future::retry};
//! # async fn example() {
//! let process = retry(|| Process::attach("MyGame.exe")).await;
//! # }
//! ```
//!
//! This will try to attach to the process every tick until it succeeds. This
//! specific example is exactly how the [`Process::wait_attach`] method is
//! implemented. So if you wanted to attach to any of multiple processes, you
//! could for example write:
//!
//! ```no_run
//! # use asr::{Process, future::retry};
//! # async fn example() {
//! let process = retry(|| {
//!    ["a.exe", "b.exe"].into_iter().find_map(Process::attach)
//! }).await;
//! # }
//! ```
//!
//! # Example
//!
//! Here is a full example of how an auto splitter could look like using the
//! [`async_main`](crate::async_main) macro:
//!
//! Usage on stable Rust:
//! ```ignore
//! asr::async_main!(stable);
//! ```
//!
//! Usage on nightly Rust:
//! ```ignore
//! #![feature(type_alias_impl_trait, const_async_blocks)]
//!
//! asr::async_main!(nightly);
//! ```
//!
//! The asynchronous main function itself:
//! ```ignore
//! async fn main() {
//!     // TODO: Set up some general state and settings.
//!     loop {
//!         let process = Process::wait_attach("explorer.exe").await;
//!         process.until_closes(async {
//!             // TODO: Load some initial information from the process.
//!             loop {
//!                 // TODO: Do something on every tick.
//!                next_tick().await;
//!             }
//!         }).await;
//!     }
//! }
//! ```
//!
//! # Running multiple tasks concurrently
//!
//! If you want to run multiple tasks concurrently, you can call the
//! [`run_tasks`] function. This will wrap a future and provide a `tasks` object
//! to it, which you can use to spawn tasks that will run in the background. The
//! entire future will complete once all tasks have completed.
//!
//! ```no_run
//! # use asr::future::run_tasks;
//! # async fn example() {
//! run_tasks(|tasks| async move {
//!     // do some work
//!
//!     tasks.spawn(async {
//!         // do some background work
//!     });
//!
//!     // use spawn_recursive to spawn tasks that can spawn further tasks
//!     tasks.spawn_recursive(|tasks| async move {
//!         tasks.spawn(async {
//!             // do some background work
//!         });
//!     });
//!
//!     // do some work
//! }).await;
//! # }
//! ```

use core::{
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "signature")]
use crate::signature::Signature;
use crate::{Address, Process};

#[cfg(target_os = "wasi")]
mod time;
#[cfg(target_os = "wasi")]
pub use self::time::*;

#[cfg(feature = "alloc")]
mod task;
#[cfg(feature = "alloc")]
pub use self::task::*;

/// A future that yields back to the runtime and continues on the next tick. It's
/// important to yield back to the runtime to communicate that the auto splitter
/// is still alive.
#[must_use = "You need to await this future."]
pub struct NextTick(bool);

impl Future for NextTick {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if !mem::replace(&mut self.0, true) {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/// A future that retries the given function until it returns [`Some`], yielding
/// back to the runtime between each call.
#[must_use = "You need to await this future."]
pub struct Retry<F> {
    f: F,
}

impl<O: IntoOption, F: FnMut() -> O + Unpin> Future for Retry<F> {
    type Output = O::T;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        match (self.f)().into_option() {
            Some(t) => Poll::Ready(t),
            None => Poll::Pending,
        }
    }
}

/// Yields back to the runtime and continues on the next tick. It's important to
/// yield back to the runtime to communicate that the auto splitter is still
/// alive.
///
/// # Example
///
/// ```no_run
/// # use asr::{Process, future::next_tick};
/// # async fn example() {
/// loop {
///     // TODO: Do something on every tick.
///     next_tick().await;
/// }
/// # }
/// ```
pub const fn next_tick() -> NextTick {
    NextTick(false)
}

/// Retries the given function until it returns [`Some`] or [`Ok`], yielding
/// back to the runtime between each call.
///
/// # Example
///
/// If you wanted to attach to a Process you could for example write:
///
/// ```no_run
/// # use asr::{Process, future::retry};
/// # async fn example() {
/// let process = retry(|| Process::attach("MyGame.exe")).await;
/// # }
/// ```
///
/// This will try to attach to the process every tick until it succeeds. This
/// specific example is exactly how the [`Process::wait_attach`] method is
/// implemented. So if you wanted to attach to any of multiple processes, you
/// could for example write:
///
/// ```no_run
/// # use asr::{Process, future::retry};
/// # async fn example() {
/// let process = retry(|| {
///    ["a.exe", "b.exe"].into_iter().find_map(Process::attach)
/// }).await;
/// # }
/// ```
pub const fn retry<O: IntoOption, F: FnMut() -> O + Unpin>(f: F) -> Retry<F> {
    Retry { f }
}

/// A trait for types that can be converted into an [`Option`].
// TODO: Replace this with `Try` once that is stable.
pub trait IntoOption {
    /// The type that is contained in the [`Option`].
    type T;
    /// Converts `self` into an [`Option`].
    fn into_option(self) -> Option<Self::T>;
}

impl<T> IntoOption for Option<T> {
    type T = T;
    fn into_option(self) -> Option<Self::T> {
        self
    }
}

impl<T, E> IntoOption for Result<T, E> {
    type T = T;
    fn into_option(self) -> Option<Self::T> {
        self.ok()
    }
}

impl Process {
    /// Asynchronously awaits attaching to a process with the given name,
    /// yielding back to the runtime between each try.
    pub async fn wait_attach(name: &str) -> Process {
        retry(|| Process::attach(name)).await
    }

    /// Executes a future until the process closes.
    pub const fn until_closes<F>(&self, future: F) -> UntilProcessCloses<'_, F> {
        UntilProcessCloses {
            process: self,
            future,
        }
    }

    /// Asynchronously awaits the address and size of a module in the process,
    /// yielding back to the runtime between each try.
    pub async fn wait_module_range(&self, name: &str) -> (Address, u64) {
        retry(|| self.get_module_range(name)).await
    }
}

#[cfg(feature = "signature")]
impl<const N: usize> Signature<N> {
    /// Asynchronously awaits scanning a process for the signature until it is
    /// found. This will scan the address range of the process given. Once the
    /// signature is found, the address of the start of the signature is
    /// returned.
    pub async fn wait_scan_process_range(
        &self,
        process: &Process,
        (addr, len): (impl Into<Address>, u64),
    ) -> Address {
        let addr = addr.into();
        retry(|| self.scan_process_range(process, (addr, len))).await
    }
}

/// A future that executes a future until the process closes.
#[must_use = "You need to await this future."]
pub struct UntilProcessCloses<'a, F> {
    process: &'a Process,
    future: F,
}

impl<T, F: Future<Output = T>> Future for UntilProcessCloses<'_, F> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.process.is_open() {
            return Poll::Ready(None);
        }
        // SAFETY: We are simply projecting the Pin.
        unsafe {
            Pin::new_unchecked(&mut self.get_unchecked_mut().future)
                .poll(cx)
                .map(Some)
        }
    }
}

/// Defines that the auto splitter is using an asynchronous `main` function
/// instead of the normal poll based `update` function. It is important to
/// frequently yield back to the runtime to communicate that the auto splitter
/// is still alive. If the function ends, the auto splitter will stop executing
/// code.
///
/// There are two versions of the macro depending on whether you use `stable` or
/// `nightly` Rust. This needs to be passed to the macro as an argument. The
/// `stable` variant currently allocates WebAssembly pages to store the future.
/// This is still compatible with `no_std`, even without `alloc`. If you use the
/// `nightly` version of this macro, the future is stored in a global variable
/// at compile time, removing the work needed at runtime to store the future. If
/// you do so, you need to enable the `type_alias_impl_trait` and
/// `const_async_blocks` features.
///
/// # Example
///
/// Usage on stable Rust:
/// ```ignore
/// async_main!(stable);
/// ```
///
/// Usage on nightly Rust:
/// ```ignore
/// #![feature(type_alias_impl_trait, const_async_blocks)]
///
/// async_main!(nightly);
/// ```
///
/// Example of an asynchronous `main` function:
/// ```ignore
/// async fn main() {
///     // TODO: Set up some general state and settings.
///     loop {
///         let process = Process::wait_attach("explorer.exe").await;
///         process.until_closes(async {
///             // TODO: Load some initial information from the process.
///             loop {
///                 // TODO: Do something on every tick.
///                next_tick().await;
///             }
///         }).await;
///     }
/// }
/// ```
#[macro_export]
macro_rules! async_main {
    (nightly) => {
        /// # Safety
        /// Invoking this function yourself causes Undefined Behavior.
        #[no_mangle]
        pub unsafe extern "C" fn update() {
            use core::{
                future::Future,
                pin::Pin,
                ptr,
                task::{Context, RawWaker, RawWakerVTable, Waker},
            };
            use $crate::sync::RacyCell;
            mod fut {
                pub type MainFuture = impl core::future::Future<Output = ()>;
                #[define_opaque(MainFuture)]
                pub const fn main_type() -> MainFuture {
                    async {
                        super::main().await;
                    }
                }
            }

            static STATE: RacyCell<fut::MainFuture> = RacyCell::new(fut::main_type());
            static FINISHED: RacyCell<bool> = RacyCell::new(false);
            if unsafe { *FINISHED.get() } {
                return;
            }
            static VTABLE: RawWakerVTable = RawWakerVTable::new(
                |_| RawWaker::new(ptr::null(), &VTABLE),
                |_| {},
                |_| {},
                |_| {},
            );
            let raw_waker = RawWaker::new(ptr::null(), &VTABLE);
            let waker = unsafe { Waker::from_raw(raw_waker) };
            let mut cx = Context::from_waker(&waker);
            unsafe {
                *FINISHED.get_mut() = Pin::new_unchecked(&mut *STATE.get_mut())
                    .poll(&mut cx)
                    .is_ready();
            }
        }
    };
    (stable) => {
        /// # Safety
        /// Invoking this function yourself causes Undefined Behavior.
        #[no_mangle]
        #[cfg(target_family = "wasm")]
        pub unsafe extern "C" fn update() {
            use core::{
                cell::UnsafeCell,
                future::Future,
                mem::{self, ManuallyDrop},
                pin::Pin,
                ptr,
                task::{Context, RawWaker, RawWakerVTable, Waker},
            };
            use $crate::sync::RacyCell;

            static STATE: RacyCell<Option<Pin<&'static mut dyn Future<Output = ()>>>> =
                RacyCell::new(None);
            static FINISHED: RacyCell<bool> = RacyCell::new(false);

            if unsafe { *FINISHED.get() } {
                return;
            }

            static VTABLE: RawWakerVTable = RawWakerVTable::new(
                |_| RawWaker::new(ptr::null(), &VTABLE),
                |_| {},
                |_| {},
                |_| {},
            );
            let raw_waker = RawWaker::new(ptr::null(), &VTABLE);
            let waker = unsafe { Waker::from_raw(raw_waker) };
            let mut cx = Context::from_waker(&waker);
            unsafe {
                *FINISHED.get_mut() =
                    Pin::new_unchecked((&mut *STATE.get_mut()).get_or_insert_with(|| {
                        fn allocate<F: Future<Output = ()> + 'static>(
                            f: ManuallyDrop<F>,
                        ) -> Pin<&'static mut dyn Future<Output = ()>> {
                            unsafe {
                                let size = mem::size_of::<F>();
                                const PAGE_SIZE: usize = 64 << 10;
                                assert!(mem::align_of::<F>() <= PAGE_SIZE);
                                let pages = size.div_ceil(PAGE_SIZE);

                                #[cfg(target_arch = "wasm32")]
                                let old_page_count = core::arch::wasm32::memory_grow(0, pages);
                                #[cfg(target_arch = "wasm64")]
                                let old_page_count = core::arch::wasm64::memory_grow(0, pages);

                                let address = old_page_count * PAGE_SIZE;
                                let ptr = address as *mut ManuallyDrop<F>;
                                ptr::write(ptr, f);
                                let ptr = ptr.cast::<F>();
                                let future: &'static mut F = &mut *ptr;
                                let future: &'static mut dyn Future<Output = ()> = future;
                                Pin::static_mut(future)
                            }
                        }

                        allocate(ManuallyDrop::new(main()))
                    }))
                    .poll(&mut cx)
                    .is_ready();
            };
        }
    };
}
