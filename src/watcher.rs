//! Support for watching values and tracking changes between them.

use core::{mem, ops};

use bytemuck::{bytes_of, Pod};

/// A watcher keeps a pair of values and allows you to track changes between
/// them.
#[derive(Copy, Clone)]
pub struct Watcher<T> {
    /// The pair of values.
    pub pair: Option<Pair<T>>,
}

impl<T> Default for Watcher<T> {
    #[inline]
    fn default() -> Self {
        Self { pair: None }
    }
}

impl<T> Watcher<T> {
    /// Creates a new empty watcher.
    #[inline]
    pub const fn new() -> Self {
        Self { pair: None }
    }
}

impl<T: Copy> Watcher<T> {
    /// Updates the watcher with a new value. Returns the pair if the value
    /// provided is not `None`.
    pub fn update(&mut self, value: Option<T>) -> Option<&Pair<T>> {
        match (&mut self.pair, value) {
            (None, Some(value)) => {
                self.pair = Some(Pair {
                    old: value,
                    current: value,
                });
            }
            (Some(pair), Some(value)) => {
                pair.old = mem::replace(&mut pair.current, value);
            }
            _ => {
                self.pair = None;
            }
        }
        self.pair.as_ref()
    }

    /// Updates the watcher with a new value that always exists. The pair is
    /// then returned.
    pub fn update_infallible(&mut self, value: T) -> &Pair<T> {
        let pair = self.pair.get_or_insert(Pair {
            old: value,
            current: value,
        });
        pair.old = mem::replace(&mut pair.current, value);
        pair
    }
}

/// A pair consisting of an old and a current value that can be used for
/// tracking changes between them.
#[derive(Copy, Clone, Default)]
pub struct Pair<T> {
    /// The old value.
    pub old: T,
    /// The current value.
    pub current: T,
}

impl<T> ops::Deref for Pair<T> {
    type Target = T;

    /// Accesses the current value.
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl<T> Pair<T> {
    /// Checks if a condition is true for the current value but false for the
    /// old value.
    pub fn check(&self, mut f: impl FnMut(&T) -> bool) -> bool {
        !f(&self.old) && f(&self.current)
    }

    /// Maps the pair to a new pair with a different type.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Pair<U> {
        Pair {
            old: f(self.old),
            current: f(self.current),
        }
    }
}

impl<T: Eq> Pair<T> {
    /// Checks if the value changed.
    pub fn changed(&self) -> bool {
        self.old != self.current
    }

    /// Checks if the value did not change.
    pub fn unchanged(&self) -> bool {
        self.old == self.current
    }

    /// Checks if the value changed to a specific value that it was not before.
    pub fn changed_to(&self, value: &T) -> bool {
        self.check(|v| v == value)
    }

    /// Checks if the value changed from a specific value that it is not now.
    pub fn changed_from(&self, value: &T) -> bool {
        self.check(|v| v != value)
    }

    /// Checks if the value changed from a specific value to another specific value.
    pub fn changed_from_to(&self, old: &T, current: &T) -> bool {
        &self.old == old && &self.current == current
    }
}

impl<T: Pod> Pair<T> {
    /// Checks if the bytes of the value changed.
    pub fn bytes_changed(&self) -> bool {
        bytes_of(&self.old) != bytes_of(&self.current)
    }

    /// Checks if the bytes of the value did not change.
    pub fn bytes_unchanged(&self) -> bool {
        bytes_of(&self.old) == bytes_of(&self.current)
    }

    /// Checks if the bytes of the value changed to a specific value that it was
    /// not before.
    pub fn bytes_changed_to(&self, value: &T) -> bool {
        self.check(|v| bytes_of(v) == bytes_of(value))
    }

    /// Checks if the bytes of the value changed from a specific value that it
    /// is not now.
    pub fn bytes_changed_from(&self, value: &T) -> bool {
        self.check(|v| bytes_of(v) != bytes_of(value))
    }

    /// Checks if the bytes of the value changed from a specific value to
    /// another specific value.
    pub fn bytes_changed_from_to(&self, old: &T, current: &T) -> bool {
        bytes_of(&self.old) == bytes_of(old) && bytes_of(&self.current) == bytes_of(current)
    }
}

impl<T: PartialOrd> Pair<T> {
    /// Checks if the value increased.
    pub fn increased(&self) -> bool {
        self.old < self.current
    }

    /// Checks if the value decreased.
    pub fn decreased(&self) -> bool {
        self.old > self.current
    }
}
