//! Support for attaching to Nintendo Gamecube emulators.

use crate::{Address, Endian, Error, FromEndian, Process};
use bytemuck::CheckedBitPattern;

mod dolphin;
mod retroarch;

/// A Nintendo Gamecube emulator that the auto splitter is attached to.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: State,
    /// The memory address of the emulated RAM
    mem1_base: Option<Address>,
    /// The endianness used by the emulator process
    endian: Endian,
}

impl Emulator {
    /// Attaches to the emulator process
    ///
    /// Returns `Option<Genesis>` if successful, `None` otherwise.
    ///
    /// Supported emulators are:
    /// - Dolphin
    /// - Retroarch (using the `dolphin_libretro.dll` core)
    pub fn attach() -> Option<Self> {
        let (&state, process) = PROCESS_NAMES
            .iter()
            .find_map(|(name, state)| Some((state, Process::attach(name)?)))?;

        Some(Self {
            process,
            state,
            mem1_base: None,
            endian: Endian::Big, // Endianness is usually Big across all GCN emulators
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
        if self.mem1_base.is_none() {
            self.mem1_base = match match &mut self.state {
                State::Dolphin(x) => x.find_ram(&self.process, &mut self.endian),
                State::Retroarch(x) => x.find_ram(&self.process, &mut self.endian),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::Dolphin(x) => x.keep_alive(&self.process, &self.mem1_base),
            State::Retroarch(x) => x.keep_alive(&self.process, &self.mem1_base),
        };

        match success {
            true => true,
            false => {
                self.mem1_base = None;
                false
            },
        }
    }

    /// Reads raw data from the emulated RAM ignoring all endianness settings.
    /// The same call, performed on two different emulators, might return different
    /// results due to the endianness used by the emulator.
    ///
    /// The offset provided is meant to be the same used on the original,
    /// big-endian system.
    ///
    /// You can, alternatively, provide the memory address as usually mapped on the original hardware.
    /// Valid addresses for the Nintendo Gamecube range from `0x80000000` to `0x817FFFFF`.
    ///
    /// Values below and up to `0x017FFFFF` are automatically assumed to be offsets from the memory's base address.
    /// Any other invalid value will make this method immediately return `Err()`.
    ///
    /// This call is meant to be used by experienced users.
    pub fn read_ignoring_endianness<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0x017FFFFF && offset < 0x80000000) || offset > 0x817FFFFF {
            return Err(Error {});
        }

        let mem1 = self.mem1_base.ok_or(Error {})?;
        let end_offset = offset.checked_sub(0x80000000).unwrap_or(offset);

        self.process.read(mem1 + end_offset)
    }

    /// Reads any value from the emulated RAM.
    ///
    /// The offset provided is meant to be the same used on the original,
    /// big-endian system. The call will automatically convert the offset and
    /// the output value to little endian.
    ///
    /// You can, alternatively, provide the memory address as usually mapped on the original hardware.
    /// Valid addresses for the Nintendo Gamecube range from `0x80000000` to `0x817FFFFF`.
    ///
    /// Values below and up to `0x017FFFFF` are automatically assumed to be offsets from the memory's base address.
    /// Any other invalid value will make this method immediately return `Err()`.
    pub fn read<T: CheckedBitPattern + FromEndian>(&self, offset: u32) -> Result<T, Error> {
        Ok(self
            .read_ignoring_endianness::<T>(offset)?
            .from_endian(self.endian))
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    Dolphin(dolphin::State),
    Retroarch(retroarch::State),
}

static PROCESS_NAMES: [(&str, State); 2] = [
    ("Dolphin.exe", State::Dolphin(dolphin::State)),
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
];
