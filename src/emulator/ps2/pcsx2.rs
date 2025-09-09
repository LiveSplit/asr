use crate::{file_format::pe, signature::Signature, Address, PointerSize, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pointer_size: PointerSize,
    addr_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Pcsx2(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        self.pointer_size = pe::MachineType::read(game, main_module_range.0)?.pointer_size()?;

        self.addr_base = if let Some(debug_symbol) =
            // Recent PCSX2 releases include a debug symbol that can be used to easily retrieve the address of the emulated RAM
            // Info: https://github.com/PCSX2/pcsx2/blob/0b6dccae5184b12e2dfc515a424d001cd38acb7c/pcsx2/Memory.cpp#L79
            pe::Symbol::iter(game, main_module_range.0)
                .find(|symbol| {
                    symbol
                        .get_name::<6>(game)
                        .is_ok_and(|name| name.matches(b"EEmem"))
                }) {
            debug_symbol.address
        } else {
            // For older versions we rely on regular sigscanning.
            // Support for 32bit versions could probably be removed as
            // they are for extremely outdated versions of PCSX2.
            match self.pointer_size {
                PointerSize::Bit64 => {
                    const SIG: Signature<12> =
                        Signature::new("48 8B ?? ?? ?? ?? ?? 25 F0 3F 00 00");
                    SIG.scan_process_range(game, main_module_range)
                        .map(|addr| addr + 3)
                        .and_then(|addr| Some(addr + 0x4 + game.read::<i32>(addr).ok()?))?
                }
                _ => {
                    const SIG: Signature<11> = Signature::new("8B ?? ?? ?? ?? ?? 25 F0 3F 00 00");
                    const SIG_ALT: Signature<12> =
                        Signature::new("8B ?? ?? ?? ?? ?? 81 ?? F0 3F 00 00");

                    let ptr = if let Some(addr) = SIG.scan_process_range(game, main_module_range) {
                        addr + 2
                    } else {
                        SIG_ALT.scan_process_range(game, main_module_range)? + 2
                    };
                    game.read_pointer(ptr, self.pointer_size).ok()?
                }
            }
        };

        // We can safely return Address::NULL as the address gets updated anyway in the keep_alive function.
        // This spares an unnecessary memory read.
        Some(Address::NULL)
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &mut Option<Address>) -> bool {
        *ram_base = Some(
            game.read_pointer(self.addr_base, self.pointer_size)
                .unwrap_or_default(),
        );
        true
    }

    pub const fn new() -> Self {
        Self {
            pointer_size: PointerSize::Bit64,
            addr_base: Address::NULL,
        }
    }
}
