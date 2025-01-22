use crate::{signature::Signature, Address, Address32, Endian, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    addr: Address,
}

impl State {
    pub fn find_wram(&mut self, game: &Process, endian: &mut Endian) -> Option<Address> {
        const SIG_GAMEROOM: Signature<16> =
            Signature::new("C7 05 ???????? ???????? A3 ???????? A3");
        const SIG_SEGACLASSICS: Signature<8> = Signature::new("89 2D ???????? 89 0D");
        const GENESISWRAPPERDLL: &str = "GenesisEmuWrapper.dll";

        let mut ptr = if let Ok(module) = game.get_module_range(GENESISWRAPPERDLL) {
            SIG_GAMEROOM.scan_once(game, module)? + 2
        } else {
            let main_module = super::PROCESS_NAMES
                .iter()
                .filter(|(_, state)| matches!(state, super::State::SegaClassics(_)))
                .find_map(|(name, _)| game.get_module_range(name).ok())?;

            SIG_SEGACLASSICS.scan_once(game, main_module)? + 8
        };

        ptr = game.read::<Address32>(ptr).ok()?.into();

        self.addr = ptr;
        *endian = Endian::Little;

        ptr = game.read::<Address32>(self.addr).ok()?.into();

        Some(ptr)
    }

    pub fn keep_alive(&self, game: &Process, wram_base: &mut Option<Address>) -> bool {
        if let Ok(addr) = game.read::<Address32>(self.addr) {
            *wram_base = Some(addr.into());
            true
        } else {
            false
        }
    }

    pub const fn new() -> Self {
        Self {
            addr: Address::NULL,
        }
    }
}
