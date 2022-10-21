use core::cmp::Eq;
use core::iter::{ExactSizeIterator, FusedIterator};
use core::slice;

use crate::arena::{Entry, Index};

/// See [`Arena::iter_mut`](crate::Arena::iter_mut).
pub struct IterMut<'a, T, I = ()>
where
    I: Eq + PartialEq,
{
    pub(crate) len: u32,
    pub(crate) slot: u32,
    pub(crate) inner: slice::IterMut<'a, Entry<T>>,
    pub(crate) _marker: core::marker::PhantomData<I>,
}

impl<'a, T, I> Iterator for IterMut<'a, T, I>
where
    I: Eq + PartialEq,
{
    type Item = (Index<I>, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.len == 0 {
                return None;
            }

            let slot = self.slot;
            self.slot = self
                .slot
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("Overflowed u32 trying to iterate Arena"));

            match self.inner.next()? {
                Entry::Empty(_) => continue,
                Entry::Occupied(occupied) => {
                    self.len = self
                        .len
                        .checked_sub(1)
                        .unwrap_or_else(|| unreachable!("Underflowed u32 trying to iterate Arena"));

                    let index = Index {
                        slot,
                        generation: occupied.generation,
                        _marker: core::marker::PhantomData,
                    };

                    return Some((index, &mut occupied.value));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len as usize, Some(self.len as usize))
    }
}

impl<'a, T, I> FusedIterator for IterMut<'a, T, I> where I: Eq + PartialEq {}
impl<'a, T, I> ExactSizeIterator for IterMut<'a, T, I> where I: Eq + PartialEq {}

#[cfg(all(test, feature = "std"))]
mod test {
    use crate::Arena;

    use std::collections::HashSet;

    #[test]
    fn iter_mut() {
        let mut arena: Arena<u32> = Arena::with_capacity(2);
        let one = arena.insert(1);
        let two = arena.insert(2);

        let mut pairs = HashSet::new();
        let mut iter = arena.iter_mut();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        pairs.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(pairs.contains(&(one, &mut 1)));
        assert!(pairs.contains(&(two, &mut 2)));
    }
}
