use crate::{file_format::pe, signature::Signature, Address, Address64, PointerSize, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr_base: Address,
    offsets: Option<[u64; 3]>,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<Address> {
        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::PcsxRedux(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        if pe::MachineType::read(game, main_module_range.0)?.pointer_size()? != PointerSize::Bit64 {
            return None;
        }

        const SIG_BASE: Signature<14> = Signature::new("48 8B 0D ?? ?? ?? ?? ?? ?? ?? FF FF FF 00");
        const SIG_OFFSET: Signature<10> = Signature::new("4C 8B 99 ?? ?? ?? ?? 4D 8B 7B");

        self.addr_base = SIG_BASE
            .scan_process_range(game, main_module_range)
            .map(|val| val + 3)
            .and_then(|addr| Some(addr + 0x4 + game.read::<i32>(addr).ok()?))?;

        let p_addr = SIG_OFFSET.scan_process_range(game, main_module_range)?;
        self.offsets = Some([
            0,
            game.read::<i32>(p_addr + 3).ok()? as u64,
            game.read::<u8>(p_addr + 10).ok()?.into(),
        ]);

        game.read_pointer_path::<Address64>(
            self.addr_base,
            PointerSize::Bit64,
            &self.offsets.unwrap(),
        )
        .map(|val| val.into())
        .ok()
    }

    pub fn keep_alive(&self, game: &Process, ram_base: &mut Option<Address>) -> bool {
        match self.offsets {
            None => false,
            Some(offsets) => {
                match game.read_pointer_path::<Address64>(
                    self.addr_base,
                    PointerSize::Bit64,
                    &offsets,
                ) {
                    Ok(result) => {
                        *ram_base = Some(result.into());
                        true
                    }
                    Err(_) => false,
                }
            }
        }
    }

    pub const fn new() -> Self {
        Self {
            addr_base: Address::NULL,
            offsets: None,
        }
    }
}
