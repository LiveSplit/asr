use crate::{file_format::pe, Address, Endian, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process, endian: &mut Endian) -> Option<[Address; 2]> {
        const SUPPORTED_CORES: [&str; 1] = ["dolphin_libretro.dll"];

        let main_module_address = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
            .find_map(|(name, _)| game.get_module_address(name).ok())?;

        let is_64_bit =
            pe::MachineType::read(game, main_module_address) == Some(pe::MachineType::X86_64);

        if !is_64_bit {
            // The Dolphin core, the only one available for retroarch, only supports 64-bit
            return None;
        }

        self.core_base = SUPPORTED_CORES
            .iter()
            .find_map(|&m| game.get_module_address(m).ok())?;

        *endian = Endian::Big;
        super::dolphin::State::find_ram(&super::dolphin::State, game, endian)
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<[Address; 2]>) -> bool {
        game.read::<u8>(self.core_base).is_ok()
            && ram_base.is_some_and(|[mem1, mem2]| {
                game.read::<u8>(mem1).is_ok() && game.read::<u8>(mem2).is_ok()
            })
    }

    pub const fn new() -> Self {
        Self {
            core_base: Address::NULL,
        }
    }
}
