use alloc::format;
use core::iter::{self, FusedIterator};

use super::{super::get_backing_name, Field, Module, Version, CSTR};
use crate::{future::retry, print_message, string::ArrayCString, Address, Error, Process};

#[cfg(feature = "derive")]
pub use asr_derive::MonoClass as Class;
use bytemuck::CheckedBitPattern;

/// The kind of MonoClass.
/// See https://github.com/mono/mono/blob/0f53e9e151d92944cacab3e24ac359410c606df6/mono/metadata/class-internals.h#L267
#[derive(CheckedBitPattern, Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
#[allow(unused)]
enum MonoTypeKind {
    /// Non-generic type
    DEF = 1,
    /// Generic type definition
    GTD = 2,
    /// Generic instantiation
    GINST = 3,
    /// Generic parameter
    GPARAM = 4,
    /// vector or array
    ARRAY = 5,
    /// pointer or function pointer
    POINTER = 6,
    GC_FILTER = 0xAC,

    #[default]
    Unknown,
}

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

    fn class_kind(&self, process: &Process, module: &Module) -> Result<MonoTypeKind, Error> {
        match module.version {
            // See https://github.com/mono/mono/blob/337052f86112fc0dc8435c5c4a2de43b399a14bb/mono/metadata/class-internals.h#L327
            Version::V2 => {
                // TODO I feel like I'm doing this very poorly

                let byte =
                    process.read::<u8>(self.class + module.offsets.class.class_kind)? & 0x7u8;

                if !MonoTypeKind::is_valid_bit_pattern(&byte) {
                    return Err(Error {});
                }

                // SAFETY: We just checked if this was valid
                let kind: MonoTypeKind = unsafe { (&raw const byte).cast::<MonoTypeKind>().read() };

                Ok(kind)
            }
            // See https://github.com/mono/mono/blob/0f53e9e151d92944cacab3e24ac359410c606df6/mono/metadata/class-private-definition.h#L28
            Version::V3 => {
                print_message(&format!("class: {}", self.class));
                process.read::<MonoTypeKind>(self.class + module.offsets.class.class_kind)
            }
            _ => Err(Error {}),
        }
    }

    fn field_count(&self, process: &Process, module: &Module) -> Result<i32, Error> {
        match module.version {
            Version::V1 | Version::V1Cattrs => {
                process.read::<i32>(self.class + module.offsets.class.field_count)
            }
            Version::V2 | Version::V3 => {
                let class_kind = self.class_kind(process, module)?;
                print_message(&format!("ck {:?} (class: {})", class_kind, self.class));

                // See https://github.com/mono/mono/blob/0f53e9e151d92944cacab3e24ac359410c606df6/mono/metadata/class-accessors.c#L216
                match class_kind {
                    MonoTypeKind::DEF | MonoTypeKind::GTD => {
                        process.read::<i32>(self.class + module.offsets.class.field_count)
                    }
                    MonoTypeKind::GINST => {
                        let generic_class = process.read_pointer(
                            self.class + module.offsets.class.generic_class,
                            module.get_pointer_size(),
                        )?;
                        let container_class = Class {
                            class: process
                                .read_pointer(generic_class + 0x0, module.get_pointer_size())?,
                        };

                        container_class.field_count(process, module)
                    }
                    _ => Ok(0),
                }
            }
            _ => Err(Error {}),
        }
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

            let field_count = class
                .field_count(process, module)
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
