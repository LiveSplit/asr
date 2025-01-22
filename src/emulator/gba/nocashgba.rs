use crate::{signature::Signature, Address, Address32, Process};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    base_addr: Address,
}

impl State {
    pub fn find_ram(&mut self, game: &Process) -> Option<[Address; 2]> {
        // Tested and working on NO$GBA 3.2 and 3.05
        const SIG: Signature<7> = Signature::new("FF 35 ?? ?? ?? ?? 55");

        let main_module_range = super::PROCESS_NAMES
            .iter()
            .filter(|(_, state)| matches!(state, super::State::NoCashGba(_)))
            .find_map(|(name, _)| game.get_module_range(name).ok())?;

        self.base_addr = game
            .read::<Address32>(SIG.scan_once(game, main_module_range)? + 0x2)
            .ok()?
            .into();

        let addr: Address = game.read::<Address32>(self.base_addr).ok()?.into();

        let ewram_pointer = addr.add(0x938C).add(0x8);
        let iwram_pointer = addr.add(0x95D4);

        Some([
            game.read::<Address32>(ewram_pointer).ok()?.into(),
            game.read::<Address32>(iwram_pointer).ok()?.into(),
        ])
    }

    pub fn keep_alive(&self, game: &Process, ram: &mut Option<[Address; 2]>) -> bool {
        let Ok(addr) = game.read::<Address32>(self.base_addr) else {
            return false;
        };
        let ewram_pointer = addr.add(0x938C).add(0x8);
        let iwram_pointer = addr.add(0x95D4);

        let Ok(ewram) = game.read::<Address32>(ewram_pointer) else {
            return false;
        };
        let Ok(iwram) = game.read::<Address32>(iwram_pointer) else {
            return false;
        };

        *ram = Some([ewram.into(), iwram.into()]);
        true
    }

    pub const fn new() -> Self {
        Self {
            base_addr: Address::NULL,
        }
    }
}
