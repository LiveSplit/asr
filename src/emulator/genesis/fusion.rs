use crate::{
    signature::{Signature, SignatureScanner},
    Address, Address32, Endian, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr: Address,
}

impl State {
    pub fn find_wram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        const SIG: Signature<4> = Signature::new("75 2F 6A 01");

        let main_module = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Fusion(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let ptr = SIG.scan(game, main_module)? + 1;

        let addr = ptr + game.read::<u8>(ptr).ok()? as u64 + 3;
        let addr = game.read::<Address32>(addr).ok()?;

        self.addr = addr.into();

        let addr = game.read::<Address32>(self.addr).ok()?;

        *endian = Endian::Big;

        Some(addr.into())
    }

    pub fn keep_alive(&self, game: &Process, wram_base: &mut Option<Address>) -> bool {
        if let Ok(addr) = game.read::<Address32>(self.addr) {
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
