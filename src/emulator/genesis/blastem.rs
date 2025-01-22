use crate::{runtime::MemoryRangeFlags, signature::Signature, Address, Address32, Endian, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_wram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        const SIG: Signature<16> =
            Signature::new("72 0E 81 E1 FF FF 00 00 66 8B 89 ?? ?? ?? ?? C3");

        *endian = Endian::Little;

        let scanned_address = game
            .memory_ranges()
            .filter(|m| {
                m.flags()
                    .unwrap_or_default()
                    .contains(MemoryRangeFlags::WRITE)
                    && m.size().unwrap_or_default() == 0x101000
            })
            .find_map(|m| SIG.scan_once(game, m.range().ok()?))?
            + 11;

        let wram = game.read::<Address32>(scanned_address).ok()?;

        Some(wram.into())
    }

    pub const fn keep_alive(&self) -> bool {
        true
    }
}
