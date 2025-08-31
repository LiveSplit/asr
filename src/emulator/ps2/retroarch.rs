use crate::{file_format::pe, signature::Signature, Address, PointerSize, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    core_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SUPPORTED_CORES: &[&str] = &["pcsx2_libretro.dll"];

        let main_module_address = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Retroarch(_)))
            .find_map(|(name, _)| game.get_module_address(name).ok())?;

        // The LRPS2 core, the only one available for retroarch at
        // the time of writing (Sep 14th, 2023), only supports 64-bit
        let pointer_size = pe::MachineType::read(game, main_module_address)?.pointer_size()?;
        if !pointer_size.eq(&PointerSize::Bit64) {
            return None;
        }

        let (_core_name, core_address) = SUPPORTED_CORES
            .iter()
            .find_map(|&m| Some((m, game.get_module_address(m).ok()?)))?;

        let base_scan = pe::symbols(game, core_address)
            .find(|symbol| {
                symbol
                    .get_name::<22>(game)
                    .is_ok_and(|name| name.matches(b"retro_get_memory_data"))
            })?
            .address;

        self.core_base = core_address;
        const SIG: Signature<5> = Signature::new("?? ?? ?? ?? C3");
        SIG.scan_process_range(game, (base_scan, 0x100))
            .and_then(|addr| Some(addr + 0x4 + game.read::<i32>(addr).ok()?))
            .and_then(|addr| game.read_pointer(addr, pointer_size).ok())
            .filter(|addr| !addr.is_null())
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
