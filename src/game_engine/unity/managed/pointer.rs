use super::{ClassRef, ImageRef, Walk};
use crate::{Address, Error, Process};
use bytemuck::CheckedBitPattern;
use core::{array, cell::RefCell};

/// The pointer path resolution both backends' `UnityPointer` types share: a
/// static root found by class name, then fields resolved by name or written as
/// literal offsets, remembered across calls so a failed resolution resumes
/// where it left off.
pub struct PointerPath<const CAP: usize> {
    inner: RefCell<PointerPathInternal<CAP>>,
}

struct PointerPathInternal<const CAP: usize> {
    base_address: Address,
    offsets: [u32; CAP],
    resolved_offsets: usize,

    starting_class_name: &'static str,
    starting_class: Option<ClassRef>,
    nr_of_parents: usize,
    fields: [&'static str; CAP],
    depth: usize,
}

impl<const CAP: usize> PointerPath<CAP> {
    pub fn new(class_name: &'static str, nr_of_parents: usize, fields: &[&'static str]) -> Self {
        let named_fields: [&str; CAP] =
            array::from_fn(|i| fields.get(i).copied().unwrap_or_default());

        Self {
            inner: RefCell::new(PointerPathInternal {
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

    /// Tries to resolve the pointer path, resuming behind whatever resolved on
    /// an earlier call. Finding the starting class walks every class the image
    /// holds, so it is remembered the first time it answers.
    fn find_offsets(&self, process: &Process, walk: &Walk, image: ImageRef) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        if inner.resolved_offsets == inner.depth {
            return Ok(());
        }

        let starting_class = match inner.starting_class {
            Some(starting_class) => starting_class,
            _ => {
                let mut class = walk
                    .find_class(process, image, inner.starting_class_name)
                    .ok_or(Error {})?;

                for _ in 0..inner.nr_of_parents {
                    class = walk.parent(process, class).ok_or(Error {})?;
                }

                inner.starting_class = Some(class);
                class
            }
        };

        if inner.base_address.is_null() {
            inner.base_address = walk.static_table(process, starting_class).ok_or(Error {})?;
        }

        // Whatever resolved already is walked again from the base, which is
        // what recovers the farthest object the resolution reached.
        let mut current_object = {
            let mut address = inner.base_address;
            for &offset in &inner.offsets[..inner.resolved_offsets] {
                address = process.read_pointer(address + offset, walk.pointer_size)?;
            }
            address
        };

        for i in inner.resolved_offsets..inner.depth {
            let offset_from_string = match inner.fields[i].strip_prefix("0x") {
                Some(rem) => u32::from_str_radix(rem, 16).ok(),
                _ => inner.fields[i].parse().ok(),
            };

            let current_offset = match offset_from_string {
                Some(offset) => offset,
                _ => {
                    let current_class = match i {
                        0 => starting_class,
                        _ => walk.object_class(process, current_object).ok_or(Error {})?,
                    };

                    walk.find_field_offset(process, current_class, inner.fields[i])
                        .ok_or(Error {})?
                        .1
                }
            };

            inner.offsets[i] = current_offset;
            inner.resolved_offsets += 1;

            current_object =
                process.read_pointer(current_object + current_offset, walk.pointer_size)?;
        }

        Ok(())
    }

    /// Dereferences the pointer path, returning the memory address of the
    /// value of interest.
    pub fn deref_offsets(
        &self,
        process: &Process,
        walk: &Walk,
        image: ImageRef,
    ) -> Result<Address, Error> {
        self.find_offsets(process, walk, image)?;
        let inner = self.inner.borrow();
        let mut address = inner.base_address;
        let (&last, path) = inner.offsets[..inner.depth].split_last().ok_or(Error {})?;
        for &offset in path {
            address = process.read_pointer(address + offset, walk.pointer_size)?;
        }
        Ok(address + last)
    }

    /// Dereferences the pointer path, returning the value stored at the final
    /// memory address.
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        walk: &Walk,
        image: ImageRef,
    ) -> Result<T, Error> {
        process.read(self.deref_offsets(process, walk, image)?)
    }
}
