//! Support for attaching to SEGA Genesis emulators.

use core::{
    cell::Cell,
    future::Future,
    mem::{size_of, MaybeUninit},
    pin::Pin,
    slice,
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

    /// Converts a SEGA Genesis memory address to a real memory address in the emulator process' virtual memory space
    ///
    /// The offset provided must not be higher than `0xFFFF`
    pub fn get_address(&self, offset: u32) -> Result<Address, Error> {
        match offset {
            (0..=0xFFFF) => Ok(self.wram_base.get().ok_or(Error {})? + offset),
            _ => Err(Error {}),
        }
    }

    /// Checks if a memory reading operation would exceed the memory bounds of the emulated system.
    ///
    /// Returns `true` if the read operation can be performed safely, `false` otherwise.
    fn check_bounds<T>(&self, offset: u32) -> bool {
        match offset {
            (0..=0xFFFF) => offset + size_of::<T>() as u32 <= 0x10000,
            _ => false,
        }
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
        if !self.check_bounds::<T>(offset) {
            return Err(Error {});
        }

        let aligned_offset = offset & !1;
        let Ok(address) = self.get_address(aligned_offset) else {
            return Err(Error {});
        };
        let endian = self.endian.get();

        #[derive(Copy, Clone)]
        #[repr(packed)]
        struct MaybePadded<T> {
            _before: MaybeUninit<u8>,
            value: MaybeUninit<T>,
            _after: MaybeUninit<u8>,
        }

        let misalignment = offset as usize & 1;
        let mut padded_value = MaybeUninit::<MaybePadded<T>>::uninit();

        // We always want to read a multiple of 2 bytes, so at the end we need
        // to find the next multiple of 2 bytes for T. However because we maybe
        // are misaligned, we need to also take that misalignment in the
        // opposite direction into account before finding the next multiple of
        // two as otherwise we may not read all of T. This would otherwise go
        // wrong when e.g. reading a u16 at a misaligned offset. We would start
        // at the padding byte before the u16, but if we only read 2 bytes, we
        // then would miss the half of the u16. So adding the misalignment of 1
        // on top and then rounding up to the next multiple of 2 bytes leaves us
        // with 4 bytes to read, which we can then nicely swap.
        let buf = unsafe {
            slice::from_raw_parts_mut(
                padded_value.as_mut_ptr().byte_add(misalignment ^ 1) as *mut MaybeUninit<u8>,
                (size_of::<T>() + misalignment).next_multiple_of(2),
            )
        };

        let buf = self.process.read_into_uninit_buf(address, buf)?;

        if endian.eq(&Endian::Little) {
            buf.chunks_exact_mut(2).for_each(|chunk| chunk.swap(0, 1));
        }

        unsafe {
            let value = padded_value.assume_init_ref().value;
            if !T::is_valid_bit_pattern(&*value.as_ptr().cast::<T::Bits>()) {
                return Err(Error {});
            }

            Ok(value.assume_init().from_be())
        }
    }

    /// Follows a path of pointers from the address given and reads a value of the type specified from
    /// the process at the end of the pointer path.
    pub fn read_pointer_path<T: CheckedBitPattern + FromEndian>(
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
