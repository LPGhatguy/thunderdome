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
    /// providing the default function a reference to the key that was moved
    /// during the `.entry(key)` method call.
    ///
    /// The reference to the moved key is provided so that cloning or copying
    /// the key is unnecessary, unlike with `.or_insert_with(|| ... )`.
    ///
    /// If this entry is vacant, this calls [`Arena::insert_at`] internally, so
    /// it is capable of "resurrecting" an old index.
    pub fn or_insert_with_key<F: FnOnce(&Index) -> T>(self, default: F) -> &'a mut T {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &Index {
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
    /// Gets a reference to the key that would be used when inserting a value
    /// through the `VacantEntry`.
    pub fn key(&self) -> &Index {
        &self.index
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> Index {
        self.index
    }

    /// Sets the value of the entry with the `VacantEntry`'s key,
    /// and returns a mutable reference to it.
    ///
    /// If this entry is vacant, this calls [`Arena::insert_at`] internally, so
    /// it is capable of "resurrecting" an old index.
    pub fn insert(self, value: T) -> &'a mut T {
        self.arena.insert_at(self.index, value);
        self.arena
            .get_mut(self.index)
            .unwrap_or_else(|| unreachable!("insert_at must create an occupied entry"))
    }
}

impl<'a, T> OccupiedEntry<'a, T> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &Index {
        &self.index
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
}
