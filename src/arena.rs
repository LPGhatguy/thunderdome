use std::convert::TryInto;
use std::mem::replace;
use std::ops;

use crate::drain::Drain;
use crate::free_pointer::FreePointer;
use crate::generation::Generation;
use crate::index::create_index;
use crate::into_iter::IntoIter;
use crate::iter::Iter;
use crate::iter_mut::IterMut;

// shorthand for docstring
#[allow(unused)]
use crate::index::Index;

/// Container that can have elements inserted into it and removed from it.
///
/// Indices use the [`Index`] type, created by inserting values with [`Arena::insert`].
#[derive(Debug, Clone)]
pub struct Arena<T> {
    storage: Vec<Entry<T>>,
    len: u32,
    first_free: Option<FreePointer>,
}

#[derive(Debug, Clone)]
pub(crate) enum Entry<T> {
    Occupied(OccupiedEntry<T>),
    Empty(EmptyEntry),
}

impl<T> Entry<T> {
    /// Consume the entry, and if it's occupied, return the value.
    fn into_value(self) -> Option<T> {
        match self {
            Entry::Occupied(occupied) => Some(occupied.value),
            Entry::Empty(_) => None,
        }
    }

    /// If the entry is empty, return a copy of the emptiness data.
    fn get_empty(&self) -> Option<EmptyEntry> {
        match self {
            Entry::Empty(empty) => Some(*empty),
            Entry::Occupied(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OccupiedEntry<T> {
    pub(crate) generation: Generation,
    pub(crate) value: T,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EmptyEntry {
    pub(crate) generation: Generation,
    pub(crate) next_free: Option<FreePointer>,
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
        self.len as usize
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
    pub fn insert(&mut self, value: T) -> IndexT!(T) {
        // This value will definitely be inserted, so we can update length now.
        self.len = self
            .len
            .checked_add(1)
            .unwrap_or_else(|| panic!("Cannot insert more than u32::MAX elements into Arena"));

        // If there was a previously free entry, we can re-use its slot as long
        // as we increment its generation.
        if let Some(free_pointer) = self.first_free {
            let slot = free_pointer.slot();
            let entry = self.storage.get_mut(slot as usize).unwrap_or_else(|| {
                unreachable!("first_free pointed past the end of the arena's storage")
            });

            let empty = entry
                .get_empty()
                .unwrap_or_else(|| unreachable!("first_free pointed to an occupied entry"));

            // If there is another empty entry after this one, we'll update the
            // arena to point to it to use it on the next insertion.
            self.first_free = empty.next_free;

            // Overwrite the entry directly using our mutable reference instead
            // of indexing into our storage again. This should avoid an
            // additional bounds check.
            let generation = empty.generation.next();
            *entry = Entry::Occupied(OccupiedEntry { generation, value });

            create_index(slot, generation)
        } else {
            // There were no more empty entries left in our free list, so we'll
            // create a new first-generation entry and push it into storage.

            let generation = Generation::first();
            let slot: u32 = self.storage.len().try_into().unwrap_or_else(|_| {
                unreachable!("Arena storage exceeded what can be represented by a u32")
            });

            self.storage
                .push(Entry::Occupied(OccupiedEntry { generation, value }));

            create_index(slot, generation)
        }
    }

    /// Returns true if the given index is valid for the arena.
    pub fn contains(&self, index: IndexT!(T)) -> bool {
        match self.storage.get(index.slot as usize) {
            Some(Entry::Occupied(occupied)) if occupied.generation == index.generation => true,
            _ => false,
        }
    }

    /// Checks to see whether a slot is occupied in the arena, and if it is,
    /// returns `Some` with the true `Index` of that slot (slot plus generation.)
    /// Otherwise, returns `None`.
    pub fn contains_slot(&self, slot: u32) -> Option<IndexT!(T)> {
        match self.storage.get(slot as usize) {
            Some(Entry::Occupied(occupied)) => Some(create_index(slot, occupied.generation)),
            _ => None,
        }
    }

    /// Get an immutable reference to a value inside the arena by
    /// [`Index`], returning `None` if the index is not contained in the arena.
    pub fn get(&self, index: IndexT!(T)) -> Option<&T> {
        match self.storage.get(index.slot as usize) {
            Some(Entry::Occupied(occupied)) if occupied.generation == index.generation => {
                Some(&occupied.value)
            }
            _ => None,
        }
    }

    /// Get a mutable reference to a value inside the arena by [`Index`],
    /// returning `None` if the index is not contained in the arena.
    pub fn get_mut(&mut self, index: IndexT!(T)) -> Option<&mut T> {
        match self.storage.get_mut(index.slot as usize) {
            Some(Entry::Occupied(occupied)) if occupied.generation == index.generation => {
                Some(&mut occupied.value)
            }
            _ => None,
        }
    }

    /// Get mutable references of two values inside this arena at once by
    /// [`Index`], returning `None` if the corresponding `index` is not
    /// contained in this arena.
    ///
    /// # Panics
    ///
    /// This function panics when the two indices are equal (having the same
    /// slot number and generation).
    pub fn get2_mut(
        &mut self,
        index1: IndexT!(T),
        index2: IndexT!(T),
    ) -> (Option<&mut T>, Option<&mut T>) {
        if index1 == index2 {
            panic!("Arena::get2_mut is called with two identical indices");
        }

        // SAFETY NOTES:
        //
        // - If `index1` and `index2` have different slot number, `item1` and
        //   `item2` would point to different elements.
        // - If `index1` and `index2` have the same slot number, only one could
        //   be valid because there is only one valid generation number.
        // - If `index1` and `index2` have the same slot number and the same
        //   generation, this function will panic.
        //
        // Since `Vec::get_mut` will not reallocate, we can safely cast
        // a mutable reference to an element to a pointer and back and remain
        // valid.

        // Hold the first value in a pointer to sidestep the borrow checker
        let item1_ptr = self.get_mut(index1).map(|x| x as *mut T);

        let item2 = self.get_mut(index2);
        let item1 = unsafe { item1_ptr.map(|x| &mut *x) };

        (item1, item2)
    }

    /// Remove the value contained at the given index from the arena, returning
    /// it if it was present.
    pub fn remove(&mut self, index: IndexT!(T)) -> Option<T> {
        let entry = self.storage.get_mut(index.slot as usize)?;

        match entry {
            Entry::Occupied(occupied) if occupied.generation == index.generation => {
                // We can replace an occupied entry with an empty entry with the
                // same generation. On next insertion, this generation will
                // increment.
                let new_entry = Entry::Empty(EmptyEntry {
                    generation: occupied.generation,
                    next_free: self.first_free,
                });

                // Swap our new entry into our storage and take ownership of the
                // old entry. We'll consume it for its value so we can give that
                // back to our caller.
                let old_entry = replace(entry, new_entry);
                let value = old_entry.into_value().unwrap_or_else(|| unreachable!());

                // The next time we insert, we can re-use the empty entry we
                // just created. If another removal happens before then, that
                // entry will be used before this one (FILO).
                self.first_free = Some(FreePointer::from_slot(index.slot));

                self.len = self.len.checked_sub(1).unwrap_or_else(|| unreachable!());

                Some(value)
            }
            _ => None,
        }
    }

    /// Invalidate the given index and return a new index to the same value. This
    /// is roughly equivalent to `remove` followed by `insert`, but much faster.
    /// If the old index is already invalid, this method returns `None`.
    pub fn invalidate(&mut self, index: IndexT!(T)) -> Option<IndexT!(T)> {
        let entry = self.storage.get_mut(index.slot as usize)?;

        match entry {
            Entry::Occupied(occupied) if occupied.generation == index.generation => {
                occupied.generation = occupied.generation.next();

                Some(create_index(index.slot, occupied.generation))
            }
            _ => None,
        }
    }

    /// Attempt to look up the given slot in the arena, disregarding any generational
    /// information, and retrieve an immutable reference to it. Returns `None` if the
    /// slot is empty.
    pub fn get_by_slot(&self, slot: u32) -> Option<(IndexT!(T), &T)> {
        match self.storage.get(slot as usize) {
            Some(Entry::Occupied(occupied)) => {
                let index = create_index(slot, occupied.generation);
                Some((index, &occupied.value))
            }
            _ => None,
        }
    }

    /// Attempt to look up the given slot in the arena, disregarding any generational
    /// information, and retrieve a mutable reference to it. Returns `None` if the
    /// slot is empty.
    pub fn get_by_slot_mut(&mut self, slot: u32) -> Option<(IndexT!(T), &mut T)> {
        match self.storage.get_mut(slot as usize) {
            Some(Entry::Occupied(occupied)) => {
                let index = create_index(slot, occupied.generation);
                Some((index, &mut occupied.value))
            }
            _ => None,
        }
    }

    /// Remove an entry in the arena by its slot, disregarding any generational info.
    /// Returns `None` if the slot was already empty.
    pub fn remove_by_slot(&mut self, slot: u32) -> Option<(IndexT!(T), T)> {
        let entry = self.storage.get_mut(slot as usize)?;

        match entry {
            Entry::Occupied(occupied) => {
                // Construct the index that would be used to access this entry.
                let index = create_index(slot, occupied.generation);

                // This occupied entry will be replaced with an empty one of the
                // same generation. Generation will be incremented on the next
                // insert.
                let next_entry = Entry::Empty(EmptyEntry {
                    generation: occupied.generation,
                    next_free: self.first_free,
                });

                // Swap new entry into place and consume the old one.
                let old_entry = replace(entry, next_entry);
                let value = old_entry.into_value().unwrap_or_else(|| unreachable!());

                // Set this entry as the next one that should be inserted into,
                // should an insertion happen.
                self.first_free = Some(FreePointer::from_slot(slot));

                self.len = self.len.checked_sub(1).unwrap_or_else(|| unreachable!());

                Some((index, value))
            }
            _ => None,
        }
    }

    /// Clear the arena and drop all elements.
    pub fn clear(&mut self) {
        self.drain().for_each(drop);
    }

    /// Iterate over all of the indexes and values contained in the arena.
    ///
    /// Iteration order is not defined.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.storage.iter().enumerate(),
            len: self.len,
        }
    }

    /// Iterate over all of the indexes and values contained in the arena, with
    /// mutable access to each value.
    ///
    /// Iteration order is not defined.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            inner: self.storage.iter_mut().enumerate(),
            len: self.len,
        }
    }

    /// Returns an iterator that removes each element from the arena.
    ///
    /// Iteration order is not defined.
    ///
    /// If the iterator is dropped before it is fully consumed, any uniterated
    /// items will be dropped from the arena, and the arena will be empty.
    /// The arena's capacity will not be changed.
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain {
            arena: self,
            slot: 0,
        }
    }

