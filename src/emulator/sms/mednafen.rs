use crate::{
    file_format::pe,
    signature::{Signature, SignatureScanner},
    Address, Address32, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process) -> Option<Address> {
        const SIG_32: Signature<8> = Signature::new("25 FF 1F 00 00 0F B6 80");
        const SIG_64: Signature<7> = Signature::new("25 FF 1F 00 00 88 90");

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Mednafen(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let is_64_bit =
            pe::MachineType::read(game, main_module_range.0) == Some(pe::MachineType::X86_64);

        let ptr = match is_64_bit {
            true => SIG_64.scan(game, main_module_range)? + 8,
            false => SIG_32.scan(game, main_module_range)? + 7,
        };

        Some(game.read::<Address32>(ptr).ok()?.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
