use crate::{Address, Endian, MemoryRangeFlags, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        for entry in game.memory_ranges() {
            if entry.size().is_ok_and(|size| size == 0x2000000)
                && entry.flags().is_ok_and(|flag| {
                    flag.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ)
                })
            {
                if let Ok(addr) = entry.address() {
                    *endian = Endian::Big;
                    return Some(addr);
                }
            }
        }
        None
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<Address>) -> bool {
        ram_base.is_some_and(|addr| game.read::<u8>(addr).is_ok())
    }
}
