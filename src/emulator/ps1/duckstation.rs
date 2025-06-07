use crate::{file_format::pe, signature::Signature, Address, Address64, PointerSize, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Duckstation(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        // 32bit versions of Duckstation are not supported (nor developed)
        if pe::MachineType::read(game, main_module_range.0)?.pointer_size()? != PointerSize::Bit64 {
            return None;
        }

        // Recent Duckstation releases include a debug symbol that can be used to easily retrieve the address of the emulated RAM
        // Info: https://github.com/stenzek/duckstation/commit/c98e0bd0969abdd82589bfc565aea52119fd0f19
        if let Some(debug_symbol) = pe::symbols(game, main_module_range.0).find(|symbol| {
            symbol
                .get_name::<4>(game)
                .is_ok_and(|name| name.matches(b"RAM"))
        }) {
            self.addr = debug_symbol.address;
        } else {
            // For older versions of Duckstation, we fall back to regular sigscanning
            const SIG: Signature<8> = Signature::new("48 89 0D ?? ?? ?? ?? B8");
            self.addr = SIG
                .scan_process_range(game, main_module_range)
                .map(|val| val + 3)
                .and_then(|addr| Some(addr + 0x4 + game.read::<i32>(addr).ok()?))?;
        }

        Some(game.read::<Address64>(self.addr).ok()?.into())
    }

    pub fn keep_alive(&self, game: &Process, wram_base: &mut Option<Address>) -> bool {
        if let Ok(addr) = game.read::<Address64>(self.addr) {
            *wram_base = Some(addr.into());
            true
        } else {
            false
        }
    }

    pub const fn new() -> Self {
        Self {
            addr: Address::NULL,
        }
    }
}