    /// Remove all entries in the `Arena` which don't satisfy the provided predicate.
    pub fn retain<F: FnMut(IndexT!(T), &mut T) -> bool>(&mut self, mut f: F) {
        for (i, entry) in self.storage.iter_mut().enumerate() {
            if let Entry::Occupied(occupied) = entry {
                let index = create_index(i as u32, occupied.generation);

                if !f(index, &mut occupied.value) {
                    // We can replace an occupied entry with an empty entry with the
                    // same generation. On next insertion, this generation will
                    // increment.
                    *entry = Entry::Empty(EmptyEntry {
                        generation: occupied.generation,
                        next_free: self.first_free,
                    });

                    // The next time we insert, we can re-use the empty entry we
                    // just created. If another removal happens before then, that
                    // entry will be used before this one (FILO).
                    self.first_free = Some(FreePointer::from_slot(index.slot));

                    // We just verified that this entry is (was) occupied, so there's
                    // trivially no way for this `checked_sub` to fail.
                    self.len = self.len.checked_sub(1).unwrap_or_else(|| unreachable!());
                }
            }
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Arena::new()
    }
}

impl<T> IntoIterator for Arena<T> {
    type Item = (IndexT!(T), T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            arena: self,
            slot: 0,
        }
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = (IndexT!(T), &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = (IndexT!(T), &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> ops::Index<IndexT!(T)> for Arena<T> {
    type Output = T;

    fn index(&self, index: IndexT!(T)) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!("No entry at index {:?}", index))
    }
}

impl<T> ops::IndexMut<IndexT!(T)> for Arena<T> {
    fn index_mut(&mut self, index: IndexT!(T)) -> &mut Self::Output {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("No entry at index {:?}", index))
    }
}

#[cfg(test)]
mod test {
    use super::Arena;

