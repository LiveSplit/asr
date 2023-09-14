//! Support for attaching to Playstation 2 emulators.

use crate::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

mod pcsx2;
mod retroarch;

/// A Playstation 2 emulator that the auto splitter is attached to.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: State,
    /// The memory address of the emulated RAM
    ram_base: Option<Address>,
}

impl Emulator {
    /// Attaches to the emulator process
    ///
    /// Returns `Option<T>` if successful, `None` otherwise.
    ///
    /// Supported emulators are:
    /// - PCSX2
    /// - Retroarch (64-bit version, using the `pcsx2_libretro.dll` core)
    pub fn attach() -> Option<Self> {
        let (&state, process) = PROCESS_NAMES
            .iter()
            .find_map(|(name, state)| Some((state, Process::attach(name)?)))?;

        Some(Self {
            process,
            state,
            ram_base: None,
        })
    }

    /// Checks whether the emulator is still open. If it is not open anymore,
    /// you should drop the emulator.
    pub fn is_open(&self) -> bool {
        self.process.is_open()
    }

    /// Calls the internal routines needed in order to find (and update, if
    /// needed) the address of the emulated RAM.
    ///
    /// Returns true if successful, false otherwise.
    pub fn update(&mut self) -> bool {
        if self.ram_base.is_none() {
            self.ram_base = match match &mut self.state {
                State::Pcsx2(x) => x.find_ram(&self.process),
                State::Retroarch(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::Pcsx2(x) => x.keep_alive(&self.process, &mut self.ram_base),
            State::Retroarch(x) => x.keep_alive(&self.process),
        };

        match success {
            true => true,
            false => {
                self.ram_base = None;
                false
            },
        }
    }

    /// Reads any value from the emulated RAM.
    ///
    /// In PS2, memory addresses are mapped at fixed locations starting
    /// from `0x00100000` (addresses below this threashold are
    /// reserved for the kernel).
    ///
    /// Valid addresses for the PS2's memory range from `0x00100000` to `0x01FFFFFF`
    ///
    /// Providing any offset outside the range of the PS2's RAM will return
    /// `Err()`.
    pub fn read<T: CheckedBitPattern>(&self, address: u32) -> Result<T, Error> {
        if address < 0x00100000 || address > 0x01FFFFFF {
            return Err(Error {});
        }

        let Some(ram_base) = self.ram_base else {
            return Err(Error {});
        };

        self.process.read(ram_base + address)
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum State {
    Pcsx2(pcsx2::State),
    Retroarch(retroarch::State),
}

const PROCESS_NAMES: [(&str, State); 6] = [
    ("pcsx2x64.exe", State::Pcsx2(pcsx2::State::new())),
    ("pcsx2-qt.exe", State::Pcsx2(pcsx2::State::new())),
    ("pcsx2x64-avx2.exe", State::Pcsx2(pcsx2::State::new())),
    ("pcsx2-avx2.exe", State::Pcsx2(pcsx2::State::new())),
    ("pcsx2.exe", State::Pcsx2(pcsx2::State::new())),
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
];
