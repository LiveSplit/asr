use crate::{
    file_format::pe::{self, MachineType},
    signature::Signature,
    Address, Address32, Address64, Endian, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_wram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        const SUPPORTED_CORES: &[&str] = &[
            "blastem_libretro.dll",
            "genesis_plus_gx_libretro.dll",
            "genesis_plus_gx_wide_libretro.dll",
            "picodrive_libretro.dll",
            "blastem_libretro.so",
            "genesis_plus_gx_libretro.so",
            "genesis_plus_gx_wide_libretro.so",
            "picodrive_libretro.so",
        ];

        let (core_name, core_address) = SUPPORTED_CORES
            .iter()
            .find_map(|&m| Some((m, game.get_module_address(m).ok()?)))?;

        self.core_base = core_address;

        match core_name {
            "blastem_libretro.dll" => {
                *endian = Endian::Little;
                match pe::MachineType::read(game, core_address) {
                    Some(MachineType::X86_64) => {
                        const SIG: Signature<15> =
                            Signature::new("48 8B 05 ?? ?? ?? ?? 48 8B 80 ?? ?? ?? ?? C3");

                        let ptr = SIG.scan_process_range(
                            game,
                            (
                                core_address,
                                pe::read_size_of_image(game, core_address)? as u64,
                            ),
                        )?;

                        let mut addr = ptr + 3 + 0x4 + game.read::<i32>(ptr + 3).ok()?;
                        addr = game.read::<Address64>(addr).ok()?.into();
                        addr = addr + game.read::<i32>(ptr + 10).ok()?;
                        addr = game.read::<Address64>(addr).ok()?.into();
                        Some(addr)
                    }
                    Some(_) => {
                        const SIG: Signature<14> =
                            Signature::new("A1 ?? ?? ?? ?? 8B ?? ?? ?? ?? ?? 89 ?? C3");

                        let ptr = SIG.scan_process_range(
                            game,
                            (
                                core_address,
                                pe::read_size_of_image(game, core_address)? as u64,
                            ),
                        )?;

                        let mut addr = game.read::<Address32>(ptr + 1).ok()?;
                        addr = game.read::<Address32>(addr).ok()?;
                        addr = game
                            .read::<Address32>(addr + game.read::<i32>(ptr + 7).ok()?)
                            .ok()?;
                        Some(addr.into())
                    }
                    _ => None,
                }
            }
            "genesis_plus_gx_libretro.dll" | "genesis_plus_gx_wide_libretro.dll" => {
                *endian = Endian::Little;

                let main_module_address = super::PROCESS_NAMES
                    .iter()
                    .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
                    .find_map(|(name, _)| game.get_module_address(name).ok())?;

                let is_x86_64 =
                    pe::MachineType::read(game, main_module_address)? == pe::MachineType::X86_64;

                // Genesis plus GX
                if is_x86_64 {
                    const SIG_64: Signature<10> = Signature::new("48 8D 0D ?? ?? ?? ?? 4C 8B 2D");

                    let addr = SIG_64.scan_process_range(
                        game,
                        (
                            core_address,
                            pe::read_size_of_image(game, core_address)? as u64,
                        ),
                    )? + 3;

                    let wram = addr + 0x4 + game.read::<i32>(addr).ok()?;

                    Some(wram)
                } else {
                    const SIG_32: Signature<7> = Signature::new("A3 ?? ?? ?? ?? 29 F9");

                    let ptr = SIG_32.scan_process_range(
                        game,
                        (
                            core_address,
                            pe::read_size_of_image(game, core_address)? as u64,
                        ),
                    )? + 1;

                    let wram = game.read::<Address32>(ptr).ok()?;

                    Some(wram.into())
                }
            }
            "picodrive_libretro.dll" => {
                *endian = Endian::Little;

                let main_module_address = super::PROCESS_NAMES
                    .iter()
                    .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
                    .find_map(|(name, _)| game.get_module_address(name).ok())?;

                let is_x86_64 =
                    pe::MachineType::read(game, main_module_address)? == pe::MachineType::X86_64;

                // Picodrive
                if is_x86_64 {
                    const SIG_64: Signature<9> = Signature::new("48 8D 0D ?? ?? ?? ?? 41 B8");

                    let addr = SIG_64.scan_process_range(
                        game,
                        (
                            core_address,
                            pe::read_size_of_image(game, core_address)? as u64,
                        ),
                    )? + 3;

                    let wram = addr + 0x4 + game.read::<i32>(addr).ok()?;

                    Some(wram)
                } else {
                    const SIG_32: Signature<8> = Signature::new("B9 ?? ?? ?? ?? C1 EF 10");

                    let ptr = SIG_32.scan_process_range(
                        game,
                        (
                            core_address,
                            pe::read_size_of_image(game, core_address)? as u64,
                        ),
                    )? + 1;

                    let wram = game.read::<Address32>(ptr).ok()?;

                    Some(wram.into())
                }
            }
            "blastem_libretro.so" => {
                *endian = Endian::Little;

                const SIG: Signature<18> =
                    Signature::new("48 8B 05 ?? ?? ?? ?? 48 8B 00 48 8B 80 ?? ?? ?? ?? C3");

                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )?;

                let mut addr = ptr + 3 + 0x4 + game.read::<i32>(ptr + 3).ok()?;
                addr = game.read::<Address64>(addr).ok()?.into();
                addr = game.read::<Address64>(addr).ok()?.into();
                addr = addr + game.read::<i32>(ptr + 13).ok()?;
                addr = game.read::<Address64>(addr).ok()?.into();
                Some(addr)
            }
            "genesis_plus_gx_libretro.so" | "genesis_plus_gx_wide_libretro.so" => {
                *endian = Endian::Little;

                const SIG: Signature<9> = Signature::new("C3 48 8D 05 ?? ?? ?? ?? C3");

                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 4;
                let wram = ptr + 0x4 + game.read::<i32>(ptr).ok()?;
                Some(wram)
            }
            "picodrive_libretro.so" => {
                *endian = Endian::Little;

                const SIG: Signature<11> = Signature::new("48 8B 05 ?? ?? ?? ?? 74 ?? 48 05");

                let ptr = SIG.scan_process_range(
                    game,
                    (core_address, game.get_module_size(core_name).ok()?),
                )? + 3;
                let wram = game
                    .read::<Address64>(ptr + 0x4 + game.read::<i32>(ptr).ok()?)
                    .ok()?;
                Some(wram.into())
            }
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
