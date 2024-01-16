use crate::{file_format::pe, signature::Signature, Address, Address64, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let (&module_name, main_module) = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Duckstation(_)))
            .find_map(|(name, _)| Some((name, game.get_module_address(name).ok()?)))?;

        match module_name {
            "duckstation-qt-x64-ReleaseLTCG.exe" | "duckstation-nogui-x64-ReleaseLTCG.exe" => {
                const SIG: Signature<8> = Signature::new("48 89 0D ?? ?? ?? ?? B8");

                let main_module_size = pe::read_size_of_image(game, main_module)? as u64;

                // Recent Duckstation releases include a debug symbol that can be used to easily retrieve the address of the emulated RAM
                // Info: https://github.com/stenzek/duckstation/commit/c98e0bd0969abdd82589bfc565aea52119fd0f19
                if let Some(debug_symbol) = pe::symbols(game, main_module).find(|symbol| {
                    symbol
                        .get_name::<4>(game)
                        .is_ok_and(|name| name.matches(b"RAM"))
                }) {
                    self.addr = debug_symbol.address;
                } else {
                    // For older versions of Duckstation, we fall back to regular sigscanning
                    let addr = SIG.scan_process_range(game, (main_module, main_module_size))? + 3;
                    self.addr = addr + 0x4 + game.read::<i32>(addr).ok()?;
                }

                Some(game.read::<Address64>(self.addr).ok()?.into())
            }
            "duckstation-qt" => {
                const SIG: Signature<15> =
                    Signature::new("48 8B 05 ?? ?? ?? ?? 48 89 05 ?? ?? ?? ?? C3");
                let main_module_range = game.get_module_range("duckstation-qt").ok()?;
                let addr = SIG.scan_process_range(game, main_module_range)? + 10;
                self.addr = addr + 0x4 + game.read::<i32>(addr).ok()?;
                Some(game.read::<Address64>(self.addr).ok()?.into())
            }
            _ => None,
        }
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
