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

        let ptr = SIG.scan_process_range(game, main_module_range)? + 1;
        let addr = ptr + 0x4 + game.read::<i32>(ptr).ok()?;
        let addr = game.read::<Address32>(addr + 0x16A).ok()?;
        let addr = game.read::<Address32>(addr).ok()?;
        Some(addr.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
