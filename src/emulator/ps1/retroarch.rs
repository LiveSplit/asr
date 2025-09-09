use crate::{
    file_format::pe, signature::Signature, Address, Address32, Address64, PointerSize, Process,
};

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
            pe::MachineType::read(game, main_module_address)?.pointer_size()? == PointerSize::Bit64;

        let (core_name, core_address) = SUPPORTED_CORES
            .iter()
            .find_map(|&m| Some((m, game.get_module_address(m).ok()?)))?;

        self.core_base = core_address;

        let base_scan = pe::Symbol::iter(game, core_address)
            .find(|symbol| {
                symbol
                    .get_name::<22>(game)
                    .is_ok_and(|name| name.matches(b"retro_get_memory_data"))
            })?
            .address;

        if core_name == SUPPORTED_CORES[0] || core_name == SUPPORTED_CORES[1] {
            // Mednafen
            if is_64_bit {
                const SIG: Signature<4> = Signature::new("48 0F 44 05");
                SIG.scan_process_range(game, (base_scan, 0x100))
                    .map(|addr| addr + 4)
                    .and_then(|addr| {
                        game.read::<Address64>(addr + 0x4 + game.read::<i32>(addr).ok()?)
                            .ok()
                    })
                    .map(|addr| addr.into())
            } else {
                const SIG: Signature<3> = Signature::new("0F 44 05");
                SIG.scan_process_range(game, (base_scan, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| {
                        game.read::<Address32>(game.read::<Address32>(addr).ok()?)
                            .ok()
                    })
                    .map(|addr| addr.into())
            }
        } else if core_name == SUPPORTED_CORES[2] {
            // Swanstation
            if is_64_bit {
                const SIG: Signature<8> = Signature::new("48 8B 05 ?? ?? ?? ?? C3");
                SIG.scan_process_range(game, (base_scan, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| {
                        game.read::<Address64>(addr + 0x4 + game.read::<i32>(addr).ok()?)
                            .ok()
                    })
                    .map(|addr| addr.into())
            } else {
                const SIG: Signature<3> = Signature::new("74 ?? A1");
                SIG.scan_process_range(game, (base_scan, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| {
                        game.read::<Address32>(game.read::<Address32>(addr).ok()?)
                            .ok()
                    })
                    .map(|addr| addr.into())
            }
        } else if core_name == SUPPORTED_CORES[3] {
            // PCSX ReARMed
            if is_64_bit {
                const SIG: Signature<10> = Signature::new("48 8B 05 ?? ?? ?? ?? 48 8B 00");
                SIG.scan_process_range(game, (base_scan, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| {
                        game.read::<Address64>(
                            game.read::<Address64>(addr + 0x4 + game.read::<i32>(addr).ok()?)
                                .ok()?,
                        )
                        .ok()
                    })
                    .map(|addr| addr.into())
            } else {
                const SIG: Signature<8> = Signature::new("0F 44 05 ?? ?? ?? ?? C3");
                SIG.scan_process_range(game, (base_scan, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| {
                        game.read::<Address32>(game.read::<Address32>(addr).ok()?)
                            .ok()
                    })
                    .map(|addr| addr.into())
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
