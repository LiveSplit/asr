use super::super::managed::ClassRef;
use super::Module;
use crate::{future::retry, Address, Process};

#[cfg(feature = "derive")]
pub use asr_derive::Il2cppClass as Class;

/// A .NET class that is part of an [`Image`](super::Image).
#[derive(Copy, Clone)]
pub struct Class {
    pub(super) class: Address,
}

impl Class {
    /// Tries to find a field with the specified name in the class. This returns
    /// the offset of the field from the start of an instance of the class. If
    /// it's a static field, the offset will be from the start of the static
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
                .filter(|val| !val.is_null())
        })
        .await
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
