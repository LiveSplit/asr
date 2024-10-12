use crate::{
    signature::{Signature, SignatureScanner},
    Address, Address32, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process) -> Option<Address> {
        const SIG: Signature<9> = Signature::new("8B 15 ?? ?? ?? ?? 8D 34 1A"); // v1.13
        const SIG_0: Signature<8> = Signature::new("A1 ?? ?? ?? ?? 8D 34 18"); // v1.12
        const SIG_1: Signature<9> = Signature::new("A1 ?? ?? ?? ?? 8B 7C 24 14"); // v1.5 through v1.11
        const SIG_2: Signature<8> = Signature::new("A1 ?? ?? ?? ?? 8B 6C 24"); // v1.0 through v1.4

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::PsxFin(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let mut ptr: Address32 =
            if let Some(sig) = SIG.scan(game, main_module_range.0, main_module_range.1) {
                game.read(sig + 2).ok()?
            } else if let Some(sig) = SIG_0.scan(game, main_module_range.0, main_module_range.1) {
                game.read(sig + 1).ok()?
            } else if let Some(sig) = SIG_1.scan(game, main_module_range.0, main_module_range.1) {
                game.read(sig + 1).ok()?
            } else if let Some(sig) = SIG_2.scan(game, main_module_range.0, main_module_range.1) {
                game.read(sig + 1).ok()?
            } else {
                return None;
            };

        ptr = game.read::<Address32>(ptr).ok()?;

        if ptr.is_null() {
            None
        } else {
            Some(ptr.into())
        }
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
