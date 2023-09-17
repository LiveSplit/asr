use crate::{Address, MemoryRangeFlags, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<[Address; 2]> {
        // Latest version tested: 0.10.2 (September 2023)
        let addr = game
            .memory_ranges()
            .find(|range| {
                range.size().is_ok_and(|size| size == 0x48000)
                    && range.flags().is_ok_and(|flag| {
                        flag.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ)
                    })
            })?
            .address()
            .ok()?;
        Some([addr, addr + 0x40000])
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<[Address; 2]>) -> bool {
        ram_base.is_some_and(|[ewram, _]| game.read::<u8>(ewram).is_ok())
    }
}
