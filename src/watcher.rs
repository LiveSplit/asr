use core::{mem, ops};

use bytemuck::{bytes_of, Pod};

#[derive(Copy, Clone)]
pub struct Watcher<T> {
    pub pair: Option<Pair<T>>,
}

// We need to impl Default manually here because the derive implmentation adds the unnecessary `T: Default` bound
impl<T> Default for Watcher<T> {
    fn default() -> Self {
        Self { pair: None }
    }
}

impl<T> Watcher<T> {
    pub const fn new() -> Self {
        Self { pair: None }
    }
}

impl<T: Copy> Watcher<T> {
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

    pub fn update_infallible(&mut self, value: T) -> &Pair<T> {
        let pair = self.pair.get_or_insert_with(|| Pair {
            old: value,
            current: value,
        });
        pair.old = mem::replace(&mut pair.current, value);
        pair
    }
}

#[derive(Copy, Clone, Default)]
pub struct Pair<T> {
    pub old: T,
    pub current: T,
}

impl<T> ops::Deref for Pair<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl<T> Pair<T> {
    pub fn check(&self, mut f: impl FnMut(&T) -> bool) -> bool {
        !f(&self.old) && f(&self.current)
    }

    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Pair<U> {
        Pair {
            old: f(self.old),
            current: f(self.current),
        }
    }
}

impl<T: Eq> Pair<T> {
    pub fn changed(&self) -> bool {
        self.old != self.current
    }

    pub fn unchanged(&self) -> bool {
        self.old == self.current
    }

    pub fn changed_to(&self, value: &T) -> bool {
        self.check(|v| v == value)
    }

    pub fn changed_from(&self, value: &T) -> bool {
        self.check(|v| v != value)
    }

    pub fn changed_from_to(&self, old: &T, current: &T) -> bool {
        &self.old == old && &self.current == current
    }
}

impl<T: Pod> Pair<T> {
    pub fn bytes_changed(&self) -> bool {
        bytes_of(&self.old) != bytes_of(&self.current)
    }

    pub fn bytes_unchanged(&self) -> bool {
        bytes_of(&self.old) == bytes_of(&self.current)
    }

    pub fn bytes_changed_to(&self, value: &T) -> bool {
        self.check(|v| bytes_of(v) == bytes_of(value))
    }

    pub fn bytes_changed_from(&self, value: &T) -> bool {
        self.check(|v| bytes_of(v) != bytes_of(value))
    }

    pub fn bytes_changed_from_to(&self, old: &T, current: &T) -> bool {
        bytes_of(&self.old) == bytes_of(old) && bytes_of(&self.current) == bytes_of(current)
    }
}

impl<T: PartialOrd> Pair<T> {
    pub fn increased(&self) -> bool {
        self.old < self.current
    }

    pub fn decreased(&self) -> bool {
        self.old > self.current
    }
}
