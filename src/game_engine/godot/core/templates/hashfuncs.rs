//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/hashfuncs.h>

use core::num::NonZeroU32;

use bytemuck::CheckedBitPattern;

use crate::Process;

/// A trait for looking up a key in a hash table. The type of the key to look up
/// does not need to match the type in the target process. However, it needs to
/// hash and compare equally in the same way.
pub trait Hash<Q>: CheckedBitPattern {
    /// Hashes the lookup key.
    fn hash_of_lookup_key(lookup_key: &Q) -> u32;
    /// Compares the lookup key with the key in the target process. Errors are
    /// meant to be ignored and instead treated as comparing unequal.
    fn eq(&self, lookup_key: &Q, process: &Process) -> bool;
}

const HASH_TABLE_SIZE_MAX: usize = 29;

#[track_caller]
pub(super) const fn n32(x: u32) -> NonZeroU32 {
    match NonZeroU32::new(x) {
        Some(x) => x,
        None => panic!(),
    }
}

pub(super) const HASH_TABLE_SIZE_PRIMES: [NonZeroU32; HASH_TABLE_SIZE_MAX] = [
    n32(5),
    n32(13),
    n32(23),
    n32(47),
    n32(97),
    n32(193),
    n32(389),
    n32(769),
    n32(1543),
    n32(3079),
    n32(6151),
    n32(12289),
    n32(24593),
    n32(49157),
    n32(98317),
    n32(196613),
    n32(393241),
    n32(786433),
    n32(1572869),
    n32(3145739),
    n32(6291469),
    n32(12582917),
    n32(25165843),
    n32(50331653),
    n32(100663319),
    n32(201326611),
    n32(402653189),
    n32(805306457),
    n32(1610612741),
];

pub(super) const HASH_TABLE_SIZE_PRIMES_INV: [u64; HASH_TABLE_SIZE_MAX] = [
    3689348814741910324,
    1418980313362273202,
    802032351030850071,
    392483916461905354,
    190172619316593316,
    95578984837873325,
    47420935922132524,
    23987963684927896,
    11955116055547344,
    5991147799191151,
    2998982941588287,
    1501077717772769,
    750081082979285,
    375261795343686,
    187625172388393,
    93822606204624,
    46909513691883,
    23456218233098,
    11728086747027,
    5864041509391,
    2932024948977,
    1466014921160,
    733007198436,
    366503839517,
    183251896093,
    91625960335,
    45812983922,
    22906489714,
    11453246088,
];

pub(super) fn fastmod(n: u32, _c: u64, d: NonZeroU32) -> u32 {
    #[cfg(not(target_family = "wasm"))]
    {
        let lowbits = _c.wrapping_mul(n as u64);
        // TODO: `widening_mul`
        ((lowbits as u128 * d.get() as u128) >> 64) as u32
    }
    #[cfg(target_family = "wasm")]
    {
        n % d.get()
    }
}
