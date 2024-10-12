use crate::{
    file_format::pe,
    signature::{Signature, SignatureScanner},
    Address, Address32, Address64, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<[Address; 2]> {
        const SUPPORTED_CORES: &[&str] = &[
            "vbam_libretro.dll",
            "mednafen_gba_libretro.dll",
            "vba_next_libretro.dll",
            "mgba_libretro.dll",
            "gpsp_libretro.dll",
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

        match core_name {
            "vbam_libretro.dll" | "vba_next_libretro.dll" | "mednafen_gba_libretro.dll" => {
                self.vba(game, is_64_bit, core_name)
            }
            "mgba_libretro.dll" => super::mgba::State::find_ram(&super::mgba::State, game),
            "gpsp_libretro.dll" => self.gpsp(game, is_64_bit, core_name),
            _ => None,
        }
    }

    fn vba(&self, game: &Process, is_64_bit: bool, core_name: &str) -> Option<[Address; 2]> {
        let module_range = (self.core_base, game.get_module_size(core_name).ok()?);

        if is_64_bit {
            const SIG: Signature<13> = Signature::new("48 8B 05 ?? ?? ?? ?? 81 E1 FF FF 03 00");
            const SIG2: Signature<13> = Signature::new("48 8B 05 ?? ?? ?? ?? 81 E1 FF 7F 00 00");

            let ewram_pointer = {
                let ptr: Address = SIG.scan(game, module_range.0, module_range.1)? + 3;
                let mut addr: Address = ptr + 0x4 + game.read::<i32>(ptr).ok()?;

                if game.read::<u8>(ptr + 10).ok()? == 0x48 {
                    addr = game.read::<Address64>(addr).ok()?.into();
                    if addr.is_null() {
                        return None;
                    }
                }

                addr
            };

            let iwram_pointer = {
                let ptr: Address = SIG2.scan(game, module_range.0, module_range.1)? + 3;
                let mut addr: Address = ptr + 0x4 + game.read::<i32>(ptr).ok()?;

                if game.read::<u8>(ptr + 10).ok()? == 0x48 {
                    addr = game.read::<Address64>(addr).ok()?.into();
                    if addr.is_null() {
                        return None;
                    }
                }

                addr
            };

            let ewram = game.read::<Address64>(ewram_pointer).ok()?;
            let iwram = game.read::<Address64>(iwram_pointer).ok()?;

            if ewram.is_null() || iwram.is_null() {
                None
            } else {
                Some([ewram.into(), iwram.into()])
            }
        } else {
            let ewram_pointer: Address = {
                const SIG: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 ?? FF FF 03 00");
                let ptr = SIG.scan(game, module_range.0, module_range.1)?;
                game.read::<Address32>(ptr + 1).ok()?.into()
            };
            let iwram_pointer: Address = {
                const SIG2: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 ?? FF 7F 00 00");
                let ptr = SIG2.scan(game, module_range.0, module_range.1)?;
                game.read::<Address32>(ptr + 1).ok()?.into()
            };

            let ewram = game.read::<Address32>(ewram_pointer).ok()?;
            let iwram = game.read::<Address32>(iwram_pointer).ok()?;

            if ewram.is_null() || iwram.is_null() {
                None
            } else {
                Some([ewram.into(), iwram.into()])
            }
        }
    }

    fn gpsp(&self, game: &Process, is_64_bit: bool, core_name: &str) -> Option<[Address; 2]> {
        const SIG_EWRAM: Signature<8> = Signature::new("25 FF FF 03 00 88 94 03");
        const SIG_IWRAM: Signature<9> = Signature::new("25 FE 7F 00 00 66 89 94 03");

        let module_size = game.get_module_size(core_name).ok()?;

        let base_addr: Address = match is_64_bit {
            true => {
                const SIG: Signature<10> = Signature::new("48 8B 15 ?? ?? ?? ?? 8B 42 40");
                let ptr = SIG.scan(game, self.core_base, module_size)? + 3;
                let ptr: Address = ptr + 0x4 + game.read::<i32>(ptr).ok()?;
                game.read::<Address64>(ptr).ok()?.into()
            }
            false => {
                const SIG: Signature<11> = Signature::new("A3 ?? ?? ?? ?? F7 C5 02 00 00 00");
                let ptr = SIG.scan(game, self.core_base, module_size)? + 1;
                game.read::<Address32>(ptr).ok()?.into()
            }
        };

        let ewram = {
            let offset = SIG_EWRAM.scan(game, self.core_base, module_size)? + 8;
            base_addr + game.read::<i32>(offset).ok()?
        };

        let iwram = {
            let offset = SIG_IWRAM.scan(game, self.core_base, module_size)? + 9;
            base_addr + game.read::<i32>(offset).ok()?
        };

        Some([ewram, iwram])
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
