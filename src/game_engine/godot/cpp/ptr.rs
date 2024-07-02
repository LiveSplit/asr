use core::{any::type_name, fmt, marker::PhantomData, ops::Add};

use bytemuck::{CheckedBitPattern, Pod, Zeroable};

use crate::{Address64, Error, Process};

/// A pointer is an address in the target process that knows the type that it's
/// targeting.
#[repr(transparent)]
pub struct Ptr<T>(Address64, PhantomData<fn() -> T>);

impl<T> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}*: {}", type_name::<T>(), self.0)
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY: The type is transparent over an `Address64`, which is `Pod`.
unsafe impl<T: 'static> Pod for Ptr<T> {}

// SAFETY: The type is transparent over an `Address64`, which is `Zeroable`.
unsafe impl<T> Zeroable for Ptr<T> {}

impl<T> Ptr<T> {
    /// Creates a new pointer from the given address.
    pub fn new(addr: Address64) -> Self {
        Self(addr, PhantomData)
    }

    /// Checks whether the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Reads the value that this pointer points to from the target process.
    pub fn deref(self, process: &Process) -> Result<T, Error>
    where
        T: CheckedBitPattern,
    {
        process.read(self.0)
    }

    /// Reads the value that this pointer points to from the target process at
    /// the given offset.
    pub fn read_at_offset<U, O>(self, offset: O, process: &Process) -> Result<U, Error>
    where
        U: CheckedBitPattern,
        Address64: Add<O, Output = Address64>,
    {
        process.read(self.0 + offset)
    }

    /// Casts this pointer to a pointer of a different type without any checks.
    pub fn unchecked_cast<U>(self) -> Ptr<U> {
        Ptr::new(self.0)
    }

    /// Returns the address that this pointer points to.
    pub fn addr(self) -> Address64 {
        self.0
    }
}
