//! Support for attaching to SEGA Genesis emulators.

use core::{
    cell::Cell,
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{future::retry, Address, Endian, Error, FromEndian, Process};
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
    state: Cell<State>,
    /// The memory address of the emulated RAM
    wram_base: Cell<Option<Address>>,
    /// The endianness used by the emulator process
    endian: Cell<Endian>,
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
            state: Cell::new(state),
            wram_base: Cell::new(None),
            endian: Cell::new(Endian::Little), // Endianness is supposed to be Little, until stated otherwise in the code
        })
    }

    /// Asynchronously awaits attaching to a target emulator,
    /// yielding back to the runtime between each try.
    ///
    /// Supported emulators are:
    /// - Retroarch
    /// - SEGA Classics / SEGA Game Room
    /// - Fusion
    /// - Gens
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
        let mut wram_base = self.wram_base.get();
        let mut endian = self.endian.get();
        let mut state = self.state.get();

        if wram_base.is_none() {
            wram_base = match match &mut state {
                State::Retroarch(x) => x.find_wram(&self.process, &mut endian),
                State::SegaClassics(x) => x.find_wram(&self.process, &mut endian),
                State::Fusion(x) => x.find_wram(&self.process, &mut endian),
                State::Gens(x) => x.find_wram(&self.process, &mut endian),
                State::BlastEm(x) => x.find_wram(&self.process, &mut endian),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &state {
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::SegaClassics(x) => x.keep_alive(&self.process, &mut wram_base),
            State::Fusion(x) => x.keep_alive(&self.process, &mut wram_base),
            State::Gens(x) => x.keep_alive(),
            State::BlastEm(x) => x.keep_alive(),
        };

        self.endian.set(endian);
        self.state.set(state);

        if success {
            self.wram_base.set(wram_base);
            true
        } else {
            self.wram_base.set(None);
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

        let wram = self.wram_base.get().ok_or(Error {})?;
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
        if (offset > 0xFFFF && offset < 0xFF0000) || offset > 0xFFFFFF {
            return Err(Error {});
        }

        let wram = self.wram_base.get().ok_or(Error {})?;

        let mut end_offset = offset.checked_sub(0xFF0000).unwrap_or(offset);
        let endian = self.endian.get();

        let toggle = endian == Endian::Little && mem::size_of::<T>() == 1;
        end_offset ^= toggle as u32;

        let value = self.process.read::<T>(wram + end_offset)?;
        Ok(value.from_endian(endian))
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
        if !self.emulator.process.is_open() {
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
