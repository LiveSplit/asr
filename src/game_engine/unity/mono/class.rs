use super::{Field, Module, Version, CSTR};
use crate::{future::retry, string::ArrayCString, Address, Error, Process};
use core::iter::{self, FusedIterator};

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
                    if name.matches(field_name) {
                        return true;
                    }

                    // BackingField pattern: <FieldName>k__BackingField
                    if let Ok(this_field_name) = name.validate_utf8() {
                        if let Some(start) = this_field_name.find('<') {
                            if let Some(end) = this_field_name[start + 1..].find('>') {
                                return &this_field_name[start + 1..start + 1 + end] == field_name;
                            }
                        }
                    }

                    false
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

    fn get_static_table_pointer(&self, process: &Process, module: &Module) -> Option<Address> {
        let runtime_info = process
            .read_pointer(
                self.class + module.offsets.class.runtime_info,
                module.pointer_size,
            )
            .ok()
            .filter(|addr| !addr.is_null())?;

        let mut vtables = process
            .read_pointer(runtime_info + module.size_of_ptr(), module.pointer_size)
            .ok()
            .filter(|addr| !addr.is_null())?;

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
