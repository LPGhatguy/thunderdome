//! Entry API for Thunderdome.

use core::fmt;

use crate::arena::{Arena, Index};

/// A view into a single entry in an [`Arena`], which may either be vacant or
/// occupied.
///
/// This `enum` is constructed from the [`entry`] method on [`Arena`].
///
/// [`entry`]: Arena::entry
pub enum Entry<'a, T> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, T>),

    /// An occupied entry.
    Occupied(OccupiedEntry<'a, T>),
}

impl<T: fmt::Debug> fmt::Debug for Entry<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Vacant(v) => f.debug_tuple("Entry").field(v).finish(),
            Entry::Occupied(o) => f.debug_tuple("Entry").field(o).finish(),
        }
    }
}

/// A view into a vacant entry in an [`Arena`].
/// It is part of the [`Entry`] enum.
#[derive(Debug)]
pub struct VacantEntry<'a, T> {
    arena: &'a mut Arena<T>,
    index: Index,
}

/// A view into an occupied entry in an [`Arena`].
/// It is part of the [`Entry`] enum.
#[derive(Debug)]
pub struct OccupiedEntry<'a, T> {
    arena: &'a mut Arena<T>,
    index: Index,
}

impl<'a, T> Entry<'a, T> {
    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    ///
    /// If this entry is vacant, this calls [`Arena::insert_at`] internally, so
    /// it is capable of "resurrecting" an old index.
    pub fn or_insert(self, default: T) -> &'a mut T {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns a mutable reference to the value in the
    /// entry.
    ///
    /// If this entry is vacant, this calls [`Arena::insert_at`] internally, so
    /// it is capable of "resurrecting" an old index.
    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> &'a mut T {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting, if empty, the result of
    /// the default function.
    ///
    /// This method allows for generating key-derived values for insertion by
    /// providing the default function the key that was moved during the
    /// `.entry(key)` method call.
    ///
    /// If this entry is vacant, this calls [`Arena::insert_at`] internally, so
    /// it is capable of "resurrecting" an old index.
    pub fn or_insert_with_key<F: FnOnce(Index) -> T>(self, default: F) -> &'a mut T {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Returns this entry's key.
    pub fn key(&self) -> Index {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the arena.
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }
}

impl<'a, T: Default> Entry<'a, T> {
    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// If this entry is vacant, this calls [`Arena::insert_at`] internally, so
    /// it is capable of "resurrecting" an old index.
    pub fn or_default(self) -> &'a mut T {
        self.or_insert_with(Default::default)
    }
}

impl<'a, T> VacantEntry<'a, T> {
    /// Gets the key that would be used when inserting a value through the
    /// `VacantEntry`.
    pub fn key(&self) -> Index {
        self.index
    }

    /// Sets the value of the entry with the `VacantEntry`'s key,
    /// and returns a mutable reference to it.
    ///
    /// This calls [`Arena::insert_at`] internally, so it is capable of
    /// "resurrecting" an old index.
    pub fn insert(self, value: T) -> &'a mut T {
        self.arena.insert_at(self.index, value);
        self.arena
            .get_mut(self.index)
            .unwrap_or_else(|| unreachable!("insert_at must create an occupied entry"))
    }
}

impl<'a, T> OccupiedEntry<'a, T> {
    /// Gets the key in the entry.
    pub fn key(&self) -> Index {
        self.index
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &T {
        self.arena
            .get(self.index)
            .unwrap_or_else(|| unreachable!("OccupiedEntry points to a vacant slot"))
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// If you need a reference to the `OccupiedEntry` that may outlive the
    /// destruction of the `Entry` value, see [`into_mut`].
    ///
    /// [`into_mut`]: OccupiedEntry::into_mut
    pub fn get_mut(&mut self) -> &mut T {
        self.arena
            .get_mut(self.index)
            .unwrap_or_else(|| unreachable!("OccupiedEntry points to a vacant slot"))
    }

    /// Converts the entry into a mutable reference to its value.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
    ///
    /// [`get_mut`]: OccupiedEntry::get_mut
    pub fn into_mut(self) -> &'a mut T {
        self.arena
            .get_mut(self.index)
            .unwrap_or_else(|| unreachable!("OccupiedEntry points to a vacant slot"))
    }

    /// Sets the value of the entry with the `OccupiedEntry`'s key,
    /// and returns the entry's old value.
    pub fn insert(&mut self, value: T) -> T {
        core::mem::replace(self.get_mut(), value)
    }

    /// Takes the value of the entry out of the arena, and returns it.
    pub fn remove(self) -> T {
        self.arena
            .remove(self.index)
            .unwrap_or_else(|| unreachable!("OccupiedEntry points to a vacant slot"))
    }
}

impl<T> Arena<T> {
    /// Gets the given key's corresponding entry in the arena for in-place
    /// manipulation.
    ///
    /// The entry is occupied if `index` is currently contained in the arena,
    /// and vacant otherwise (that is, when the slot is empty, out of bounds, or
    /// occupied by a different generation).
    pub fn entry(&mut self, index: Index) -> Entry<'_, T> {
        if self.contains(index) {
            Entry::Occupied(OccupiedEntry { arena: self, index })
        } else {
            Entry::Vacant(VacantEntry { arena: self, index })
        }
    }

