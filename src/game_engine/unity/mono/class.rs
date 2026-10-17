use super::super::managed::{self, ClassRef};
use super::Module;
use crate::{future::retry, Address, Process};

#[cfg(feature = "derive")]
pub use asr_derive::MonoClass as Class;

/// A .NET class that is part of an [`Image`](super::Image).
#[derive(Copy, Clone)]
pub struct Class {
    pub(super) class: Address,
}

impl Class {
    /// Tries to find the offset for a field with the specified name in the class.
    /// If it's a static field, the offset will be from the start of the static
    /// table.
    pub fn get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Option<u32> {
        module
            .walk()
            .find_field_offset(process, ClassRef::new(self.class), field_name)
            .map(|(_, offset)| offset)
    }

    /// Tries to find the address of a static instance of the class based on its
    /// field name. This waits until the field is not null.
    pub async fn wait_get_static_instance(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Address {
        // The field's offset measures into the static table of whichever
        // class declares it, which a climb may find on a parent.
        retry(|| {
            let walk = module.walk();
            let (class, offset) =
                walk.find_field_offset(process, ClassRef::new(self.class), field_name)?;
            let static_table = walk.static_table(process, class)?;

            process
                .read_pointer(static_table + offset, module.pointer_size)
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
        module
            .walk()
            .object_class(process, object)
            .map(|class| Class {
                class: class.address,
            })
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
        let managed::Runtime::Mono(mono) = module.walk().runtime else {
            return None;
        };
        mono.vtable_address(process, module.pointer_size, ClassRef::new(self.class))
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields.
    pub fn get_static_table(&self, process: &Process, module: &Module) -> Option<Address> {
        module
            .walk()
            .static_table(process, ClassRef::new(self.class))
    }

    /// Tries to find the parent class.
    pub fn get_parent(&self, process: &Process, module: &Module) -> Option<Class> {
        module
            .walk()
            .parent(process, ClassRef::new(self.class))
            .map(|class| Class {
                class: class.address,
            })
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
