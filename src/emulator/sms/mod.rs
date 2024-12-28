//! Support for attaching to SEGA Master System / SEGA GameGear emulators.

use core::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{future::retry, Address, Error, Process};
use bytemuck::CheckedBitPattern;

mod blastem;
mod fusion;
mod mednafen;
mod retroarch;

/// A SEGA Master System / GameGear emulator that the auto splitter is attached to.
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
            state: Cell::new(state),
            ram_base: Cell::new(None),
        })
    }

    /// Asynchronously awaits attaching to a target emulator,
    /// yielding back to the runtime between each try.
    ///
    /// Supported emulators are:
    /// - Retroarch, with one of the following cores: `genesis_plus_gx_libretro.dll`,
    /// `genesis_plus_gx_wide_libretro.dll`, `picodrive_libretro.dll`, `smsplus_libretro.dll`, `gearsystem_libretro.dll`
    /// - Fusion
    /// - BlastEm
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
                State::Retroarch(x) => x.find_ram(&self.process),
                State::Fusion(x) => x.find_ram(&self.process),
                State::BlastEm(x) => x.find_ram(&self.process),
                State::Mednafen(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &state {
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::Fusion(x) => x.keep_alive(&self.process, &mut ram_base),
            State::BlastEm(x) => x.keep_alive(),
            State::Mednafen(x) => x.keep_alive(),
        };

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
    /// The offset provided is meant to be the same used on the original hardware.
    ///
    /// The SEGA Master System has 8KB of RAM, mapped from address
    /// `0xC000` to `0xDFFF`.
    ///
    /// Providing any offset outside this range will return `Err()`.
    pub fn read<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (offset > 0x1FFF && offset < 0xC000) || offset > 0xDFFF {
            return Err(Error {});
        }

        let wram = self.ram_base.get().ok_or(Error {})?;
        let end_offset = offset.checked_sub(0xC000).unwrap_or(offset);

        self.process.read(wram + end_offset)
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
pub enum State {
    Retroarch(retroarch::State),
    Fusion(fusion::State),
    BlastEm(blastem::State),
    Mednafen(mednafen::State),
}

static PROCESS_NAMES: &[(&str, State)] = &[
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
    ("Fusion.exe", State::Fusion(fusion::State::new())),
    ("blastem.exe", State::BlastEm(blastem::State)),
    ("mednafen.exe", State::Mednafen(mednafen::State)),
];
