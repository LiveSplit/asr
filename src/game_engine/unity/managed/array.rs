use arrayvec::ArrayVec;
use core::ops;

/// A managed array read out of a game: the elements of a `T[]`, as many as
/// the array counts. `N` bounds how many elements the array holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManagedArray<T, const N: usize>(ArrayVec<T, N>);

impl<T: Copy, const N: usize> ManagedArray<T, N> {
    /// Builds an array from its elements. More than `N` elements is `None`.
    pub(crate) fn from_elements(elements: &[T]) -> Option<Self> {
        let mut array = ArrayVec::new();
        array.try_extend_from_slice(elements).ok()?;
        Some(Self(array))
    }
}

impl<T, const N: usize> ManagedArray<T, N> {
    /// Returns every element of the array.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Returns how many elements the array holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the array holds no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T, const N: usize> ops::Deref for ManagedArray<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::ManagedArray;

    #[test]
    fn compares_by_every_element() {
        let three = ManagedArray::<i32, 4>::from_elements(&[7, 8, 9]).unwrap();
        let two = ManagedArray::<i32, 4>::from_elements(&[7, 8]).unwrap();
        assert_eq!(three.len(), 3);
        assert!(!three.is_empty());
        assert_ne!(three, two);
        assert_eq!(three[2], 9);
        assert_eq!(three.iter().sum::<i32>(), 24);
    }

    #[test]
    fn refuses_more_elements_than_it_holds() {
        assert!(ManagedArray::<u8, 2>::from_elements(&[1, 2, 3]).is_none());
        assert!(ManagedArray::<u8, 2>::from_elements(&[])
            .unwrap()
            .is_empty());
    }
}
