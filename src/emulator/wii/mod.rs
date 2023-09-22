//! Support for attaching to Nintendo Wii emulators.

use crate::{Address, Endian, Error, FromEndian, Process};
use bytemuck::CheckedBitPattern;

mod dolphin;
mod retroarch;

/// A Nintendo Wii emulator that the auto splitter is attached to,
/// for supporting Wii and WiiWare games.
pub struct Emulator {
    /// The attached emulator process
    process: Process,
    /// An enum stating which emulator is currently attached
    state: State,
    /// The memory address of the emulated RAM
    ram_base: Option<[Address; 2]>, // [MEM1, MEM2]
    /// The endianness used by the emulator process
    endian: Endian,
}

impl Emulator {
    /// Attaches to the emulator process
    ///
    /// Returns `Option<Emulator>` if successful, `None` otherwise.
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
            state,
            ram_base: None,      // [MEM1, MEM2]
            endian: Endian::Big, // Endianness is usually Big in Wii emulators
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
                State::Dolphin(x) => x.find_ram(&self.process, &mut self.endian),
                State::Retroarch(x) => x.find_ram(&self.process, &mut self.endian),
            } {
                None => return false,
                something => something,
            };
        }

        let success = match &self.state {
            State::Dolphin(x) => x.keep_alive(&self.process, &self.ram_base),
            State::Retroarch(x) => x.keep_alive(&self.process, &self.ram_base),
        };

        if success {
            true
        } else {
            self.ram_base = None;
            false
        }
    }

    /// Reads raw data from the emulated RAM ignoring all endianness settings.
    /// The same call, performed on two different emulators, might return different
    /// results due to the endianness used by the emulator.
    ///
    /// The address provided is meant to be the mapped address used on the original, big-endian system.
    /// The call will automatically convert the address provided to its corresponding offset from
    /// `MEM1` or `MEM2` and read the value.
    ///
    /// The provided memory address has to match a mapped memory address on the original Wii:
    /// - Valid addresses for `MEM1` range from `0x80000000` to `0x817FFFFF`
    /// - Valid addresses for `MEM2` range from `0x90000000` to `0x93FFFFFF`
    ///
    /// Any other invalid value will make this method immediately return `Err()`.
    ///
    /// This call is meant to be used by experienced users.
    pub fn read_ignoring_endianness<T: CheckedBitPattern>(&self, address: u32) -> Result<T, Error> {
        if address >= 0x80000000 && address <= 0x817FFFFF {
            self.read_ignoring_endianness_from_mem_1(address)
        } else if address >= 0x90000000 && address <= 0x93FFFFFF {
            self.read_ignoring_endianness_from_mem_2(address)
        } else {
            Err(Error {})
        }
    }

    /// Reads any value from the emulated RAM.
    ///
    /// The offset provided is meant to be the mapped address used on the original, big-endian system.
    /// The call will automatically convert the address provided to its corresponding offset from
    /// `MEM1` or `MEM2` and read the value, providing conversion from Big Endian to Little Endian.
    ///
    /// The provided memory address has to match a mapped memory address on the original Wii:
    /// - Valid addresses for `MEM1` range from `0x80000000` to `0x817FFFFF`
    /// - Valid addresses for `MEM2` range from `0x90000000` to `0x93FFFFFF`
    ///
    /// Any other invalid value will make this method immediately return `Err()`.
    pub fn read<T: CheckedBitPattern + FromEndian>(&self, address: u32) -> Result<T, Error> {
        Ok(self
            .read_ignoring_endianness::<T>(address)?
            .from_endian(self.endian))
    }

    /// Follows a path of pointers from the address given and reads a value of the type specified from
    /// the process at the end of the pointer path.
    ///
    /// The end value is automatically converted to little endian if needed.
    pub fn read_pointer_path<T: CheckedBitPattern + FromEndian>(
        &self,
        base_address: u32,
        path: &[u32],
    ) -> Result<T, Error> {
        self.read(self.deref_offsets(base_address, path)?)
    }

    /// Follows a path of pointers from the address given and reads a value of the type specified from
    /// the process at the end of the pointer path.
    pub fn read_pointer_path_ignoring_endianness<T: CheckedBitPattern>(
        &self,
        base_address: u32,
        path: &[u32],
    ) -> Result<T, Error> {
        self.read_ignoring_endianness(self.deref_offsets(base_address, path)?)
    }

    fn deref_offsets(&self, base_address: u32, path: &[u32]) -> Result<u32, Error> {
        let mut address = base_address;
        let (&last, path) = path.split_last().ok_or(Error {})?;
        for &offset in path {
            address = self.read::<u32>(address + offset)?;
        }
        Ok(address + last)
    }

    /// Reads raw data from the emulated RAM ignoring all endianness settings.
    /// The same call, performed on two different emulators, might return different
    /// results due to the endianness used by the emulator.
    ///
    /// The address provided is meant to be the mapped address used on the original, big-endian system.
    /// The call will automatically convert the address provided to its corresponding offset from
    /// `MEM1` or and read the value.
    ///
    /// The provided memory address has to match a mapped memory address on the original Wii.
    /// Valid addresses for `MEM1` range from `0x80000000` to `0x817FFFFF`
    ///
    /// Any other invalid value will make this method immediately return `Err()`.
    pub fn read_ignoring_endianness_from_mem_1<T: CheckedBitPattern>(
        &self,
        address: u32,
    ) -> Result<T, Error> {
        if address < 0x80000000 || address > 0x817FFFFF {
            return Err(Error {});
        }
        let Some([mem1, _]) = self.ram_base else {
            return Err(Error {});
        };
        let end_offset = address.checked_sub(0x80000000).unwrap_or(address);
        self.process.read(mem1 + end_offset)
    }

    /// Reads raw data from the emulated RAM ignoring all endianness settings.
    /// The same call, performed on two different emulators, might return different
    /// results due to the endianness used by the emulator.
    ///
    /// The address provided is meant to be the mapped address used on the original, big-endian system.
    /// The call will automatically convert the address provided to its corresponding offset from
    /// `MEM2` or and read the value.
    ///
    /// The provided memory address has to match a mapped memory address on the original Wii.
    /// Valid addresses for `MEM2` range from `0x90000000` to `0x93FFFFFF`
    ///
    /// Any other invalid value will make this method immediately return `Err()`.
    pub fn read_ignoring_endianness_from_mem_2<T: CheckedBitPattern>(
        &self,
        address: u32,
    ) -> Result<T, Error> {
        if address < 0x90000000 || address > 0x93FFFFFF {
            return Err(Error {});
        }
        let Some([_, mem2]) = self.ram_base else {
            return Err(Error {});
        };
        let end_offset = address.checked_sub(0x90000000).unwrap_or(address);
        self.process.read(mem2 + end_offset)
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
