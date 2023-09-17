//! Support for attaching to Nintendo Gameboy Advance emulators.

use crate::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

mod mgba;
mod nocashgba;
mod retroarch;
mod vba;

/// A Nintendo Gameboy Advance emulator that the auto splitter is attached to.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: State,
    /// The memory address of the emulated RAM
    ram_base: Option<[Address; 2]>, // [ewram, iwram]
}

impl Emulator {
    /// Attaches to the emulator process
    ///
    /// Returns `Option<Genesis>` if successful, `None` otherwise.
    ///
    /// Supported emulators are:
    /// - VisualBoyAdvance
    /// - VisualBoyAdvance-M
    /// - mGBA
    /// - NO$GBA
    /// - Retroarch, with one of the following cores: `vbam_libretro.dll`, `vba_next_libretro.dll`,
    /// `mednafen_gba_libretro.dll`, `mgba_libretro.dll`, `gpsp_libretro.dll`
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
                State::VisualBoyAdvance(x) => x.find_ram(&self.process),
                State::Mgba(x) => x.find_ram(&self.process),
                State::NoCashGba(x) => x.find_ram(&self.process),
                State::Retroarch(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::VisualBoyAdvance(x) => x.keep_alive(&self.process, &mut self.ram_base),
            State::Mgba(x) => x.keep_alive(&self.process, &self.ram_base),
            State::NoCashGba(x) => x.keep_alive(&self.process, &mut self.ram_base),
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
    /// The offset provided is meant to be the same memory address as usually mapped on the original hardware.
    /// Valid addresses range:
    /// - from `0x02000000` to `0x0203FFFF` for EWRAM
    /// - from `0x03000000` to `0x03007FFF` for IWRAM
    ///
    /// Values outside these ranges will be considered invalid, and will make this method immediately return `Err()`.
    pub fn read<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        match offset >> 24 {
            2 => self.read_from_ewram(offset),
            3 => self.read_from_iwram(offset),
            _ => Err(Error {}),
        }
    }

    /// Reads any value from the EWRAM section of the emulated RAM.
    ///
    /// The offset provided can either be the relative offset from the
    /// start of EWRAM, or a memory address as mapped on the original hardware.
    ///
    /// Valid addresses range from `0x02000000` to `0x0203FFFF`.
    /// For example, providing an offset value of `0x3000` or `0x02003000`
    /// will return the exact same value.
    ///
    /// Invalid offset values, or values outside the allowed ranges will
    /// make this method immediately return `Err()`.
    pub fn read_from_ewram<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0x3FFFF && offset < 0x02000000) || offset > 0x0203FFFF {
            return Err(Error {});
        }

        let Some([ewram, _]) = self.ram_base else {
            return Err(Error {});
        };
        let end_offset = offset.checked_sub(0x02000000).unwrap_or(offset);

        self.process.read(ewram + end_offset)
    }

    /// Reads any value from the IWRAM section of the emulated RAM.
    ///
    /// The offset provided can either be the relative offset from the
    /// start of IWRAM, or a memory address as mapped on the original hardware.
    ///
    /// Valid addresses range from `0x03000000` to `0x03007FFF`.
    /// For example, providing an offset value of `0x3000` or `0x03003000`
    /// will return the exact same value.
    ///
    /// Invalid offset values, or values outside the allowed ranges will
    /// make this method immediately return `Err()`.
    pub fn read_from_iwram<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0x7FFF && offset < 0x03000000) || offset > 0x03007FFF {
            return Err(Error {});
        }

        let Some([_, iwram]) = self.ram_base else {
            return Err(Error {});
        };
        let end_offset = offset.checked_sub(0x03000000).unwrap_or(offset);

        self.process.read(iwram + end_offset)
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    VisualBoyAdvance(vba::State),
    Mgba(mgba::State),
    NoCashGba(nocashgba::State),
    Retroarch(retroarch::State),
}

static PROCESS_NAMES: [(&str, State); 5] = [
    (
        "visualboyadvance-m.exe",
        State::VisualBoyAdvance(vba::State::new()),
    ),
    (
        "VisualBoyAdvance.exe",
        State::VisualBoyAdvance(vba::State::new()),
    ),
    ("mGBA.exe", State::Mgba(mgba::State)),
    ("NO$GBA.EXE", State::NoCashGba(nocashgba::State::new())),
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
];
