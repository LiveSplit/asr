use crate::{file_format::pe, signature::Signature, Address, Address32, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process) -> Option<Address> {
        const SIG_32: Signature<10> = Signature::new("89 01 0F B6 82 ?? ?? ?? ?? C3");
        const SIG_64: Signature<5> = Signature::new("89 01 0F B6 82");

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Mednafen(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let is_64_bit =
            pe::MachineType::read(game, main_module_range.0) == Some(pe::MachineType::X86_64);

        let ptr = match is_64_bit {
            true => SIG_64.scan_once(game, main_module_range)?,
            false => SIG_32.scan_once(game, main_module_range)?,
        } + 0x5;

        Some(game.read::<Address32>(ptr).ok()?.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
