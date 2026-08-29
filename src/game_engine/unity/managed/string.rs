use arrayvec::ArrayVec;
use core::ops;

/// A managed string read out of a game: the UTF-16 units of a
/// `System.String`, as many as the string counts. A nul unit is a character
/// like any other, and an unpaired surrogate stays as it was read. `N` bounds
/// how many units the string holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManagedString<const N: usize>(ArrayVec<u16, N>);

impl<const N: usize> ManagedString<N> {
    /// Builds a string from its units. More than `N` units is `None`.
    pub(crate) fn from_units(units: &[u16]) -> Option<Self> {
        let mut string = ArrayVec::new();
        string.try_extend_from_slice(units).ok()?;
        Some(Self(string))
    }

    /// Returns every unit of the string.
    pub fn as_slice(&self) -> &[u16] {
        self.0.as_slice()
    }

    /// Returns how many units the string holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the string holds no units.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Checks whether the string is exactly the given units.
    pub fn matches(&self, text: impl AsRef<[u16]>) -> bool {
        self.as_slice() == text.as_ref()
    }

    /// Checks whether the string is exactly the given text. This re-encodes
    /// the text to UTF-16 as it compares, which is slower than
    /// [`matches`](Self::matches).
    pub fn matches_str(&self, text: &str) -> bool {
        self.0.iter().copied().eq(text.encode_utf16())
    }
}

impl<const N: usize> ops::Deref for ManagedString<N> {
    type Target = [u16];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::ManagedString;

    #[test]
    fn compares_by_every_unit() {
        let with_nul = ManagedString::<4>::from_units(&[b'a' as u16, 0, b'b' as u16]).unwrap();
        let a = ManagedString::<4>::from_units(&[b'a' as u16]).unwrap();
        assert_eq!(with_nul.len(), 3);
        assert!(!with_nul.is_empty());
        assert_ne!(with_nul, a);
        assert!(!with_nul.matches_str("a"));
        assert!(with_nul.matches_str("a\0b"));
        assert!(with_nul.matches([b'a' as u16, 0, b'b' as u16]));
        assert!(a.matches_str("a"));
        assert_eq!(with_nul[2], b'b' as u16);
    }

    #[test]
    fn refuses_more_units_than_it_holds() {
        assert!(ManagedString::<2>::from_units(&[1, 2, 3]).is_none());
        assert!(ManagedString::<2>::from_units(&[]).unwrap().is_empty());
    }
}
