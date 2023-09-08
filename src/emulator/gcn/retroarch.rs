use crate::{file_format::pe, Address, Endian, MemoryRangeFlags, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
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

        for entry in game.memory_ranges() {
            if entry.size().is_ok_and(|size| size == 0x2000000)
                && entry.flags().is_ok_and(|flag| {
                    flag.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ)
                })
            {
                if let Ok(addr) = entry.address() {
                    *endian = Endian::Big;
                    return Some(addr);
                }
            }
        }
        None
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<Address>) -> bool {
        game.read::<u8>(self.core_base).is_ok()
            && ram_base.is_some_and(|ram| game.read::<u8>(ram).is_ok())
    }

    pub const fn new() -> Self {
        Self {
            core_base: Address::NULL,
        }
    }
}
