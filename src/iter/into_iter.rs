use core::convert::TryInto;
use core::iter::{Enumerate, ExactSizeIterator, FusedIterator};

#[cfg(feature = "std")]
use std::vec;

#[cfg(not(feature = "std"))]
use alloc::vec;

use crate::arena::{Entry, Index};

/// Iterator typed used when an Arena is turned [`IntoIterator`].
pub struct IntoIter<T> {
    pub(crate) len: u32,
    pub(crate) inner: Enumerate<vec::IntoIter<Entry<T>>>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (Index, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.len == 0 {
                return None;
            }

            match self.inner.next()? {
                (_, Entry::Empty(_)) => (),
                (slot, Entry::Occupied(occupied)) => {
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

                    return Some((index, occupied.value));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len as usize, Some(self.len as usize))
    }
}

impl<T> FusedIterator for IntoIter<T> {}
impl<T> ExactSizeIterator for IntoIter<T> {}

#[cfg(all(test, feature = "std"))]
mod test {
    use crate::Arena;

    use std::collections::HashSet;

    #[test]
    fn into_iter() {
        let mut arena = Arena::with_capacity(2);
        let one = arena.insert(1);
        let two = arena.insert(2);

        let mut pairs = HashSet::new();
        let mut into_iter = arena.into_iter();
        assert_eq!(into_iter.size_hint(), (2, Some(2)));

        pairs.insert(into_iter.next().unwrap());
        assert_eq!(into_iter.size_hint(), (1, Some(1)));

        pairs.insert(into_iter.next().unwrap());
        assert_eq!(into_iter.size_hint(), (0, Some(0)));

        assert_eq!(into_iter.next(), None);
        assert_eq!(into_iter.next(), None);
        assert_eq!(into_iter.size_hint(), (0, Some(0)));

        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&(one, 1)));
        assert!(pairs.contains(&(two, 2)));
    }
}
