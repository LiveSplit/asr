//! Support for attaching to Game Boy Advance emulators.

use bytemuck::CheckedBitPattern;

use crate::{runtime, Address, Address32, Address64, Process};

/// A Game Boy Advance emulator that the auto splitter is attached to.
pub struct Emulator {
    process: Process,
    ewram: Address,
    iwram: Address,
}

impl Emulator {
    /// Attaches to a Game Boy Advance emulator. Currently only certain versions
    /// of mGBA and Visual Boy Advance on Windows are supported.
    pub fn attach() -> Option<Self> {
        look_for_mgba().or_else(look_for_vba)
    }

    /// Checks whether the emulator is still open. If it is not open anymore, you
    /// should drop the emulator.
    pub fn is_open(&self) -> bool {
        self.process.is_open()
    }

    /// Reads a value from the emulator's memory.
    pub fn read<T: CheckedBitPattern>(&self, addr: Address) -> Result<T, runtime::Error> {
        let address = addr.value();
        let memory_section = address >> 24;
        let ram_addr = match memory_section {
            2 => self.ewram,
            3 => self.iwram,
            _ => return Err(runtime::Error {}),
        };
        let addr = ram_addr + (address & 0xFF_FF_FF);
        self.process.read(addr)
    }
}

fn look_for_mgba() -> Option<Emulator> {
    let process = Process::attach("mGBA.exe")?;

    let [ewram, iwram]: [Address64; 2] = process
        .read_pointer_path64(
            process.get_module_address("mGBA.exe").ok()?,
            &[0x01D01868, 0x38, 0x10, 0x8, 0x28],
        )
        .ok()?;

    if ewram.is_null() || iwram.is_null() {
        return None;
    }

    Some(Emulator {
        process,
        ewram: ewram.into(),
        iwram: iwram.into(),
    })
}

fn look_for_vba() -> Option<Emulator> {
    let process = Process::attach("VisualBoyAdvance.exe")?;

    let [ewram, iwram]: [Address32; 2] =
        process.read(Address::new(0x00400000 + 0x001A8F50)).ok()?;

    if ewram.is_null() || iwram.is_null() {
        return None;
    }

    Some(Emulator {
        process,
        ewram: ewram.into(),
        iwram: iwram.into(),
    })
}
