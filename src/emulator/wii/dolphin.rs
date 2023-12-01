use crate::{Address, Endian, FromEndian, MemoryRangeFlags, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State;

impl State {
    pub fn find_ram(&self, game: &Process, endian: &mut Endian) -> Option<[Address; 2]> {
        let mem_1 = game
            .memory_ranges()
            .find(|range| {
                range
                    .flags()
                    .is_ok_and(|r| r.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ))
                    && range.size().is_ok_and(|size| size == 0x2000000)
                    && range.address().is_ok_and(|addr| {
                        game.read::<[u32; 2]>(addr + 0x3118)
                            .is_ok_and(|val| val.from_endian(Endian::Big) == [0x4000000; 2])
                    })
            })?
            .address()
            .ok()?;

        let mem_2 = game
            .memory_ranges()
            .find(|range| {
                range
                    .flags()
                    .is_ok_and(|r| r.contains(MemoryRangeFlags::WRITE | MemoryRangeFlags::READ))
                    && range.size().is_ok_and(|size| size == 0x4000000)
                    && range
                        .address()
                        .is_ok_and(|addr| addr > mem_1 && addr < mem_1 + 0x10000000)
            })?
            .address()
            .ok()?;

        *endian = Endian::Big;
        Some([mem_1, mem_2])
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &Option<[Address; 2]>) -> bool {
        ram_base.is_some_and(|[mem1, mem2]| {
            game.read::<u8>(mem1).is_ok() && game.read::<u8>(mem2).is_ok()
        })
    }
}
