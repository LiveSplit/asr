use crate::{file_format::pe, signature::Signature, Address, Address64, PointerSize, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr_base: Address,
    offsets: [u64; 3],
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::PcsxRedux(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        if pe::MachineType::read(game, main_module_range.0)?.pointer_size()? != PointerSize::Bit64 {
            return None;
        }

        const SIG_BASE: Signature<19> =
            Signature::new("48 8B 05 ?? ?? ?? ?? 48 8B 80 ?? ?? ?? ?? 48 8B 50 ?? E8");

        let addr = SIG_BASE.scan_process_range(game, main_module_range)? + 3;

        self.addr_base = addr + 0x4 + game.read::<i32>(addr).ok()?;

        self.offsets = [
            0,
            game.read::<i32>(addr + 7).ok()? as u64,
            game.read::<u8>(addr + 14).ok()?.into(),
        ];

        game.read_pointer_path::<Address64>(self.addr_base, PointerSize::Bit64, &self.offsets)
            .map(|val| val.into())
            .ok()
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &mut Option<Address>) -> bool {
        match game
            .read_pointer_path::<Address64>(self.addr_base, PointerSize::Bit64, &self.offsets)
            .ok()
            .filter(|addr| !addr.is_null())
        {
            Some(result) => {
                *ram_base = Some(result.into());
                true
            }
            None => false,
        }
    }

    pub const fn new() -> Self {
        Self {
            addr_base: Address::NULL,
            offsets: [0; 3],
        }
    }
}
