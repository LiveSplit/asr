use crate::{string::ArrayCString, Address, Error, Process};

use super::Module;

#[derive(Copy, Clone)]
pub(super) struct Field {
    pub(super) field: Address,
}

impl Field {
    pub(super) fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(self.field + module.offsets.field.name, module.pointer_size)
            .and_then(|addr| process.read(addr))
    }

    pub(super) fn get_offset(&self, process: &Process, module: &Module) -> Option<u32> {
        process.read(self.field + module.offsets.field.offset).ok()
    }
}
