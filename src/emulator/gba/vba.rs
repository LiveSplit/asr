use crate::{file_format::pe, signature::Signature, Address, Address32, Address64, Error, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    cached_ewram_pointer: Address,
    cached_iwram_pointer: Address,
    is_emulating: Address,
    is_64_bit: bool,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<[Address; 2]> {
        // Latest version tested: 2.1.7

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::VisualBoyAdvance(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        self.is_64_bit =
            pe::MachineType::read(game, main_module_range.0) == Some(pe::MachineType::X86_64);

        if self.is_64_bit {
            const SIG: Signature<13> = Signature::new("48 8B 05 ?? ?? ?? ?? 81 E3 FF FF 03 00");
            const SIG2: Signature<13> = Signature::new("48 8B 05 ?? ?? ?? ?? 81 E3 FF 7F 00 00");

            self.cached_ewram_pointer = {
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

            self.is_emulating = {
                const SIG_RUNNING: Signature<19> =
                    Signature::new("83 3D ?? ?? ?? ?? 00 74 ?? 80 3D ?? ?? ?? ?? 00 75 ?? 66");
                const SIG_RUNNING2: Signature<16> =
                    Signature::new("48 8B 15 ?? ?? ?? ?? 31 C0 8B 12 85 D2 74 ?? 48");

                if let Some(ptr) = SIG_RUNNING.scan_process_range(game, main_module_range) {
                    let ptr = ptr + 2;
                    ptr + 0x4 + game.read::<i32>(ptr).ok()? + 0x1
                } else {
                    let ptr = SIG_RUNNING2.scan_process_range(game, main_module_range)? + 3;
                    let ptr = ptr + 0x4 + game.read::<i32>(ptr).ok()?;
                    game.read::<Address64>(ptr).ok()?.into()
                }
            };

            let ewram = game.read::<Address64>(self.cached_ewram_pointer).ok()?;
            let iwram = game.read::<Address64>(self.cached_iwram_pointer).ok()?;

            Some([ewram.into(), iwram.into()])
        } else {
            const SIG: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 ?? FF FF 03 00");
            const SIG_OLD: Signature<12> = Signature::new("81 E6 FF FF 03 00 8B 15 ?? ?? ?? ??");

            if let Some(ptr) = SIG.scan_process_range(game, main_module_range) {
                self.cached_ewram_pointer = game.read::<Address32>(ptr + 1).ok()?.into();
                self.cached_iwram_pointer = {
                    const SIG2: Signature<11> = Signature::new("A1 ?? ?? ?? ?? 81 ?? FF 7F 00 00");
                    let ptr = SIG2.scan_process_range(game, main_module_range)?;
                    game.read::<Address32>(ptr + 1).ok()?.into()
                };

                self.is_emulating = {
                    const SIG: Signature<19> =
                        Signature::new("83 3D ?? ?? ?? ?? 00 74 ?? 80 3D ?? ?? ?? ?? 00 75 ?? 66");
                    const SIG_OLD: Signature<13> =
                        Signature::new("8B 15 ?? ?? ?? ?? 31 C0 85 D2 74 ?? 0F");

                    let ptr = SIG
                        .scan_process_range(game, main_module_range)
                        .or_else(|| SIG_OLD.scan_process_range(game, main_module_range))?;

                    game.read::<Address32>(ptr + 2).ok()?.into()
                };

                let ewram = game.read::<Address32>(self.cached_ewram_pointer).ok()?;
                let iwram = game.read::<Address32>(self.cached_iwram_pointer).ok()?;

                Some([ewram.into(), iwram.into()])
            } else if let Some(ptr) = SIG_OLD.scan_process_range(game, main_module_range) {
                // This code is for very old versions of VisualBoyAdvance (1.8.0-beta 3)
                self.cached_ewram_pointer = game.read::<Address32>(ptr + 8).ok()?.into();
                self.cached_iwram_pointer = self.cached_ewram_pointer.add_signed(0x4);

                self.is_emulating = {
                    const SIG_RUNNING: Signature<11> =
                        Signature::new("8B 0D ?? ?? ?? ?? 85 C9 74 ?? 8A");
                    let ptr = SIG_RUNNING.scan_process_range(game, main_module_range)? + 2;
                    game.read::<Address32>(ptr).ok()?.into()
                };

                let ewram = game.read::<Address32>(self.cached_ewram_pointer).ok()?;
                let iwram = game.read::<Address32>(self.cached_iwram_pointer).ok()?;

                Some([ewram.into(), iwram.into()])
            } else {
                None
            }
        }
    }

    fn read_pointer(&self, game: &Process, address: Address) -> Result<Address, Error> {
        Ok(match self.is_64_bit {
            true => game.read::<Address64>(address)?.into(),
            false => game.read::<Address32>(address)?.into(),
        })
    }

    pub fn keep_alive(&self, game: &Process, ram: &mut Option<[Address; 2]>) -> bool {
        match game.read::<bool>(self.is_emulating) {
            Ok(false) => {
                *ram = Some([Address::NULL; 2]);
            }
            Ok(true) => {
                let Ok(ewram) = self.read_pointer(game, self.cached_ewram_pointer) else {
                    return false;
                };
                let Ok(iwram) = self.read_pointer(game, self.cached_iwram_pointer) else {
                    return false;
                };
                *ram = Some([ewram, iwram]);
            }
            _ => return false,
        };
        true
    }

    pub const fn new() -> Self {
        Self {
            cached_ewram_pointer: Address::NULL,
            cached_iwram_pointer: Address::NULL,
            is_emulating: Address::NULL,
            is_64_bit: false,
        }
    }
}
