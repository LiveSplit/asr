//! Useful synchronization primitives.

use core::{
    cell::{RefCell, RefMut},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A mutual exclusion primitive useful for protecting shared data. This mutex
/// is specifically for single-threaded WebAssembly.
pub struct Mutex<T: ?Sized>(RefCell<T>);

/// An RAII implementation of a “scoped lock” of a mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// [`Deref`] and [`DerefMut`] implementations.
///
/// This structure is created by the [`lock`](Mutex::<T>::lock) and
/// [`try_lock`](Mutex::<T>::try_lock) methods on Mutex.
pub struct MutexGuard<'a, T: ?Sized>(RefMut<'a, T>);

/// A type alias for the result of a nonblocking locking method.
pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

/// An enumeration of possible errors associated with a [`TryLockResult`] which
/// can occur while trying to acquire a lock, from the
/// [`try_lock`](Mutex::<T>::try_lock) method on a Mutex.
pub struct TryLockError<T> {
    _private: PhantomData<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(RefCell::new(value))
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a mutex, panics if it is unable to do so.
    #[track_caller]
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.borrow_mut())
    }

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then Err is returned.
    /// Otherwise, an RAII guard is returned. The lock will be unlocked when the
    /// guard is dropped.
    //
    /// This function does not block.
    #[inline]
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        Ok(MutexGuard(self.0.try_borrow_mut().map_err(|_| {
            TryLockError {
                _private: PhantomData,
            }
        })?))
    }

    /// Consumes this mutex, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.0.into_inner()
    }

    /// Returns a mutable reference to the underlying data.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(not(target_feature = "atomics"))]
// SAFETY: This is the same as std's Mutex, but it can only be safe in
// single-threaded WASM, because we use RefCell underneath.
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

#[cfg(not(target_feature = "atomics"))]
// SAFETY: This is the same as std's Mutex, but it can only be safe in
// single-threaded WASM, because we use RefCell underneath.
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

// TODO: Currently not possible in stable Rust.
// impl<T: ?Sized> !Send for MutexGuard<'_, T>
#[cfg(not(target_feature = "atomics"))]
// SAFETY: This is the same as std's MutexGuard, but it can only be safe in
// single-threaded WASM, because we use RefMut underneath.
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}
