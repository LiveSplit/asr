use crate::{
    file_format::pe,
    signature::{Signature, SignatureScanner},
    Address, Address32, Address64, Error, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    is_64_bit: bool,
    addr_base: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Pcsx2(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        self.is_64_bit =
            pe::MachineType::read(game, main_module_range.0) == Some(pe::MachineType::X86_64);

        self.addr_base = if self.is_64_bit {
            const SIG: Signature<12> = Signature::new("48 8B ?? ?? ?? ?? ?? 25 F0 3F 00 00");
            let ptr = SIG.scan(game, main_module_range.0, main_module_range.1)? + 3;
            ptr + 0x4 + game.read::<i32>(ptr).ok()?
        } else {
            const SIG: Signature<11> = Signature::new("8B ?? ?? ?? ?? ?? 25 F0 3F 00 00");
            const SIG_ALT: Signature<12> = Signature::new("8B ?? ?? ?? ?? ?? 81 ?? F0 3F 00 00");
            let ptr = if let Some(addr) = SIG.scan(game, main_module_range.0, main_module_range.1) {
                addr + 2
            } else {
                SIG_ALT.scan(game, main_module_range.0, main_module_range.1)? + 2
            };
            self.read_pointer(game, ptr).ok()?
        };

        self.read_pointer(game, self.addr_base).ok()
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &mut Option<Address>) -> bool {
        *ram_base = Some(match self.read_pointer(game, self.addr_base) {
            Ok(x) => x,
            Err(_) => return false,
        });
        true
    }

    fn read_pointer(&self, game: &Process, address: Address) -> Result<Address, Error> {
        Ok(match self.is_64_bit {
            true => game.read::<Address64>(address)?.into(),
            false => game.read::<Address32>(address)?.into(),
        })
    }

    pub const fn new() -> Self {
        Self {
            is_64_bit: true,
            addr_base: Address::NULL,
        }
    }
}
