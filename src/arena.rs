use std::iter::{ExactSizeIterator, FusedIterator};
use std::mem::replace;
use std::ops;

use crate::free_pointer::FreePointer;
use crate::generation::Generation;

/// Container that can have elements inserted into it and removed from it.
///
/// Indices use the [`Index`][Index] type, created by inserting values with
/// [`Arena::insert`][Arena::insert].
#[derive(Debug, Clone)]
pub struct Arena<T> {
    storage: Vec<Entry<T>>,
    len: usize,
    first_free: Option<FreePointer>,
}

/// Index type for [`Arena`][Arena] that has a generation attached to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Index {
    slot: usize,
    generation: Generation,
}

#[derive(Debug, Clone)]
enum Entry<T> {
    Occupied {
        generation: Generation,
        value: T,
    },
    Empty {
        generation: Generation,
        next_free: Option<FreePointer>,
    },
}

impl<T> Arena<T> {
    /// Construct an empty arena.
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            len: 0,
            first_free: None,
        }
    }

    /// Construct an empty arena with space to hold exactly `capacity` elements
    /// without reallocating.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            storage: Vec::with_capacity(capacity),
            len: 0,
            first_free: None,
        }
    }

    /// Return the number of elements contained in the arena.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return the number of elements the arena can hold without allocating,
    /// including the elements currently in the arena.
    pub fn capacity(&self) -> usize {
        self.storage.capacity()
    }

    /// Returns whether the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert a new value into the arena, returning an index that can be used
    /// to later retrieve the value.
    pub fn insert(&mut self, value: T) -> Index {
        let index;

        if let Some(free_pointer) = self.first_free {
            let slot = free_pointer.slot();

            let last_generation = match &mut self.storage[slot] {
                Entry::Empty {
                    generation,
                    next_free,
                } => {
                    self.first_free = *next_free;
                    generation
                }
                Entry::Occupied { .. } => unreachable!("first_free pointed to an occupied entry"),
            };

            let generation = last_generation.next();

            self.storage[slot] = Entry::Occupied { generation, value };
            index = Index { slot, generation };
        } else {
            let slot = self.storage.len();
            let generation = Generation::first();

            self.storage.push(Entry::Occupied { generation, value });
            index = Index { slot, generation };
        }

        self.len += 1;
        index
    }

    /// Get an immutable reference to a value inside the arena by
    /// [`Index`][Index], returning `None` if the index is not contained in the
    /// arena.
    pub fn get(&self, index: Index) -> Option<&T> {
        match self.storage.get(index.slot) {
            Some(Entry::Occupied { generation, value }) if *generation == index.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    /// Get a mutable reference to a value inside the arena by [`Index`][Index],
    /// returning `None` if the index is not contained in the arena.
    pub fn get_mut(&mut self, index: Index) -> Option<&mut T> {
        match self.storage.get_mut(index.slot) {
            Some(Entry::Occupied { generation, value }) if *generation == index.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    /// Remove the value contained at the given index from the arena, returning
    /// it if it was present.
    pub fn remove(&mut self, index: Index) -> Option<T> {
        let entry = self.storage.get_mut(index.slot)?;

        match entry {
            Entry::Occupied { generation, .. } if *generation == index.generation => {
                let next_entry = Entry::Empty {
                    generation: generation.next(),
                    next_free: self.first_free,
                };

                let old_entry = replace(entry, next_entry);
                let value = match old_entry {
                    Entry::Occupied { value, .. } => value,
                    _ => unreachable!(),
                };

                self.len -= 1;
                self.first_free = Some(FreePointer::from_slot(index.slot));

                Some(value)
            }
            _ => None,
        }
    }

    /// Returns an iterator that removes each element from the arena.
    ///
    /// Iteration order is not defined.
    ///
    /// If the iterator is dropped before it is fully consumed, any uniterated
    /// items will still be contained in the arena.
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain {
            arena: self,
            index: 0,
        }
    }

    fn remove_entry_by_slot(&mut self, slot: usize) -> Option<(Index, T)> {
        let entry = self.storage.get_mut(slot)?;

        match entry {
            Entry::Occupied { generation, .. } => {
                let index = Index {
                    generation: *generation,
                    slot,
                };

                let next_entry = Entry::Empty {
                    generation: generation.next(),
                    next_free: self.first_free,
                };

                let old_entry = replace(entry, next_entry);
                let value = match old_entry {
                    Entry::Occupied { value, .. } => value,
                    _ => unreachable!(),
                };

                self.len -= 1;
                self.first_free = Some(FreePointer::from_slot(slot));

                Some((index, value))
            }
            _ => None,
        }
    }
}

impl<T> ops::Index<Index> for Arena<T> {
    type Output = T;

    fn index(&self, index: Index) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!("No entry at index {:?}", index))
    }
}

impl<T> ops::IndexMut<Index> for Arena<T> {
    fn index_mut(&mut self, index: Index) -> &mut Self::Output {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("No entry at index {:?}", index))
    }
}

/// See [`Arena::drain`][Arena::drain].
pub struct Drain<'a, T> {
    arena: &'a mut Arena<T>,
    index: usize,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = (Index, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index >= self.arena.storage.len() {
                return None;
            }

            let index = self.index;
            self.index += 1;

            if let Some((index, value)) = self.arena.remove_entry_by_slot(index) {
                return Some((index, value));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.arena.len, Some(self.arena.len))
    }
}

impl<'a, T> FusedIterator for Drain<'a, T> {}
impl<'a, T> ExactSizeIterator for Drain<'a, T> {}

#[cfg(test)]
mod test {
    use super::{Arena, Index};

    use std::mem::size_of;

    #[test]
    fn size_of_index() {
        assert_eq!(size_of::<Index>(), 16);
        assert_eq!(size_of::<Option<Index>>(), 16);
    }

    #[test]
    fn new() {
        let arena: Arena<u32> = Arena::new();
        assert_eq!(arena.storage.len(), 0);
        assert_eq!(arena.storage.capacity(), 0);
    }

    #[test]
    fn with_capacity() {
        let arena: Arena<u32> = Arena::with_capacity(8);
        assert_eq!(arena.storage.len(), 0);
        assert_eq!(arena.storage.capacity(), 8);
    }
}