    use std::mem::size_of;

    #[test]
    fn size_of_index() {
        assert_eq!(size_of::<IndexT!(usize)>(), 8);
        assert_eq!(size_of::<Option<IndexT!(usize)>>(), 8);
    }

    #[test]
    fn new() {
        let arena: Arena<u32> = Arena::new();
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.capacity(), 0);
    }

    #[test]
    fn with_capacity() {
        let arena: Arena<u32> = Arena::with_capacity(8);
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.capacity(), 8);
    }

    #[test]
    fn insert_and_get() {
        let mut arena = Arena::new();

        let one = arena.insert(1);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.get(one), Some(&1));

        let two = arena.insert(2);
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.get(one), Some(&1));
        assert_eq!(arena.get(two), Some(&2));
    }

    #[test]
    fn insert_remove_get() {
        let mut arena = Arena::new();
        let one = arena.insert(1);

        let two = arena.insert(2);
        assert_eq!(arena.len(), 2);
        assert!(arena.contains(two));
        assert_eq!(arena.remove(two), Some(2));
        assert!(!arena.contains(two));

        let three = arena.insert(3);
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.get(one), Some(&1));
        assert_eq!(arena.get(three), Some(&3));
        assert_eq!(arena.get(two), None);
    }

    #[test]
    fn insert_remove_get_by_slot() {
        let mut arena = Arena::new();
        let one = arena.insert(1);

        let two = arena.insert(2);
        assert_eq!(arena.len(), 2);
        assert!(arena.contains(two));
        assert_eq!(arena.remove_by_slot(two.slot()), Some((two, 2)));
        assert!(!arena.contains(two));
        assert_eq!(arena.get_by_slot(two.slot()), None);

        let three = arena.insert(3);
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.get(one), Some(&1));
        assert_eq!(arena.get(three), Some(&3));
        assert_eq!(arena.get(two), None);
        assert_eq!(arena.get_by_slot(two.slot()), Some((three, &3)));
    }

    #[test]
    fn get_mut() {
        let mut arena = Arena::new();
        let foo = arena.insert(5);

        let handle = arena.get_mut(foo).unwrap();
        *handle = 6;

        assert_eq!(arena.get(foo), Some(&6));
    }

    #[test]
    fn get2_mut() {
        let mut arena = Arena::new();
        let foo = arena.insert(100);
        let bar = arena.insert(500);

        let (foo_handle, bar_handle) = arena.get2_mut(foo, bar);
        let foo_handle = foo_handle.unwrap();
        let bar_handle = bar_handle.unwrap();
        *foo_handle = 105;
        *bar_handle = 505;

        assert_eq!(arena.get(foo), Some(&105));
        assert_eq!(arena.get(bar), Some(&505));
    }

    #[test]
    fn get2_mut_reversed_order() {
        let mut arena = Arena::new();
        let foo = arena.insert(100);
        let bar = arena.insert(500);

        let (bar_handle, foo_handle) = arena.get2_mut(bar, foo);
        let foo_handle = foo_handle.unwrap();
        let bar_handle = bar_handle.unwrap();
        *foo_handle = 105;
        *bar_handle = 505;

        assert_eq!(arena.get(foo), Some(&105));
        assert_eq!(arena.get(bar), Some(&505));
    }

    #[test]
    fn get2_mut_non_exist_handle() {
        let mut arena = Arena::new();
        let foo = arena.insert(100);
        let bar = arena.insert(500);
        arena.remove(bar);

        let (bar_handle, foo_handle) = arena.get2_mut(bar, foo);
        let foo_handle = foo_handle.unwrap();
        assert!(bar_handle.is_none());
        *foo_handle = 105;

        assert_eq!(arena.get(foo), Some(&105));
    }

    #[test]
    fn get2_mut_same_slot_different_generation() {
        let mut arena = Arena::new();
        let foo = arena.insert(100);
        let mut foo1 = foo;
        foo1.generation = foo1.generation.next();

        let (foo_handle, foo1_handle) = arena.get2_mut(foo, foo1);
        assert!(foo_handle.is_some());
        assert!(foo1_handle.is_none());
    }

    #[test]
    #[should_panic]
    fn get2_mut_panics() {
        let mut arena = Arena::new();
        let foo = arena.insert(100);

        arena.get2_mut(foo, foo);
    }

    #[test]
    fn insert_remove_insert_capacity() {
        let mut arena = Arena::with_capacity(2);
        assert_eq!(arena.capacity(), 2);

        let a = arena.insert("a");
        let b = arena.insert("b");
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.capacity(), 2);

        arena.remove(a);
        arena.remove(b);
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.capacity(), 2);

        let _a2 = arena.insert("a2");
        let _b2 = arena.insert("b2");
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.capacity(), 2);
    }

    #[test]
    fn invalidate() {
        let mut arena = Arena::new();

        let a = arena.insert("a");
        assert_eq!(arena.get(a), Some(&"a"));

        let new_a = arena.invalidate(a).unwrap();
        assert_eq!(arena.get(a), None);
        assert_eq!(arena.get(new_a), Some(&"a"));
    }

    #[test]
    fn retain() {
        let mut arena = Arena::new();

        for i in 0..100 {
            arena.insert(i);
        }

        arena.retain(|_, &mut i| i % 2 == 1);

        for (_, i) in arena.iter() {
            assert_eq!(i % 2, 1);
        }

        assert_eq!(arena.len(), 50);
    }

    #[test]
    fn index_bits_roundtrip() {
        let index = <IndexT!(u64)>::from_bits(0x1BADCAFE_DEADBEEF);
        assert_eq!(index.to_bits(), 0x1BADCAFE_DEADBEEF);
    }

    #[test]
    #[should_panic]
    fn index_bits_panic_on_zero_generation() {
        <IndexT!(u64)>::from_bits(0x00000000_DEADBEEF);
    }
}
