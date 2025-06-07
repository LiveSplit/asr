use crate::{signature::Signature, Address, Address32, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process) -> Option<Address> {
        const SIG: Signature<15> = Signature::new("E8 ?? ?? ?? ?? E9 ?? ?? ?? ?? 89 C8 C1 F8 10");

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Xebra(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        SIG.scan_process_range(game, main_module_range)
            .map(|addr| addr + 1)
            .and_then(|addr| Some(addr + 0x4 + game.read::<i32>(addr).ok()?))
            .and_then(|addr| game.read::<Address32>(addr + 0x16A).ok())
            .and_then(|addr| game.read::<Address32>(addr).ok())
            .map(|addr| addr.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
