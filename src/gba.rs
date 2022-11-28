use bytemuck::Pod;

use crate::{runtime, Address, Process};

pub struct Emulator {
    process: Process,
    ewram: u64,
    iwram: u64,
}

impl Emulator {
    pub fn attach() -> Option<Self> {
        look_for_mgba().or_else(look_for_vba)
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
        let addr = ram_addr + (address & 0xFF_FF_FF) as u64;
        self.process.read(Address(addr))
    }
}

fn look_for_mgba() -> Option<Emulator> {
    let process = Process::attach("mGBA.exe")?;

    let [ewram, iwram]: [u64; 2] = process
        .read_pointer_path64(
            process.get_module_address("mGBA.exe").ok()?.0,
            &[0x01D01868, 0x38, 0x10, 0x8, 0x28],
        )
        .ok()?;

    if ewram == 0 || iwram == 0 {
        return None;
    }

    Some(Emulator {
        process,
        ewram,
        iwram,
    })
}

fn look_for_vba() -> Option<Emulator> {
    let process = Process::attach("VisualBoyAdvance.exe")?;
    let [ewram, iwram]: [u32; 2] = process.read(Address(0x00400000 + 0x001A8F50)).ok()?;
    if ewram == 0 || iwram == 0 {
        return None;
    }
    Some(Emulator {
        process,
        ewram: ewram as u64,
        iwram: iwram as u64,
    })
}
