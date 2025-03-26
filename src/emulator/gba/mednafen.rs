use crate::{file_format::pe, signature::Signature, Address, Address32, Address64, Error, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    cached_ewram_pointer: Address,
    cached_iwram_pointer: Address,
    is_64_bit: bool,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<[Address; 2]> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::Mednafen(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        self.is_64_bit =
            pe::MachineType::read(game, main_module_range.0) == Some(pe::MachineType::X86_64);

        if self.is_64_bit {
            self.cached_ewram_pointer = {
                const SIG: Signature<13> = Signature::new("48 8B 05 ?? ?? ?? ?? 81 E1 FF FF 03 00");
                let ptr: Address = SIG.scan_process_range(game, main_module_range)? + 3;
                let mut addr: Address = ptr + 0x4 + game.read::<i32>(ptr).ok()?;

                if game.read::<u8>(ptr + 10).ok()? == 0x48 {
                    addr = game.read::<Address64>(addr).ok()?.into();
                    if addr.is_null() {
                        return None;
                    }
                }

                addr
            };

            self.cached_iwram_pointer = {
                const SIG2: Signature<13> =
                    Signature::new("48 8B 05 ?? ?? ?? ?? 81 E1 FF 7F 00 00");
                let ptr: Address = SIG2.scan_process_range(game, main_module_range)? + 3;
                let mut addr: Address = ptr + 0x4 + game.read::<i32>(ptr).ok()?;

                if game.read::<u8>(ptr + 10).ok()? == 0x48 {
                    addr = game.read::<Address64>(addr).ok()?.into();
                    if addr.is_null() {
                        return None;
                    }
                }

                addr
            };

            let ewram = game.read::<Address64>(self.cached_ewram_pointer).ok()?;
            let iwram = game.read::<Address64>(self.cached_iwram_pointer).ok()?;

            Some([ewram.into(), iwram.into()])
        } else {
            self.cached_ewram_pointer = {
                const SIG: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 ?? FF FF 03 00");
                let ptr = SIG.scan_process_range(game, main_module_range)?;
                game.read::<Address32>(ptr + 1).ok()?.into()
            };

            self.cached_iwram_pointer = {
                const SIG2: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 ?? FF 7F 00 00");
                let ptr = SIG2.scan_process_range(game, main_module_range)?;
                game.read::<Address32>(ptr + 1).ok()?.into()
            };

            let ewram = game.read::<Address32>(self.cached_ewram_pointer).ok()?;
            let iwram = game.read::<Address32>(self.cached_iwram_pointer).ok()?;

            Some([ewram.into(), iwram.into()])
        }
    }

    fn read_pointer(&self, game: &Process, address: Address) -> Result<Address, Error> {
        Ok(match self.is_64_bit {
            true => game.read::<Address64>(address)?.into(),
            false => game.read::<Address32>(address)?.into(),
        })
    }

    pub fn keep_alive(&self, game: &Process, ram: &mut Option<[Address; 2]>) -> bool {
        let ewram = match self.read_pointer(game, self.cached_ewram_pointer) {
            Ok(x) => x,
            _ => return false,
        };

        let iwram = match self.read_pointer(game, self.cached_iwram_pointer) {
            Ok(x) => x,
            _ => return false,
        };

        *ram = Some([ewram, iwram]);
        true
    }

    pub const fn new() -> Self {
        Self {
            cached_ewram_pointer: Address::NULL,
            cached_iwram_pointer: Address::NULL,
            is_64_bit: false,
        }
    }
}
