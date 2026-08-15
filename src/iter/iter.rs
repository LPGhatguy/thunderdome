use core::convert::TryInto;
use core::iter::{Enumerate, ExactSizeIterator, FusedIterator};
use core::slice;

use crate::arena::{Index, Slot};

/// See [`Arena::iter`](crate::Arena::iter).
#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    pub(crate) len: u32,
    pub(crate) inner: Enumerate<slice::Iter<'a, Slot<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Index, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.len == 0 {
                return None;
            }

            match self.inner.next()? {
                (_, Slot::Vacant(_)) => (),
                (slot, Slot::Occupied(occupied)) => {
                    self.len = self
                        .len
                        .checked_sub(1)
                        .unwrap_or_else(|| unreachable!("Underflowed u32 trying to iterate Arena"));

                    let slot = slot
                        .try_into()
                        .unwrap_or_else(|_| unreachable!("Overflowed u32 trying to iterate Arena"));

                    let index = Index {
                        slot,
                        generation: occupied.generation,
                    };

                    return Some((index, &occupied.value));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len as usize, Some(self.len as usize))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            if self.len == 0 {
                return None;
            }

            match self.inner.next_back()? {
                (_, Slot::Vacant(_)) => (),
                (slot, Slot::Occupied(occupied)) => {
                    self.len = self.len.checked_sub(1).unwrap_or_else(|| {
                        unreachable!("Underflowed u32 trying to iterate Arena in reverse")
                    });

                    let slot = slot
                        .try_into()
                        .unwrap_or_else(|_| unreachable!("Overflowed u32 trying to iterate Arena"));

                    let index = Index {
                        slot,
                        generation: occupied.generation,
                    };

                    return Some((index, &occupied.value));
                }
            }
        }
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

impl<T> Default for Iter<'_, T> {
    fn default() -> Self {
        Self {
            len: 0,
            inner: slice::Iter::<Slot<T>>::default().enumerate(),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use crate::Arena;

    use std::collections::HashSet;

    #[test]
    fn iter() {
        let mut arena = Arena::with_capacity(2);
        let one = arena.insert(1);
        let two = arena.insert(2);

        let mut pairs = HashSet::new();
        let mut iter = arena.iter();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(pairs.contains(&(one, &1)));
        assert!(pairs.contains(&(two, &2)));
    }

    #[test]
    fn iter_rev() {
        let mut arena = Arena::with_capacity(2);
        let one = arena.insert(1);
        let two = arena.insert(2);

        let mut pairs = HashSet::new();
        let mut iter = arena.iter().rev();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(pairs.contains(&(two, &2)));
        assert!(pairs.contains(&(one, &1)));
    }

    #[test]
    fn iter_both_directions() {
        let mut arena = Arena::with_capacity(2);
        let one = arena.insert(1);
        let two = arena.insert(2);
        let three = arena.insert(3);
        let four = arena.insert(4);

        let mut pairs = HashSet::new();
        let mut iter = arena.iter();
        assert_eq!(iter.size_hint(), (4, Some(4)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (3, Some(3)));

        pairs.insert(iter.next_back().unwrap());
        assert_eq!(iter.size_hint(), (2, Some(2)));

        pairs.insert(iter.next_back().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(pairs.contains(&(one, &1)));
        assert!(pairs.contains(&(two, &2)));
        assert!(pairs.contains(&(three, &3)));
        assert!(pairs.contains(&(four, &4)));
    }
}
