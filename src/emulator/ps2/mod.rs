//! Support for attaching to Playstation 2 emulators.

use core::{
    cell::Cell,
    future::Future,
    mem::size_of,
    ops::Sub,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{future::retry, Address, Error, Process};
use bytemuck::CheckedBitPattern;

mod pcsx2;
mod retroarch;

/// A Playstation 2 emulator that the auto splitter is attached to.
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
    /// - PCSX2
    /// - Retroarch (64-bit version, using the `pcsx2_libretro.dll` core)
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
    /// - PCSX2
    /// - Retroarch (64-bit version, using the `pcsx2_libretro.dll` core)
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
                State::Pcsx2(x) => x.find_ram(&self.process),
                State::Retroarch(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &state {
            State::Pcsx2(x) => x.keep_alive(&self.process, &mut ram_base),
            State::Retroarch(x) => x.keep_alive(&self.process),
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

    /// Converts a PS2 memory address to a real memory address in the emulator process' virtual memory space
    ///
    /// Valid addresses for the PS2 range from `0x00100000` to `0x01FFFFFF`.
    pub fn get_address(&self, offset: u32) -> Result<Address, Error> {
        match offset {
            (0x00100000..=0x01FFFFFF) => {
                Ok(self.ram_base.get().ok_or(Error {})? + offset.sub(0x00100000))
            }
            _ => Err(Error {}),
        }
    }

    /// Checks if a memory reading operation would exceed the memory bounds of the emulated system.
    ///
    /// Returns `true` if the read operation can be performed safely, `false` otherwise.
    const fn check_bounds<T>(&self, offset: u32) -> bool {
        match offset {
            (0x00100000..=0x01FFFFFF) => offset + size_of::<T>() as u32 <= 0x02000000,
            _ => false,
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
    pub fn read<T: CheckedBitPattern>(&self, offset: u32) -> Result<T, Error> {
        match self.check_bounds::<T>(offset) {
            true => self.process.read(self.get_address(offset)?),
            false => Err(Error {}),
        }
    }

    /// Follows a path of pointers from the address given and reads a value of the type specified from
    /// the process at the end of the pointer path.
    pub fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        base_address: u32,
        path: &[u32],
    ) -> Result<T, Error> {
        self.read(self.deref_offsets(base_address, path)?)
    }

    /// Follows a path of pointers from the address given and returns the address at the end
    /// of the pointer path
    fn deref_offsets(&self, base_address: u32, path: &[u32]) -> Result<u32, Error> {
        let mut address = base_address;
        let (&last, path) = path.split_last().ok_or(Error {})?;
        for &offset in path {
            address = self.read::<u32>(address + offset)?;
        }
        Ok(address + last)
    }
}

/// A future that executes a future until the emulator closes.
#[must_use = "You need to await this future."]
pub struct UntilEmulatorCloses<'a, F> {
    emulator: &'a Emulator,
    future: F,
}

impl<T, F: Future<Output = T>> Future for UntilEmulatorCloses<'_, F> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.emulator.is_open() {
            return Poll::Ready(None);
        }
        self.emulator.update();
        // SAFETY: We are simply projecting the Pin.
        unsafe {
            Pin::new_unchecked(&mut self.get_unchecked_mut().future)
                .poll(cx)
                .map(Some)
        }
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
