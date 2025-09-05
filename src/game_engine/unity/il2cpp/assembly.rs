use super::{Image, Module};
use crate::{string::ArrayCString, Address, Error, Process};

#[derive(Copy, Clone)]
pub(super) struct Assembly {
    pub(super) assembly: Address,
}

impl Assembly {
    pub(super) fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.assembly + module.offsets.assembly.aname,
                module.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    pub(super) fn get_image(&self, process: &Process, module: &Module) -> Option<Image> {
        process
            .read_pointer(
                self.assembly + module.offsets.assembly.image,
                module.pointer_size,
            )
            .ok()
            .filter(|addr| !addr.is_null())
            .map(|image| Image { image })
    }
}
