use crate::{file_format::pe, signature::Signature, Address, Address32, PointerSize, Process};

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
            pe::MachineType::read(game, main_module_range.0)?.pointer_size()? == PointerSize::Bit64;

        match is_64_bit {
            true => SIG_64.scan_process_range(game, main_module_range),
            false => SIG_32.scan_process_range(game, main_module_range),
        }
        .map(|val| val + 0x5)
        .and_then(|addr| game.read::<Address32>(addr).ok())
        .map(|val| val.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
