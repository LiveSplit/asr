//! Support for attaching to SEGA Genesis emulators.

use core::mem;

use crate::{Address, Endian, Error, FromEndian, Process};
use bytemuck::CheckedBitPattern;

mod blastem;
mod fusion;
mod gens;
mod retroarch;
mod segaclassics;

/// A SEGA Genesis emulator that the auto splitter is attached to.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: State,
    /// The memory address of the emulated RAM
    wram_base: Option<Address>,
    /// The endianness used by the emulator process
    endian: Endian,
}

impl Emulator {
    /// Attaches to the emulator process
    ///
    /// Returns `Option<Genesis>` if successful, `None` otherwise.
    ///
    /// Supported emulators are:
    /// - Retroarch
    /// - SEGA Classics / SEGA Game Room
    /// - Fusion
    /// - Gens
    /// - BlastEm
    pub fn attach() -> Option<Self> {
        let (&state, process) = PROCESS_NAMES
            .iter()
            .find_map(|(name, state)| Some((state, Process::attach(name)?)))?;

        Some(Self {
            process,
            state,
            wram_base: None,
            endian: Endian::Little, // Endianness is supposed to be Little, until stated otherwise in the code
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
        if self.wram_base.is_none() {
            self.wram_base = match match &mut self.state {
                State::Retroarch(x) => x.find_wram(&self.process, &mut self.endian),
                State::SegaClassics(x) => x.find_wram(&self.process, &mut self.endian),
                State::Fusion(x) => x.find_wram(&self.process, &mut self.endian),
                State::Gens(x) => x.find_wram(&self.process, &mut self.endian),
                State::BlastEm(x) => x.find_wram(&self.process, &mut self.endian),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::SegaClassics(x) => x.keep_alive(&self.process, &mut self.wram_base),
            State::Fusion(x) => x.keep_alive(&self.process, &mut self.wram_base),
            State::Gens(x) => x.keep_alive(),
            State::BlastEm(x) => x.keep_alive(),
        };

        if success {
            true
        } else {
            self.wram_base = None;
            false
        }
    }

    /// Reads raw data from the emulated RAM ignoring all endianess settings The
    /// same call, performed on two different emulators, can be different due to
    /// the endianness used by the emulator.
    ///
    /// The offset provided must not be higher than `0xFFFF`, otherwise this
    /// method will immediately return `Err()`.
    ///
    /// This call is meant to be used by experienced users.
    pub fn read_ignoring_endianness<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if offset > 0xFFFF {
            return Err(Error {});
        }

        let wram = self.wram_base.ok_or(Error {})?;

        self.process.read(wram + offset)
    }

    /// Reads any value from the emulated RAM.
    ///
    /// The offset provided is meant to be the same used on the original,
    /// big-endian system. The call will automatically convert the offset and
    /// the output value to little endian.
    ///
    /// The offset provided must not be higher than `0xFFFF`, otherwise this
    /// method will immediately return `Err()`.
    pub fn read<T: CheckedBitPattern + FromEndian>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0xFFFF && offset <= 0xFF0000) || offset > 0xFFFFFF {
            return Err(Error {});
        }

        let wram = self.wram_base.ok_or(Error {})?;

        let toggle = self.endian == Endian::Little && mem::size_of::<T>() == 1;
        let end_offset = offset ^ toggle as u32;

        let Ok(value) = self.process.read::<T>(wram + end_offset) else { return Err(Error {}) };
        Ok(value.from_endian(self.endian))
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    Retroarch(retroarch::State),
    SegaClassics(segaclassics::State),
    Fusion(fusion::State),
    Gens(gens::State),
    BlastEm(blastem::State),
}

static PROCESS_NAMES: [(&str, State); 6] = [
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
    (
        "SEGAGameRoom.exe",
        State::SegaClassics(segaclassics::State::new()),
    ),
    (
        "SEGAGenesisClassics.exe",
        State::SegaClassics(segaclassics::State::new()),
    ),
    ("Fusion.exe", State::Fusion(fusion::State::new())),
    ("gens.exe", State::Gens(gens::State)),
    ("blastem.exe", State::BlastEm(blastem::State)),
];
