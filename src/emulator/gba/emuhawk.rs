use crate::{Address, Process, MemoryRangeFlags};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    base_addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<[Address; 2]> {
        self.base_addr = game.get_module_address("mgba.dll").ok()?;

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

    pub fn keep_alive(&self, game: &Process) -> bool {
        game.read::<u8>(self.base_addr).is_ok()
    }

    pub const fn new() -> Self {
        Self {
            base_addr: Address::NULL,
        }
    }
}
