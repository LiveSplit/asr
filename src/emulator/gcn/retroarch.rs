use crate::{file_format::pe, Address, Endian, FromEndian, MemoryRangeFlags, Process};

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
            // The Dolphin core, the only one available for retroarch at
            // the time of writing (Sep 19th, 2023), only supports 64-bit
            return None;
        }

        self.core_base = SUPPORTED_CORES
            .iter()
            .find_map(|&m| game.get_module_address(m).ok())?;

        // The following code is essentially the same used for Dolphin
        *endian = Endian::Big;

        // Main logic: finding the address for the GCN main memory by looking for
        // memory ranges with the READ and WRITE flags and a size of 0x2000000.
        // In order to verify we found the correct memory range, we take advantage
        // of a small 'hack', by checking if the offset 0x1C contains a "magic number"
        // fixed for all Gamecube games.

        game.memory_ranges()
            .find(|range| {
                range
                    .flags()
                    .is_ok_and(|r| r.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ))
                    && range.size().is_ok_and(|size| size == 0x2000000)
                    && range.address().is_ok_and(|addr| {
                        game.read::<u32>(addr + 0x1C)
                            .is_ok_and(|magic| magic.from_endian(Endian::Big) == 0xC2339F3D)
                    })
            })?
            .address()
            .ok()
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
