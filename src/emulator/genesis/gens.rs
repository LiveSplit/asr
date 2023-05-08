use crate::{signature::Signature, Address, Address32, Endian, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_wram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        const SIG: Signature<10> = Signature::new("72 ?? 81 ?? FF FF 00 00 66 8B");

        let main_module = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Gens(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        let ptr = SIG.scan_process_range(game, main_module)? + 11;

        *endian = if game.read::<u8>(ptr + 4).ok()? == 0x86 {
            Endian::Big
        } else {
            Endian::Little
        };

        let wram = game.read::<Address32>(ptr).ok()?;

        Some(wram.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
