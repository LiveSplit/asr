use super::{Class, Image, Module};
use crate::{print_message, Address, Error, Process};
use bytemuck::CheckedBitPattern;
use core::{array, cell::RefCell};

/// A Mono-specific implementation for automatic pointer path resolution
pub struct UnityPointer<const CAP: usize> {
    inner: RefCell<UnityPointerInternal<CAP>>,
}

struct UnityPointerInternal<const CAP: usize> {
    base_address: Address,
    offsets: [u32; CAP],
    resolved_offsets: usize,

    starting_class_name: &'static str,
    starting_class: Option<Class>,
    nr_of_parents: usize,
    fields: [&'static str; CAP],
    depth: usize,
}

impl<const CAP: usize> UnityPointer<CAP> {
    /// Creates a new instance of the Pointer struct
    ///
    /// `CAP` should be higher or equal to the number of offsets defined in `fields`.
    ///
    /// If a higher number of offsets is provided, the pointer path will be truncated
    /// according to the value of `CAP`.
    pub fn new(class_name: &'static str, nr_of_parents: usize, fields: &[&'static str]) -> Self {
        let named_fields: [&str; CAP] = {
            let mut iter = fields.iter();
            array::from_fn(|_| iter.next().copied().unwrap_or_default())
        };

        Self {
            inner: RefCell::new(UnityPointerInternal {
                base_address: Address::NULL,
                offsets: [0; CAP],
                resolved_offsets: 0,
                starting_class_name: class_name,
                starting_class: None,
                nr_of_parents,
                fields: named_fields,
                depth: fields.len().min(CAP),
            }),
        }
    }

    /// Tries to resolve the pointer path for the `Mono` class specified
    fn find_offsets(&self, process: &Process, module: &Module, image: &Image) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        // If the pointer path has already been found, there's no need to continue
        if inner.resolved_offsets == inner.depth {
            return Ok(());
        }

        // Logic: the starting class can be recovered with the get_class() function,
        // and parent class can be recovered if needed. However, this is a VERY
        // intensive process because it involves looping through all the main classes
        // in the game. For this reason, once the class is found, we want to store it
        // into the cache, where it can be recovered if this function need to be run again
        // (for example if a previous attempt at pointer path resolution failed)
        let starting_class = match inner.starting_class {
            Some(starting_class) => starting_class,
            _ => {
                let mut class = image
                    .get_class(process, module, inner.starting_class_name)
                    .ok_or(Error {})?;

                for _ in 0..inner.nr_of_parents {
                    class = class.get_parent(process, module).ok_or(Error {})?;
                }

                inner.starting_class = Some(class);
                class
            }
        };

        // Recovering the address of the static table is not very CPU intensive,
        // but it might be worth caching it as well
        if inner.base_address.is_null() {
            inner.base_address = starting_class
                .get_static_table(process, module)
                .ok_or(Error {})?;
        };

        // If we already resolved some offsets, we need to traverse them again starting from the base address
        // of the static table in order to recalculate the address of the farthest object we can reach.
        // If no offsets have been resolved yet, we just need to read the base address instead.
        let mut current_object = {
            let mut addr = inner.base_address;
            for &i in &inner.offsets[..inner.resolved_offsets] {
                addr = process.read_pointer(addr + i, module.pointer_size)?;
            }
            addr
        };

        // We keep track of the already resolved offsets in order to skip resolving them again
        for i in inner.resolved_offsets..inner.depth {
            let offset_from_string = match inner.fields[i].strip_prefix("0x") {
                Some(rem) => u32::from_str_radix(rem, 16).ok(),
                _ => inner.fields[i].parse().ok(),
            };

            let current_offset = match offset_from_string {
                Some(offset) => offset as _,
                _ => {
                    let current_class = match i {
                        0 => starting_class,
                        _ => process
                            .read_pointer(current_object, module.pointer_size)
                            .ok()
                            .filter(|val| !val.is_null())
                            .and_then(|addr| process.read_pointer(addr, module.pointer_size).ok())
                            .filter(|val| !val.is_null())
                            .map(|class| Class { class })
                            .ok_or(Error {})?,
                    };

                    current_class
                        .get_field_offset(process, module, inner.fields[i])
                        .ok_or(Error {})?
                }
            };

            inner.offsets[i] = current_offset as _;
            inner.resolved_offsets += 1;

            current_object =
                process.read_pointer(current_object + current_offset, module.pointer_size)?;
        }

        Ok(())
    }

    /// Dereferences the pointer path, returning the memory address of the value of interest
    pub fn deref_offsets(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<Address, Error> {
        self.find_offsets(process, module, image)?;
        let inner = self.inner.borrow();
        let mut address = inner.base_address;
        let (&last, path) = inner.offsets[..inner.depth].split_last().ok_or(Error {})?;
        for &offset in path {
            address = process.read_pointer(address + offset, module.pointer_size)?;
        }
        Ok(address + last)
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<T, Error> {
        process.read(self.deref_offsets(process, module, image)?)
    }
}
