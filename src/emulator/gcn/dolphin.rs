use crate::{Address, Endian, FromEndian, MemoryRangeFlags, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        // Main logic: finding the address for the GCN main memory by looking for
        // memory ranges with the READ and WRITE flags and a size of 0x2000000
        for entry in game.memory_ranges() {
            if entry.size().is_ok_and(|size| size == 0x2000000)
                && entry.flags().is_ok_and(|flag| {
                    flag.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ)
                })
            {
                if let Ok(addr) = entry.address() {
                    // In order to verify this is the correct memory range, we take advantage
                    // of a small 'hack', by checking if the offset 0x1C contains a "magic number"
                    // fixed for all Gamecube games.
                    if game
                        .read::<u32>(addr + 0x1C)
                        .is_ok_and(|magic| magic.from_endian(Endian::Big) == 0xC2339F3D)
                    {
                        *endian = Endian::Big;
                        return Some(addr);
                    }
                }
            }
        }
        None
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<Address>) -> bool {
        ram_base.is_some_and(|addr| game.read::<u8>(addr).is_ok())
    }
}
