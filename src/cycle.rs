//! Catches pointer chains that lead back to where they have been.

use crate::Address;

/// Catches a chain of pointers that leads back to an address it already
/// passed, which wrong offsets can cause. It keeps 1 address and swaps in the
/// current one each time the step count reaches the next power of 2 (Brent's
/// algorithm). It notices a cycle within about 2 to 3 times the number of
/// different addresses on the chain.
pub(crate) struct Cycle {
    saved: Address,
    power: u32,
    steps: u32,
}

impl Cycle {
    /// Starts at the first address of the chain.
    pub(crate) const fn new(start: Address) -> Self {
        Self {
            saved: start,
            power: 1,
            steps: 0,
        }
    }

    /// Takes the next address of the chain and returns whether the chain has
    /// been there before.
    pub(crate) fn revisits(&mut self, at: Address) -> bool {
        if at == self.saved {
            return true;
        }
        self.steps += 1;
        if self.steps == self.power {
            (self.saved, self.power, self.steps) = (at, self.power * 2, 0);
        }
        false
    }
}
