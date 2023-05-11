use crate::{Address, signature::Signature, Process, Address64};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SIG: Signature<8> = Signature::new("48 89 0D ?? ?? ?? ?? B8");

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Duckstation(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let addr: Address = SIG.scan_process_range(game, main_module_range)? + 3;

        self.addr = addr + 0x4 + game.read::<i32>(addr).ok()? as i64;
        
        let ram = game.read::<Address64>(self.addr).ok()?;
        Some(ram.into())
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
            addr: Address::NULL
        }
    }
}