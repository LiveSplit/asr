//! Support for attaching to SEGA Master System / SEGA GameGear emulators.

use crate::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

mod blastem;
mod fusion;
mod retroarch;

/// A SEGA Master System / GameGear emulator that the auto splitter is attached to.
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
    /// - Retroarch, with one of the following cores: `genesis_plus_gx_libretro.dll`,
    /// `genesis_plus_gx_wide_libretro.dll`, `picodrive_libretro.dll`, `smsplus_libretro.dll`, `gearsystem_libretro.dll`
    /// - Fusion
    /// - BlastEm
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
                State::Retroarch(x) => x.find_ram(&self.process),
                State::Fusion(x) => x.find_ram(&self.process),
                State::BlastEm(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::Fusion(x) => x.keep_alive(&self.process, &mut self.ram_base),
            State::BlastEm(x) => x.keep_alive(),
        };

        if success {
            true
        } else {
            self.ram_base = None;
            false
        }
    }

    /// Reads raw data from the emulated RAM ignoring all endianness settings
    /// The same call, performed on two different emulators, can be different
    /// due to the endianness used by the emulator.
    ///
    /// The offset provided must not be higher than `0xFFFF`, otherwise this
    /// method will immediately return `Err()`.
    ///
    /// This call is meant to be used by experienced users.
    pub fn read_ignoring_endianness<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if offset > 0xFFFF {
            return Err(Error {});
        }

        let wram = self.ram_base.ok_or(Error {})?;

        self.process.read(wram + offset)
    }

    /// Reads any value from the emulated RAM.
    ///
    /// The offset provided is meant to be the same used on the original,
    /// big-endian system.
    ///
    /// The SEGA Master System has 8KB of RAM, mapped from address
    /// `0xC000` to `0xDFFF`.
    ///
    /// Providing any offset outside this range will return `Err()`.
    pub fn read<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0x1FFF && offset < 0xC000) || offset > 0xDFFF {
            return Err(Error {});
        }

        let wram = self.ram_base.ok_or(Error {})?;
        let end_offset = offset.checked_sub(0xC000).unwrap_or(offset);

        self.process.read(wram + end_offset)
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    Retroarch(retroarch::State),
    Fusion(fusion::State),
    BlastEm(blastem::State),
}

static PROCESS_NAMES: &[(&str, State)] = &[
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
    ("Fusion.exe", State::Fusion(fusion::State::new())),
    ("blastem.exe", State::BlastEm(blastem::State)),
];
