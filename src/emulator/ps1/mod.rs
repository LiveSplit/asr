//! Support for attaching to Playstation 1 emulators.

use core::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{future::retry, Address, Error, Process};
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
    state: Cell<State>,
    /// The memory address of the emulated RAM
    ram_base: Cell<Option<Address>>,
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
    /// - Mednafen
    /// - Retroarch (supported cores: Beetle-PSX, Swanstation, PCSX ReARMed)
    /// - PCSX-redux
    /// - XEBRA
    pub fn attach() -> Option<Self> {
        let (&state, process) = PROCESS_NAMES
            .iter()
            .find_map(|(name, state)| Some((state, Process::attach(name)?)))?;

        Some(Self {
            process,
            state: Cell::new(state),
            ram_base: Cell::new(None),
        })
    }

    /// Asynchronously awaits attaching to a target emulator,
    /// yielding back to the runtime between each try.
    ///
    /// Supported emulators are:
    /// - ePSXe
    /// - pSX
    /// - Duckstation
    /// - Mednafen
    /// - Retroarch (supported cores: Beetle-PSX, Swanstation, PCSX ReARMed)
    /// - PCSX-redux
    /// - XEBRA
    pub async fn wait_attach() -> Self {
        retry(Self::attach).await
    }

    /// Checks whether the emulator is still open. If it is not open anymore,
    /// you should drop the emulator.
    pub fn is_open(&self) -> bool {
        self.process.is_open()
    }

    /// Executes a future until the emulator process closes.
    pub const fn until_closes<F>(&self, future: F) -> UntilEmulatorCloses<'_, F> {
        UntilEmulatorCloses {
            emulator: self,
            future,
        }
    }

    /// Calls the internal routines needed in order to find (and update, if
    /// needed) the address of the emulated RAM.
    ///
    /// Returns true if successful, false otherwise.
    pub fn update(&self) -> bool {
        let mut ram_base = self.ram_base.get();
        let mut state = self.state.get();

        if ram_base.is_none() {
            ram_base = match match &mut state {
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

        let success = match &state {
            State::Epsxe(x) => x.keep_alive(),
            State::PsxFin(x) => x.keep_alive(),
            State::Duckstation(x) => x.keep_alive(&self.process, &mut ram_base),
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::PcsxRedux(x) => x.keep_alive(&self.process),
            State::Xebra(x) => x.keep_alive(),
            State::Mednafen(x) => x.keep_alive(),
        };

        self.state.set(state);

        if success {
            self.ram_base.set(ram_base);
            true
        } else {
            self.ram_base.set(None);
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

        let ram_base = self.ram_base.get().ok_or(Error {})?;
        let end_offset = offset.checked_sub(0x80000000).unwrap_or(offset);

        self.process.read(ram_base + end_offset)
    }
}

/// A future that executes a future until the emulator closes.
#[must_use = "You need to await this future."]
pub struct UntilEmulatorCloses<'a, F> {
    emulator: &'a Emulator,
    future: F,
}

impl<F: Future<Output = ()>> Future for UntilEmulatorCloses<'_, F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.emulator.is_open() {
            return Poll::Ready(());
        }
        self.emulator.update();
        // SAFETY: We are simply projecting the Pin.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().future).poll(cx) }
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
