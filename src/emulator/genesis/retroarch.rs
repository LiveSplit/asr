use crate::{
    file_format::pe,
    signature::{Signature, SignatureScanner},
    Address, Address32, Endian, MemoryRangeFlags, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_wram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        const SUPPORTED_CORES: [&str; 4] = [
            "blastem_libretro.dll",
            "genesis_plus_gx_libretro.dll",
            "genesis_plus_gx_wide_libretro.dll",
            "picodrive_libretro.dll",
        ];

        let main_module_address = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
            .find_map(|(name, _)| game.get_module_address(name).ok())?;

        let is_x86_64 =
            pe::MachineType::read(game, main_module_address) == Some(pe::MachineType::X86_64);

        let (core_name, core_address) = SUPPORTED_CORES
            .iter()
            .find_map(|&m| Some((m, game.get_module_address(m).ok()?)))?;

        self.core_base = core_address;

        if core_name == SUPPORTED_CORES[0] {
            *endian = Endian::Little;

            // BlastEm
            const SIG: Signature<16> =
                Signature::new("72 0E 81 E1 FF FF 00 00 66 8B 89 ?? ?? ?? ?? C3");

            let scanned_address = game
                .memory_ranges()
                .filter(|m| {
                    m.flags()
                        .unwrap_or_default()
                        .contains(MemoryRangeFlags::WRITE)
                        && m.size().unwrap_or_default() == 0x101000
                })
                .find_map(|m| SIG.scan(game, m.range().ok()?))?
                + 11;

            let wram = game.read::<Address32>(scanned_address).ok()?;

            Some(wram.into())
        } else if core_name == SUPPORTED_CORES[1] || core_name == SUPPORTED_CORES[2] {
            *endian = Endian::Little;

            // Genesis plus GX
            if is_x86_64 {
                const SIG_64: Signature<10> = Signature::new("48 8D 0D ?? ?? ?? ?? 4C 8B 2D");

                let addr =
                    SIG_64.scan(game, (core_address, game.get_module_size(core_name).ok()?))? + 3;

                let wram = addr + 0x4 + game.read::<i32>(addr).ok()?;

                Some(wram)
            } else {
                const SIG_32: Signature<7> = Signature::new("A3 ?? ?? ?? ?? 29 F9");

                let ptr =
                    SIG_32.scan(game, (core_address, game.get_module_size(core_name).ok()?))? + 1;

                let wram = game.read::<Address32>(ptr).ok()?;

                Some(wram.into())
            }
        } else if core_name == SUPPORTED_CORES[3] {
            *endian = Endian::Little;

            // Picodrive
            if is_x86_64 {
                const SIG_64: Signature<9> = Signature::new("48 8D 0D ?? ?? ?? ?? 41 B8");

                let addr =
                    SIG_64.scan(game, (core_address, game.get_module_size(core_name).ok()?))? + 3;

                let wram = addr + 0x4 + game.read::<i32>(addr).ok()?;

                Some(wram)
            } else {
                const SIG_32: Signature<8> = Signature::new("B9 ?? ?? ?? ?? C1 EF 10");

                let ptr =
                    SIG_32.scan(game, (core_address, game.get_module_size(core_name).ok()?))? + 1;

                let wram = game.read::<Address32>(ptr).ok()?;

                Some(wram.into())
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
