//! Support for attaching to Playstation 1 emulators.

use crate::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

mod duckstation;
mod epsxe;
mod mednafen;
mod pcsx_redux;
mod psxfin;
mod retroarch;
mod xebra;

/// A Playstation 1 emulator that the auto splitter is attached to.
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
    /// - ePSXe
    /// - pSX
    /// - Duckstation
    /// - Retroarch (supported cores: Beetle-PSX, Swanstation, PCSX ReARMed)
    /// - PCSX-redux
    /// - XEBRA
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
                State::Epsxe(x) => x.find_ram(&self.process),
                State::PsxFin(x) => x.find_ram(&self.process),
                State::Duckstation(x) => x.find_ram(&self.process),
                State::Retroarch(x) => x.find_ram(&self.process),
                State::PcsxRedux(x) => x.find_ram(&self.process),
                State::Xebra(x) => x.find_ram(&self.process),
                State::Mednafen(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::Epsxe(x) => x.keep_alive(),
            State::PsxFin(x) => x.keep_alive(),
            State::Duckstation(x) => x.keep_alive(&self.process, &mut self.ram_base),
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::PcsxRedux(x) => x.keep_alive(&self.process),
            State::Xebra(x) => x.keep_alive(),
            State::Mednafen(x) => x.keep_alive(),
        };

        if success {
            true
        } else {
            self.ram_base = None;
            false
        }
    }

    /// Reads any value from the emulated RAM.
    ///
    /// In PS1, memory addresses are usually mapped at fixed locations starting
    /// from `0x80000000`, and is the way many emulators, as well as the
    /// GameShark on original hardware, access memory.
    ///
    /// For this reason, this method will automatically convert offsets provided
    /// in such format. For example, providing an offset of `0x1234` or
    /// `0x80001234` will return the same value.
    ///
    /// Providing any offset outside the range of the PS1's RAM will return
    /// `Err()`.
    pub fn read<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0x1FFFFF && offset < 0x80000000) || offset > 0x801FFFFF {
            return Err(Error {});
        };

        let Some(ram_base) = self.ram_base else {
            return Err(Error {});
        };

        let end_offset = offset.checked_sub(0x80000000).unwrap_or(offset);

        self.process.read(ram_base + end_offset)
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum State {
    Epsxe(epsxe::State),
    PsxFin(psxfin::State),
    Duckstation(duckstation::State),
    Retroarch(retroarch::State),
    PcsxRedux(pcsx_redux::State),
    Xebra(xebra::State),
    Mednafen(mednafen::State),
}

const PROCESS_NAMES: [(&str, State); 8] = [
    ("ePSXe.exe", State::Epsxe(epsxe::State)),
    ("psxfin.exe", State::PsxFin(psxfin::State)),
    (
        "duckstation-qt-x64-ReleaseLTCG.exe",
        State::Duckstation(duckstation::State::new()),
    ),
    (
        "duckstation-nogui-x64-ReleaseLTCG.exe",
        State::Duckstation(duckstation::State::new()),
    ),
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
    (
        "pcsx-redux.main",
        State::PcsxRedux(pcsx_redux::State::new()),
    ),
    ("XEBRA.EXE", State::Xebra(xebra::State)),
    ("mednafen.exe", State::Mednafen(mednafen::State)),
];
