use core::iter::{self, FusedIterator};

use super::{super::get_backing_name, Field, Module, Version, CSTR};
use crate::{future::retry, string::ArrayCString, Address, Error, Process};

#[cfg(feature = "derive")]
pub use asr_derive::MonoClass as Class;

/// A .NET class that is part of an [`Image`](Image).
#[derive(Copy, Clone)]
pub struct Class {
    pub(super) class: Address,
}

impl Class {
    pub(super) fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(self.class + module.offsets.class.name, module.pointer_size)
            .and_then(|addr| process.read(addr))
    }

    pub(super) fn get_name_space<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.class + module.offsets.class.namespace,
                module.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    fn fields<'a>(
        &'a self,
        process: &'a Process,
        module: &'a Module,
    ) -> impl FusedIterator<Item = Field> + 'a {
        let mut this_class = Some(*self);

        iter::from_fn(move || {
            let class = this_class?;

            if class
                .get_name::<CSTR>(process, module)
                .ok()?
                .matches("Object")
                || class
                    .get_name_space::<CSTR>(process, module)
                    .ok()?
                    .matches("UnityEngine")
            {
                return None;
            }

            // Prepare for next iteration
            this_class = class.get_parent(process, module);

            let field_count = process
                .read::<i32>(class.class + module.offsets.class.field_count)
                .ok()
                .filter(|&val| val > 0)
                .unwrap_or_default();

            let fields = match field_count {
                0 => None,
                _ => process
                    .read_pointer(
                        class.class + module.offsets.class.fields,
                        module.pointer_size,
                    )
                    .ok(),
            };

            Some((0..field_count as u64).filter_map(move |i| {
                fields.map(|fields| Field {
                    field: fields + i.wrapping_mul(module.offsets.field.alignment as u64),
                })
            }))
        })
        .flatten()
        .fuse()
    }

    /// Tries to find the offset for a field with the specified name in the class.
    /// If it's a static field, the offset will be from the start of the static
    /// table.
    pub fn get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Option<u32> {
        self.fields(process, module)
            .find(|field| {
                field.get_name::<CSTR>(process, module).is_ok_and(|name| {
                    // If the name matches, return immediately
                    name.matches(field_name)

                    // BackingField pattern: <FieldName>k__BackingField
                    || name.validate_utf8()
                        .ok()
                        .and_then(|name| get_backing_name(name))
                        .is_some_and(|name| name == field_name)
                })
            })
            .and_then(|field| field.get_offset(process, module))
    }

    /// Tries to find the address of a static instance of the class based on its
    /// field name. This waits until the field is not null.
    pub async fn wait_get_static_instance(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Address {
        let static_table = self.wait_get_static_table(process, module).await;
        let field_offset = self
            .wait_get_field_offset(process, module, field_name)
            .await;
        let singleton_location = static_table + field_offset;

        retry(|| {
            process
                .read_pointer(singleton_location, module.pointer_size)
                .ok()
                .filter(|addr| !addr.is_null())
        })
        .await
    }

    /// Reads the class of an object found in memory.
    ///
    /// Every managed object begins with a pointer to its class's vtable, and a
    /// vtable begins with a pointer to its class, so an object can be resolved
    /// back to a [`Class`] whose fields are then available by name.
    ///
    /// This is the only way to reach a generic instantiation such as
    /// `HashSet<string>`. Each instantiation is a distinct class with its own
    /// field offsets, and none of them can be looked up in an
    /// [`Image`](super::Image) by name -- but any instance points at its own.
    ///
    /// # Inflated generics may have no field names
    ///
    /// Mono fills a class's field table in lazily, and for an inflated generic
    /// nothing necessarily has. [`get_field_offset`](Self::get_field_offset)
    /// can therefore return [`None`] for every field of a class resolved this
    /// way, against an object that is plainly an instance of it -- and the
    /// same lookup may succeed against another process running the same build,
    /// so it is runtime state rather than anything a version check could
    /// predict. Code that must not fail on a collection needs a fallback to
    /// the known layout, guarded by a check that the object agrees with it.
    pub fn of_object(process: &Process, module: &Module, object: Address) -> Option<Self> {
        // `MonoVTable::klass` is the first member in every version Mono has
        // shipped, so unlike the other offsets in this module it needs no
        // version table. It is the same invariant an object's own header
        // relies on.
        let vtable = process
            .read_pointer(object, module.pointer_size)
            .ok()
            .filter(|addr| !addr.is_null())?;
        let class = process
            .read_pointer(vtable, module.pointer_size)
            .ok()
            .filter(|addr| !addr.is_null())?;
        Some(Self { class })
    }

    /// Returns the address of this class's `MonoVTable` in the first domain.
    ///
    /// Every managed object begins with a pointer to the vtable of its class,
    /// so this doubles as an identity handle for the class: an object at
    /// `addr` is an instance of this exact class if and only if the pointer at
    /// `addr` equals this value.
    ///
    /// This is useful for games where the object of interest cannot be reached
    /// by walking static fields. Games built around constructor-injection
    /// dependency injection often have no static roots at all, which makes
    /// [`UnityPointer`](super::UnityPointer) inapplicable, and the only way to
    /// find a service is to scan the heap for the instance of its class.
    pub fn get_vtable(&self, process: &Process, module: &Module) -> Option<Address> {
        let runtime_info = process
            .read_pointer(
                self.class + module.offsets.class.runtime_info,
                module.pointer_size,
            )
            .ok()
            .filter(|addr| !addr.is_null())?;

        process
            .read_pointer(runtime_info + module.size_of_ptr(), module.pointer_size)
            .ok()
            .filter(|addr| !addr.is_null())
    }

    fn get_static_table_pointer(&self, process: &Process, module: &Module) -> Option<Address> {
        let mut vtables = self.get_vtable(process, module)?;

        // Mono V1 behaves differently when it comes to recover the static table
        match module.version {
            Version::V1 | Version::V1Cattrs => Some(vtables + module.offsets.class.vtable_size),
            _ => {
                vtables = vtables + module.offsets.v_table.vtable;

                let vtable_size = process
                    .read::<u32>(self.class + module.offsets.class.vtable_size)
                    .ok()?;

                Some(vtables + module.size_of_ptr().wrapping_mul(vtable_size as u64))
            }
        }
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields.
    pub fn get_static_table(&self, process: &Process, module: &Module) -> Option<Address> {
        process
            .read_pointer(
                self.get_static_table_pointer(process, module)?,
                module.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
    }

    /// Tries to find the parent class.
    pub fn get_parent(&self, process: &Process, module: &Module) -> Option<Class> {
        process
            .read_pointer(
                self.class + module.offsets.class.parent,
                module.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
            .map(|class| Class { class })
    }

    /// Tries to find a field with the specified name in the class. This returns
    /// the offset of the field from the start of an instance of the class. If
    /// it's a static field, the offset will be from the start of the static
    /// table. This is the `await`able version of the
    /// [`get_field_offset`](Self::get_field_offset) function.
    pub async fn wait_get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        name: &str,
    ) -> u32 {
        retry(|| self.get_field_offset(process, module, name)).await
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields. This is the `await`able version of the
    /// [`get_static_table`](Self::get_static_table) function.
    pub async fn wait_get_static_table(&self, process: &Process, module: &Module) -> Address {
        retry(|| self.get_static_table(process, module)).await
    }

    /// Tries to find the parent class. This is the `await`able version of the
    /// [`get_parent`](Self::get_parent) function.
    pub async fn wait_get_parent(&self, process: &Process, module: &Module) -> Class {
        retry(|| self.get_parent(process, module)).await
    }
}
