use crate::{runtime::MemoryRangeFlags, signature::Signature, Address, Address32, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        const SIG: Signature<15> = Signature::new("66 81 E1 FF 1F 0F B7 C9 8A 89 ?? ?? ?? ?? C3");

        let scanned_address = game
            .memory_ranges()
            .filter(|m| {
                m.flags()
                    .unwrap_or_default()
                    .contains(MemoryRangeFlags::WRITE)
                    && m.size().unwrap_or_default() == 0x101000
            })
            .find_map(|m| SIG.scan_process_range(game, m.range().ok()?))?
            + 10;

        let wram: Address = game.read::<Address32>(scanned_address).ok()?.into();

        if wram.is_null() {
            None
        } else {
            Some(wram)
        }
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
