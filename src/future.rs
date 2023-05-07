//! Futures support for writing auto splitters with asynchronous code.
//!
//! If you want to write an auto splitter that uses asynchronous code, you can
//! use the [`async_main`] macro to define an asynchronous `main` function
//! instead of defining an `update` function as the entrypoint for your auto
//! splitter.
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
//! let process = retry(|| Process::attach("MyGame.exe")).await;
//! ```
//!
//! This will try to attach to the process every tick until it succeeds. This
//! specific example is exactly how the [`Process::wait_attach`] method is
//! implemented. So if you wanted to attach to any of multiple processes, you
//! could for example write:
//!
//! ```no_run
//! let process = retry(|| {
//!    ["a.exe", "b.exe"].into_iter().find_map(Process::attach)
//! }).await;
//! ```
//!
//! # Example
//!
//! Here is a full example of how an auto splitter could look like using the
//! [`async_main`] macro:
//!
//! Usage on stable Rust:
//! ```no_run
//! async_main!(stable);
//! ```
//!
//! Usage on nightly Rust:
//! ```no_run
//! #![feature(type_alias_impl_trait, const_async_blocks)]
//!
//! async_main!(nightly);
//! ```
//!
//! The asynchronous main function itself:
//! ```no_run
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

use core::{
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "signature")]
use crate::signature::Signature;
use crate::{Address, Process};

/// A future that yields back to the runtime and continues on the next tick. It's
/// important to yield back to the runtime to communicate that the auto splitter
/// is still alive.
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

/// A future that retries the given function until it returns `Some`, yielding
/// back to the runtime between each call.
pub struct Retry<F> {
    f: F,
}

impl<T, F: FnMut() -> Option<T> + Unpin> Future for Retry<F> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        match (self.f)() {
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
/// loop {
///     // TODO: Do something on every tick.
///     next_tick().await;
/// }
/// ```
#[must_use = "You need to await this future."]
pub const fn next_tick() -> NextTick {
    NextTick(false)
}

/// Retries the given function until it returns `Some`, yielding back to the
/// runtime between each call.
///
/// # Example
///
/// If you wanted to attach to a Process you could for example write:
///
/// ```no_run
/// let process = retry(|| Process::attach("MyGame.exe")).await;
/// ```
///
/// This will try to attach to the process every tick until it succeeds. This
/// specific example is exactly how the [`Process::wait_attach`] method is
/// implemented. So if you wanted to attach to any of multiple processes, you
/// could for example write:
///
/// ```no_run
/// let process = retry(|| {
///    ["a.exe", "b.exe"].into_iter().find_map(Process::attach)
/// }).await;
/// ```
#[must_use = "You need to await this future."]
pub const fn retry<T, F: FnMut() -> Option<T>>(f: F) -> Retry<F> {
    Retry { f }
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
        retry(|| {
            let address = self.get_module_address(name).ok()?;
            let size = self.get_module_size(name).ok()?;
            Some((address, size))
        })
        .await
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
        addr: Address,
        len: u64,
    ) -> Address {
        retry(|| self.scan_process_range(process, addr, len)).await
    }
}

/// A future that executes a future until the process closes.
pub struct UntilProcessCloses<'a, F> {
    process: &'a Process,
    future: F,
}

impl<F: Future<Output = ()>> Future for UntilProcessCloses<'_, F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.process.is_open() {
            return Poll::Ready(());
        }
        // SAFETY: We are simply projecting the Pin.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().future).poll(cx) }
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
/// ```no_run
/// async_main!(stable);
/// ```
///
/// Usage on nightly Rust:
/// ```no_run
/// #![feature(type_alias_impl_trait, const_async_blocks)]
///
/// async_main!(nightly);
/// ```
///
/// Example of an asynchronous `main` function:
/// ```no_run
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
        #[no_mangle]
        pub extern "C" fn update() {
            use core::{
                future::Future,
                pin::Pin,
                ptr,
                task::{Context, RawWaker, RawWakerVTable, Waker},
            };

            type MainFuture = impl Future<Output = ()>;
            const fn main_type() -> MainFuture {
                async {
                    main().await;
                }
            }
            static mut STATE: MainFuture = main_type();

            static VTABLE: RawWakerVTable = RawWakerVTable::new(
                |_| RawWaker::new(ptr::null(), &VTABLE),
                |_| {},
                |_| {},
                |_| {},
            );
            let raw_waker = RawWaker::new(ptr::null(), &VTABLE);
            let waker = unsafe { Waker::from_raw(raw_waker) };
            let mut cx = Context::from_waker(&waker);
            let _ = unsafe { Pin::new_unchecked(&mut STATE).poll(&mut cx) };
        }
    };
    (stable) => {
        #[no_mangle]
        pub extern "C" fn update() {
            use core::{
                future::Future,
                mem::{self, ManuallyDrop},
                pin::Pin,
                ptr,
                task::{Context, RawWaker, RawWakerVTable, Waker},
            };

            static mut STATE: Option<Pin<&'static mut dyn Future<Output = ()>>> = None;

            static VTABLE: RawWakerVTable = RawWakerVTable::new(
                |_| RawWaker::new(ptr::null(), &VTABLE),
                |_| {},
                |_| {},
                |_| {},
            );
            let raw_waker = RawWaker::new(ptr::null(), &VTABLE);
            let waker = unsafe { Waker::from_raw(raw_waker) };
            let mut cx = Context::from_waker(&waker);
            let _ = unsafe {
                Pin::new_unchecked(STATE.get_or_insert_with(|| {
                    fn allocate<F: Future<Output = ()> + 'static>(
                        f: ManuallyDrop<F>,
                    ) -> Pin<&'static mut dyn Future<Output = ()>> {
                        unsafe {
                            let size = mem::size_of::<F>();
                            const PAGE_SIZE: usize = 64 << 10;
                            assert!(mem::align_of::<F>() <= PAGE_SIZE);
                            // TODO: div_ceil
                            let pages = (size + (PAGE_SIZE - 1)) / PAGE_SIZE;

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
            };
        }
    };
}
