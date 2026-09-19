/// The version of Mono that was used for the game. These don't correlate to the
/// Mono version numbers.
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum Version {
    /// Version 1
    V1,
    /// Version 1 with cattrs
    V1Cattrs,
    /// Version 2
    V2,
    /// Version 3
    V3,
}
