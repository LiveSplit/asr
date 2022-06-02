use core::{mem, ops};

#[derive(Copy, Clone, Default)]
pub struct Watcher<T> {
    pair: Option<Pair<T>>,
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
}
