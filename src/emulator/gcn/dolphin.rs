use crate::{Address, Endian, FromEndian, MemoryRangeFlags, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process, endian: &mut Endian) -> Option<Address> {
        *endian = Endian::Big;

        // Main logic: finding the address for the GCN main memory by looking for
        // memory ranges with the READ and WRITE flags and a size of 0x2000000.
        // In order to verify we found the correct memory range, we take advantage
        // of a small 'hack', by checking if the offset 0x1C contains a "magic number"
        // fixed for all Gamecube games.

        game.memory_ranges()
            .find(|range| {
                range
                    .flags()
                    .is_ok_and(|r| r.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ))
                    && range.size().is_ok_and(|size| size == 0x2000000)
                    && range.address().is_ok_and(|addr| {
                        game.read::<u32>(addr + 0x1C)
                            .is_ok_and(|magic| magic.from_endian(Endian::Big) == 0xC2339F3D)
                    })
            })?
            .address()
            .ok()
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<Address>) -> bool {
        ram_base.is_some_and(|addr| game.read::<u8>(addr).is_ok())
    }
}
