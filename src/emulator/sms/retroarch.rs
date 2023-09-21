use crate::{file_format::pe, signature::Signature, Address, Address32, Address64, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SUPPORTED_CORES: &[&str] = &[
            "genesis_plus_gx_libretro.dll",
            "genesis_plus_gx_wide_libretro.dll",
            "picodrive_libretro.dll",
            "smsplus_libretro.dll",
            "gearsystem_libretro.dll",
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
            "genesis_plus_gx_libretro.dll" | "genesis_plus_gx_wide_libretro.dll" => {
                self.genesis_plus(game, is_64_bit, core_name)
            }
            "picodrive_libretro.dll" => self.picodrive(game, is_64_bit, core_name),
            "smsplus_libretro.dll" => self.sms_plus(game, is_64_bit, core_name),
            "gearsystem_libretro.dll" => self.gearsystem(game, is_64_bit, core_name),
            _ => None,
        }
    }

    fn picodrive(&self, game: &Process, is_64_bit: bool, core_name: &str) -> Option<Address> {
        let module_size = game.get_module_size(core_name).ok()?;

        Some(
            if is_64_bit {
                const SIG: Signature<9> = Signature::new("48 8D 0D ?? ?? ?? ?? 41 B8");
                let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 3;
                ptr + 0x4 + game.read::<i32>(ptr).ok()?
            } else {
                const SIG: Signature<8> = Signature::new("B9 ?? ?? ?? ?? C1 EF 10");
                let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 1;
                game.read::<Address32>(ptr).ok()?.into()
            } + 0x20000,
        )
    }

    fn genesis_plus(&self, game: &Process, is_64_bit: bool, core_name: &str) -> Option<Address> {
        let module_size = game.get_module_size(core_name).ok()?;

        Some(if is_64_bit {
            const SIG: Signature<10> = Signature::new("48 8D 0D ?? ?? ?? ?? 4C 8B 2D");
            let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 3;
            ptr + 0x4 + game.read::<i32>(ptr).ok()?
        } else {
            const SIG: Signature<7> = Signature::new("A3 ?? ?? ?? ?? 29 F9");
            let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 1;
            game.read::<Address32>(ptr).ok()?.into()
        })
    }

    fn sms_plus(&self, game: &Process, is_64_bit: bool, core_name: &str) -> Option<Address> {
        let module_size = game.get_module_size(core_name).ok()?;

        Some(if is_64_bit {
            const SIG: Signature<5> = Signature::new("31 F6 48 C7 05");
            let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 5;
            ptr + 0x8 + game.read::<i32>(ptr).ok()?
        } else {
            const SIG: Signature<4> = Signature::new("83 FA 02 B8");
            let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 4;
            game.read::<Address32>(ptr).ok()?.into()
        })
    }

    fn gearsystem(&self, game: &Process, is_64_bit: bool, core_name: &str) -> Option<Address> {
        let module_size = game.get_module_size(core_name).ok()?;

        Some(if is_64_bit {
            const SIG: Signature<13> = Signature::new("83 ?? 02 75 ?? 48 8B 0D ?? ?? ?? ?? E8");
            let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 8;
            let offset = game
                .read::<u8>(ptr + 13 + 0x4 + game.read::<i32>(ptr + 13).ok()? + 3)
                .ok()?;
            let addr = game
                .read_pointer_path64::<Address64>(
                    ptr + 0x4 + game.read::<i32>(ptr).ok()?,
                    &[0x0, 0x0, offset as _],
                )
                .ok()?;
            if addr.is_null() {
                return None;
            } else {
                addr.add(0xC000).into()
            }
        } else {
            const SIG: Signature<12> = Signature::new("83 ?? 02 75 ?? 8B ?? ?? ?? ?? ?? E8");
            let ptr = SIG.scan_process_range(game, (self.core_base, module_size))? + 7;
            let offset = game
                .read::<u8>(ptr + 12 + 0x4 + game.read::<i32>(ptr + 12).ok()? + 2)
                .ok()?;
            let addr = game
                .read_pointer_path32::<Address32>(ptr, &[0x0, 0x0, 0x0, offset as _])
                .ok()?;
            if addr.is_null() {
                return None;
            } else {
                addr.add(0xC000).into()
            }
        })
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
