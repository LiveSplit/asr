//! Support for attaching to Nintendo Gameboy Advance emulators.

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

mod emuhawk;
mod mednafen;
mod mgba;
mod nocashgba;
mod retroarch;
mod vba;

/// A Nintendo Gameboy Advance emulator that the auto splitter is attached to.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: Cell<State>,
    /// The memory address of the emulated RAM
    ram_base: Cell<Option<[Address; 2]>>, // [ewram, iwram]
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
    /// - BizHawk
    /// - Retroarch, with one of the following cores: `vbam_libretro.dll`, `vba_next_libretro.dll`,
    /// `mednafen_gba_libretro.dll`, `mgba_libretro.dll`, `gpsp_libretro.dll`
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
    /// - VisualBoyAdvance
    /// - VisualBoyAdvance-M
    /// - mGBA
    /// - NO$GBA
    /// - BizHawk
    /// - Retroarch, with one of the following cores: `vbam_libretro.dll`, `vba_next_libretro.dll`,
    /// `mednafen_gba_libretro.dll`, `mgba_libretro.dll`, `gpsp_libretro.dll`
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
                State::VisualBoyAdvance(x) => x.find_ram(&self.process),
                State::Mgba(x) => x.find_ram(&self.process),
                State::NoCashGba(x) => x.find_ram(&self.process),
                State::Retroarch(x) => x.find_ram(&self.process),
                State::EmuHawk(x) => x.find_ram(&self.process),
                State::Mednafen(x) => x.find_ram(&self.process),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &state {
            State::VisualBoyAdvance(x) => x.keep_alive(&self.process, &mut ram_base),
            State::Mgba(x) => x.keep_alive(&self.process, &ram_base),
            State::NoCashGba(x) => x.keep_alive(&self.process, &mut ram_base),
            State::Retroarch(x) => x.keep_alive(&self.process),
            State::EmuHawk(x) => x.keep_alive(&self.process, &ram_base),
            State::Mednafen(x) => x.keep_alive(&self.process, &mut ram_base),
        };

        self.state.set(state);

        self.ram_base.set(if success { ram_base } else { None });
        success
    }

    /// Converts a GBA memory address to a real memory address in the emulator process' virtual memory space
    ///
    /// Valid addresses range:
    /// - from `0x02000000` to `0x0203FFFF` for EWRAM
    /// - from `0x03000000` to `0x03007FFF` for IWRAM
    pub fn get_address(&self, offset: u32) -> Result<Address, Error> {
        match offset {
            (0x02000000..=0x0203FFFF) => {
                let r_offset = offset.sub(0x02000000);
                let [ewram, _] = self.ram_base.get().ok_or(Error {})?;
                Ok(ewram + r_offset)
            }
            (0x03000000..=0x03007FFF) => {
                let r_offset = offset.sub(0x03000000);
                let [_, iwram] = self.ram_base.get().ok_or(Error {})?;
                Ok(iwram + r_offset)
            }
            _ => Err(Error {}),
        }
    }

    /// Checks if a memory reading operation would exceed the memory bounds of the emulated system.
    ///
    /// Returns `true` if the read operation can be performed safely, `false` otherwise.
    const fn check_bounds<T>(&self, offset: u32) -> bool {
        match offset {
            (0x02000000..=0x0203FFFF) => offset + size_of::<T>() as u32 <= 0x02040000,
            (0x03000000..=0x03007FFF) => offset + size_of::<T>() as u32 <= 0x03008000,
            _ => false,
        }
    }

    /// Reads any value from the emulated RAM.
    ///
    /// The offset provided is meant to be the same memory address as usually mapped on the original hardware.
    /// Valid addresses range:
    /// - from `0x02000000` to `0x0203FFFF` for EWRAM
    /// - from `0x03000000` to `0x03007FFF` for IWRAM
    ///
    /// Values outside these ranges are invalid, and will make this method immediately return `Err()`.
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
pub enum State {
    VisualBoyAdvance(vba::State),
    Mgba(mgba::State),
    NoCashGba(nocashgba::State),
    Retroarch(retroarch::State),
    EmuHawk(emuhawk::State),
    Mednafen(mednafen::State),
}

static PROCESS_NAMES: [(&str, State); 7] = [
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
    ("EmuHawk.exe", State::EmuHawk(emuhawk::State::new())),
    ("mednafen.exe", State::Mednafen(mednafen::State::new())),
];