    /// Gets a vacant entry in the arena, with its key computed up front.
    ///
    /// Unlike [`Arena::entry`], `vacant_entry` computes the same key that
    /// [`Arena::insert`] would produce, without inserting a value yet, using
    /// [`Arena::next_index`].
    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T> {
        let index = self.next_index();
        VacantEntry { arena: self, index }
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::Entry;
    use crate::arena::{Arena, Index};
    use crate::generation::Generation;

    #[test]
    fn occupied_get_modify_remove() {
        let mut arena = Arena::new();
        let index = arena.insert(10);

        match arena.entry(index) {
            Entry::Occupied(mut entry) => {
                assert_eq!(entry.key(), index);
                assert_eq!(entry.get(), &10);

                *entry.get_mut() = 20;
                assert_eq!(entry.insert(30), 20);
            }
            Entry::Vacant(_) => panic!(),
        }

        assert_eq!(arena[index], 30);

        // Now let's vacate back the slot under index.
        match arena.entry(index) {
            Entry::Occupied(entry) => {
                assert_eq!(entry.remove(), 30);
            }
            Entry::Vacant(_) => panic!(),
        }

        assert!(!arena.contains(index));
    }

    #[test]
    fn vacant_or_insert_on_empty_slot() {
        let mut arena = Arena::new();
        let index = arena.insert(1);
        arena.remove(index);

        let value = arena.entry(index).or_insert(5);
        assert_eq!(*value, 5);
        assert_eq!(arena[index], 5);
    }

    #[test]
    fn vacant_or_insert_out_of_bounds() {
        let mut arena = Arena::new();
        let index = Index {
            slot: 3,
            generation: Generation::first(),
        };

        arena.entry(index).or_insert(42);
        assert_eq!(arena[index], 42);
        // `.len()` is not contiguous length, but number of occupied slots.
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn vacant_wrong_generation_displaces() {
        let mut arena = Arena::new();
        let first = arena.insert("first");
        let invalidated = first;
        let second = arena.invalidate(first).unwrap();

        assert!(matches!(arena.entry(invalidated), Entry::Vacant(_)));
        assert!(matches!(arena.entry(second), Entry::Occupied(_)));

        // Entry API, just like `.insert_at()`, can resurrect an invalidated
        // index.
        arena.entry(invalidated).or_insert("resurrected");
        assert_eq!(arena[invalidated], "resurrected");
        assert!(!arena.contains(second));
    }

    #[test]
    fn and_modify_then_or_insert() {
        let mut arena = Arena::new();
        let index = arena.insert(1);
        arena.remove(index);

        // Nothing exists on index, so += 10 is not done.
        arena.entry(index).and_modify(|v| *v += 10).or_insert(5);
        assert_eq!(arena[index], 5);

        // Previous or_insert(5) filled a slot, so += 10 works now.
        arena.entry(index).and_modify(|v| *v += 10).or_insert(5);
        assert_eq!(arena[index], 15);
    }

    #[test]
    fn or_default() {
        let mut arena = Arena::new();
        let index = arena.insert(1);
        arena.remove(index);

        arena.entry(index).or_default();
        assert_eq!(arena[index], 0);
    }

    #[test]
    fn or_insert_with_key() {
        let mut arena = Arena::new();
        let index = arena.insert(0);
        arena.remove(index);

        arena
            .entry(index)
            .or_insert_with_key(|key| key.slot() as i32 + 7);
        assert_eq!(arena[index], index.slot() as i32 + 7);
    }

    #[test]
    fn vacant_key() {
        let mut arena = Arena::new();
        let index = arena.insert(1);
        arena.remove(index);

        match arena.entry(index) {
            Entry::Vacant(entry) => {
                assert_eq!(entry.key(), index);
            }
            Entry::Occupied(_) => panic!("expected vacant"),
        }
    }

    #[test]
    fn vacant_entry_key_matches_insert() {
        let mut arena = Arena::new();
        let predicted = arena.vacant_entry().key();
        let actual = arena.insert(10);
        assert_eq!(predicted, actual);
    }

    #[test]
    fn vacant_entry_insert() {
        let mut arena = Arena::new();
        let entry = arena.vacant_entry();
        let key = entry.key();

        let value = entry.insert(42);
        assert_eq!(*value, 42);
        assert_eq!(arena[key], 42);
    }

    #[test]
    fn occupied_into_mut() {
        let mut arena = Arena::new();
        let index = arena.insert(1);

        let value: &mut i32 = match arena.entry(index) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(_) => panic!("expected occupied"),
        };

        *value = 9;
        assert_eq!(arena[index], 9); // Mutable reference works.
    }
}
