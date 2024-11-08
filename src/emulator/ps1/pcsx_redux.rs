use crate::{
    file_format::pe,
    signature::{Signature, SignatureScanner},
    Address, Address32, Address64, MemoryRangeFlags, Process,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    is_64_bit: bool,
    addr_base: Address,
    addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::PcsxRedux(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        self.is_64_bit =
            pe::MachineType::read(game, main_module_range.0) == Some(pe::MachineType::X86_64);

        if self.is_64_bit {
            const SIG_BASE: Signature<25> = Signature::new(
                "48 B9 ?? ?? ?? ?? ?? ?? ?? ?? E8 ?? ?? ?? ?? C7 85 ?? ?? ?? ?? 00 00 00 00",
            );
            const SIG_OFFSET: Signature<9> = Signature::new("89 D1 C1 E9 10 48 8B ?? ??");

            self.addr_base = SIG_BASE.scan(game, main_module_range)? + 2;
            self.addr = game.read::<Address64>(self.addr_base).ok()?.into();

            let offset = SIG_OFFSET.scan(game, main_module_range)? + 8;
            let offset = game.read::<u8>(offset).ok()? as u64;

            let addr = game.read::<Address64>(self.addr + offset).ok()?;

            Some(game.read::<Address64>(addr).ok()?.into())
        } else {
            const SIG: Signature<18> =
                Signature::new("8B 3D 20 ?? ?? ?? 0F B7 D3 8B 04 95 ?? ?? ?? ?? 21 05");

            self.addr_base = game
                .memory_ranges()
                .filter(|m| {
                    m.flags()
                        .unwrap_or_default()
                        .contains(MemoryRangeFlags::WRITE)
                })
                .find_map(|m| SIG.scan(game, m.range().ok()?))?
                + 2;

            self.addr = game.read::<Address32>(self.addr_base).ok()?.into();
            Some(self.addr)
        }
    }

    pub fn keep_alive(&self, game: &Process) -> bool {
        if self.is_64_bit {
            game.read::<Address64>(self.addr_base)
                .is_ok_and(|addr| self.addr == addr.into())
        } else {
            game.read::<Address32>(self.addr_base)
                .is_ok_and(|addr| self.addr == addr.into())
        }
    }

    pub const fn new() -> Self {
        Self {
            is_64_bit: true,
            addr_base: Address::NULL,
            addr: Address::NULL,
        }
    }
}
