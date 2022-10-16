use bytemuck::Pod;

use crate::{runtime, Address, Process};

pub struct Emulator {
    process: Process,
    ewram: u32,
    iwram: u32,
}

impl Emulator {
    pub fn attach() -> Option<Self> {
        look_for_vba()
    }

    pub fn is_open(&self) -> bool {
        self.process.is_open()
    }

    pub fn read<T: Pod>(&self, address: u32) -> Result<T, runtime::Error> {
        let memory_section = address >> 24;
        let ram_addr = match memory_section {
            2 => self.ewram,
            3 => self.iwram,
            _ => return Err(runtime::Error),
        };
        let addr = ram_addr + (address & 0xFF_FF_FF);
        self.process.read(Address(addr as u64))
    }
}

fn look_for_vba() -> Option<Emulator> {
    let process = Process::attach("VisualBoyAdvance.exe")?;
    let [ewram, iwram]: [u32; 2] = process.read(Address(0x00400000 + 0x001A8F50)).ok()?;
    if ewram == 0 || iwram == 0 {
        return None;
    }
    Some(Emulator {
        process,
        ewram,
        iwram,
    })
}
