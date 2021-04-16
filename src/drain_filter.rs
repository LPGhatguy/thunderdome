use std::iter::FusedIterator;

use crate::arena::{Arena, Index};

/// See [`Arena::drain_filter`].
pub struct DrainFilter<'a, T, F: FnMut(&mut T) -> bool> {
    pub(crate) arena: &'a mut Arena<T>,
    pub(crate) slot: u32,
    pub(crate) predicate: F,
}

impl<'a, T, F: FnMut(&mut T) -> bool> Iterator for DrainFilter<'a, T, F> {
    type Item = (Index, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.slot as usize >= self.arena.len_storage() {
                return None;
            }

            // slot may overflow if the arena's underlying storage contains more
            // than 2^32 elements, but its internal length value was not
            // changed, as it overflowing would panic before reaching this code.
            let slot = self.slot;
            self.slot = self
                .slot
                .checked_add(1)
                .unwrap_or_else(|| panic!("Overflowed u32 trying to drain Arena"));

            // If this entry is occupied, this method will mark it as an empty.
            // Otherwise, we'll continue looping until we've drained all
            // occupied entries from the arena.
            if let Some((index, value)) = self.arena.get_by_slot_mut(slot) {
                if (self.predicate)(value) {
                    let value = self.arena.remove(index).unwrap();
                    return Some((index, value));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            0,
            Some(
                self.arena
                    .len_storage()
                    .checked_sub(self.slot as usize)
                    .unwrap(),
            ),
        )
    }
}

impl<'a, T, F: FnMut(&mut T) -> bool> FusedIterator for DrainFilter<'a, T, F> {}

#[cfg(test)]
mod test {
    use crate::Arena;

    use std::collections::HashSet;

    #[test]
    fn drain_filter() {
        let mut arena = Arena::with_capacity(2);
        let one = arena.insert(1);
        let two = arena.insert(2);

        // remove `1` from the arena
        let mut drained_pairs = HashSet::new();
        {
            let mut drain_filter = arena.drain_filter(|x| *x == 1);
            assert_eq!(drain_filter.size_hint(), (0, Some(2)));

            drained_pairs.insert(drain_filter.next().unwrap());
            assert_eq!(drain_filter.size_hint(), (0, Some(1)));

            // Do not fully drain so we can ensure everything is dropped when the
            // `DrainFilter` is dropped.
            assert_eq!(drain_filter.size_hint(), (0, Some(1)));
        }

        assert_eq!(arena.len(), 1);
        assert_eq!(arena.capacity(), 2);
        assert_eq!(drained_pairs.len(), 1);

        // We should still be able to use the arena after this.
        let three_prime = arena.insert(3);
        let four_prime = arena.insert(4);

        assert_eq!(arena.len(), 3);
        assert_eq!(arena.capacity(), 4);
        assert_eq!(arena.get(three_prime), Some(&3));
        assert_eq!(arena.get(four_prime), Some(&4));
        assert_eq!(arena.get(one), None);
        assert_eq!(arena.get(two), Some(&2));
    }
}
