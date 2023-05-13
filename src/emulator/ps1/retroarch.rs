use crate::{file_format::pe, signature::Signature, Address, Address32, Address64, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SUPPORTED_CORES: [&str; 4] = [
            "mednafen_psx_hw_libretro.dll",
            "mednafen_psx_libretro.dll",
            "swanstation_libretro.dll",
            "pcsx_rearmed_libretro.dll",
        ];

        let main_module_address = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
            .find_map(|(name, _)| game.get_module_address(name).ok())?;

        let is_64_bit =
            pe::MachineType::read(game, main_module_address) == Some(pe::MachineType::X86_64);

        let (core_name, core_address) = SUPPORTED_CORES
            .iter()
            .find_map(|&m| Some((m, game.get_module_address(m).ok()?)))?;

        self.core_base = core_address;

        if core_name == SUPPORTED_CORES[0] || core_name == SUPPORTED_CORES[1] {
            // Mednafen
            if is_64_bit {
                const SIG: Signature<14> =
                    Signature::new("48 8B 05 ?? ?? ?? ?? 41 81 E4 FF FF 1F 00");
                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 3;
                let ptr = ptr + 0x4 + game.read::<i32>(ptr).ok()?;
                Some(game.read::<Address64>(ptr).ok()?.into())
            } else {
                const SIG: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 E3 FF FF 1F 00");
                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 1;
                let ptr = game.read::<Address32>(ptr).ok()?;
                Some(game.read::<Address32>(ptr).ok()?.into())
            }
        } else if core_name == SUPPORTED_CORES[2] {
            // Swanstation
            if is_64_bit {
                const SIG: Signature<15> =
                    Signature::new("48 89 0D ?? ?? ?? ?? 89 35 ?? ?? ?? ?? 89 3D");
                let addr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 3;
                let ptr = addr + 0x4 + game.read::<i32>(addr).ok()?;
                Some(game.read::<Address64>(ptr).ok()?.into())
            } else {
                const SIG: Signature<8> = Signature::new("A1 ?? ?? ?? ?? 23 CB 8B");
                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 1;
                let ptr = game.read::<Address32>(ptr).ok()?;
                Some(game.read::<Address32>(ptr).ok()?.into())
            }
        } else if core_name == SUPPORTED_CORES[3] {
            // PCSX ReARMed
            if is_64_bit {
                const SIG: Signature<9> = Signature::new("48 8B 35 ?? ?? ?? ?? 81 E2");
                let addr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 3;
                let ptr = addr + 0x4 + game.read::<i32>(addr).ok()?;
                let ptr = game.read::<Address64>(ptr).ok()?;
                Some(game.read::<Address64>(ptr).ok()?.into())
            } else {
                const SIG: Signature<9> = Signature::new("FF FF 1F 00 89 ?? ?? ?? A1");
                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 9;
                let ptr = game.read::<Address32>(ptr).ok()?;
                Some(game.read::<Address32>(ptr).ok()?.into())
            }
        } else {
            None
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
