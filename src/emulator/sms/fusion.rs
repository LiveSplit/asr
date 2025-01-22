use crate::{signature::Signature, Address, Address32, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SIG: Signature<4> = Signature::new("74 C8 83 3D");

        let main_module = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Fusion(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let ptr = SIG.scan_once(game, main_module)? + 4;
        self.addr = game.read::<Address32>(ptr).ok()?.into();

        Some(game.read::<Address32>(self.addr).ok()?.add(0xC000).into())
    }

    pub fn keep_alive(&self, game: &Process, wram_base: &mut Option<Address>) -> bool {
        *wram_base = Some(match game.read::<Address32>(self.addr) {
            Ok(Address32::NULL) => Address::NULL,
            Ok(x) => x.add(0xC000).into(),
            _ => return false,
        });
        true
    }

    pub const fn new() -> Self {
        Self {
            addr: Address::NULL,
        }
    }
}
