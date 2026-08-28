use super::super::managed::{ImageRef, PointerPath};
use super::{Image, Module};
use crate::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

/// A Mono-specific implementation for automatic pointer path resolution
pub struct UnityPointer<const CAP: usize> {
    path: PointerPath<CAP>,
}

impl<const CAP: usize> UnityPointer<CAP> {
    /// Creates a new instance of the Pointer struct
    ///
    /// `CAP` should be higher or equal to the number of offsets defined in `fields`.
    ///
    /// If a higher number of offsets is provided, the pointer path will be truncated
    /// according to the value of `CAP`.
    pub fn new(class_name: &'static str, nr_of_parents: usize, fields: &[&'static str]) -> Self {
        Self {
            path: PointerPath::new(class_name, nr_of_parents, fields),
        }
    }

    /// Dereferences the pointer path, returning the memory address of the value of interest
    pub fn deref_offsets(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<Address, Error> {
        self.path
            .deref_offsets(process, &module.walk(), ImageRef::new(image.image))
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<T, Error> {
        self.path
            .deref(process, &module.walk(), ImageRef::new(image.image))
    }
}
