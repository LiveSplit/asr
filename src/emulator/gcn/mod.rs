//! Support for attaching to Nintendo Gamecube emulators.

use core::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{future::retry, Address, Endian, Error, FromEndian, Process};
use bytemuck::CheckedBitPattern;

mod dolphin;
mod retroarch;

/// A Nintendo Gamecube emulator that the auto splitter is attached to.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: Cell<State>,
    /// The memory address of the emulated RAM
    mem1_base: Cell<Option<Address>>,
    /// The endianness used by the emulator process
    endian: Cell<Endian>,
}

impl Emulator {
    /// Attaches to the emulator process
    ///
    /// Returns `Option<T>` if successful, `None` otherwise.
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
            state: Cell::new(state),
            mem1_base: Cell::new(None),
            endian: Cell::new(Endian::Big), // Endianness is usually Big across all GCN emulators
        })
    }

    /// Asynchronously awaits attaching to a target emulator,
    /// yielding back to the runtime between each try.
    ///
    /// Supported emulators are:
    /// - Dolphin
    /// - Retroarch (using the `dolphin_libretro.dll` core)
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
        let mut mem1_base = self.mem1_base.get();
        let mut state = self.state.get();
        let mut endian = self.endian.get();

        if mem1_base.is_none() {
            mem1_base = match match &mut state {
                State::Dolphin(x) => x.find_ram(&self.process, &mut endian),
                State::Retroarch(x) => x.find_ram(&self.process, &mut endian),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &state {
            State::Dolphin(x) => x.keep_alive(&self.process, &mem1_base),
            State::Retroarch(x) => x.keep_alive(&self.process, &mem1_base),
        };

        self.state.set(state);
        self.endian.set(endian);

        if success {
            self.mem1_base.set(mem1_base);
            true
        } else {
            self.mem1_base.set(None);
            false
        }
    }

    /// Reads raw data from the emulated RAM ignoring all endianness settings.
    /// The same call, performed on two different emulators, might return different
    /// results due to the endianness used by the emulator.
    ///
    /// The offset provided is meant to be the same used on the original,
    /// big-endian system.
    ///
    /// You can alternatively provide the memory address as usually mapped on the original hardware.
    /// Valid addresses for the Nintendo Gamecube range from `0x80000000` to `0x817FFFFF`.
    ///
    /// Values below and up to `0x017FFFFF` are automatically assumed to be offsets from the memory's base address.
    /// Any other invalid value will make this method immediately return `Err()`.
    ///
    /// This call is meant to be used by experienced users.
    pub fn read_ignoring_endianness<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        if (0x01800000..0x80000000).contains(&offset) || offset >= 0x81800000 {
            return Err(Error {});
        }

        let mem1 = self.mem1_base.get().ok_or(Error {})?;
        let end_offset = offset.checked_sub(0x80000000).unwrap_or(offset);

        self.process.read(mem1 + end_offset)
    }

    /// Reads any value from the emulated RAM.
    ///
    /// The offset provided is meant to be the same used on the original,
    /// big-endian system. The call will automatically convert the offset and
    /// the output value to little endian.
    ///
    /// You can alternatively provide the memory address as usually mapped on the original hardware.
    /// Valid addresses for the Nintendo Gamecube range from `0x80000000` to `0x817FFFFF`.
    ///
    /// Values below and up to `0x017FFFFF` are automatically assumed to be offsets from the memory's base address.
    /// Any other invalid value will make this method immediately return `Err()`.
    pub fn read<T: CheckedBitPattern + FromEndian>(&self, offset: u32) -> Result<T, Error> {
        Ok(self
            .read_ignoring_endianness::<T>(offset)?
            .from_endian(self.endian.get()))
    }
}

/// A future that executes a future until the emulator closes.
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
    Dolphin(dolphin::State),
    Retroarch(retroarch::State),
}

static PROCESS_NAMES: [(&str, State); 2] = [
    ("Dolphin.exe", State::Dolphin(dolphin::State)),
    ("retroarch.exe", State::Retroarch(retroarch::State::new())),
];
