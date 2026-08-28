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
        let name = match (
            module.offsets.image.assembly_name,
            module.offsets.assembly.aname,
        ) {
            (Some(assembly_name), _) => {
                self.get_image(process, module).ok_or(Error {})?.image + assembly_name
            }
            (_, Some(aname)) => self.assembly + aname,
            _ => return Err(Error {}),
        };

        process
            .read_pointer(name, module.pointer_size)
            .and_then(|addr| process.read(addr))
    }

    pub(super) fn get_image(&self, process: &Process, module: &Module) -> Option<Image> {
        process
            .read_pointer(
                self.assembly + module.offsets.assembly.image,
                module.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
            .map(|image| Image { image })
    }
}
