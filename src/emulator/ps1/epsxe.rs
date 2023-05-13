use crate::{Address, Process, signature::Signature};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process) -> Option<Address> {
        const SIG: Signature<5> = Signature::new("C1 E1 10 8D 89");

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Epsxe(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let ptr = SIG.scan_process_range(game, main_module_range)? + 5;

        Some(game.read::<u32>(ptr).ok()?.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}