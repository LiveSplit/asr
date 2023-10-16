use crate::{file_format::pe, signature::Signature, Address, Address64, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SUPPORTED_CORES: [&str; 1] = ["pcsx2_libretro.dll"];

        let main_module_address = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
            .find_map(|(name, _)| game.get_module_address(name).ok())?;

        let is_64_bit =
            pe::MachineType::read(game, main_module_address) == Some(pe::MachineType::X86_64);

        if !is_64_bit {
            // The LRPS2 core, the only one available for retroarch at
            // the time of writing (Sep 14th, 2023), only supports 64-bit
            return None;
        }

        let (core_name, core_address) = SUPPORTED_CORES
            .iter()
            .find_map(|&m| Some((m, game.get_module_address(m).ok()?)))?;

        self.core_base = core_address;

        let base_addr = {
            const SIG: Signature<13> = Signature::new("48 8B ?? ?? ?? ?? ?? 81 ?? F0 3F 00 00");
            let ptr = SIG
                .scan_process_range(game, (core_address, game.get_module_size(core_name).ok()?))?
                + 3;
            ptr + 0x4 + game.read::<i32>(ptr).ok()?
        };

        match game.read::<Address64>(base_addr) {
            Ok(Address64::NULL) => None,
            Ok(x) => Some(x.into()),
            _ => None,
        }
    }

    pub fn keep_alive(&self, game: &Process) -> bool {
        game.read::<u8>(self.core_base).is_ok()
    }

    pub const fn new() -> Self {
        Self {
            core_base: Address::NULL,
        }
    }
}
