use core::iter::{ExactSizeIterator, FusedIterator};

use super::IterMut;

/// See [`Arena::values_mut`](crate::Arena::values_mut).
#[derive(Debug, Default)]
pub struct ValuesMut<'a, T> {
    pub(crate) inner: IterMut<'a, T>,
}

impl<'a, T> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, value)| value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for ValuesMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, value)| value)
    }
}

impl<'a, T> FusedIterator for ValuesMut<'a, T> {}
impl<'a, T> ExactSizeIterator for ValuesMut<'a, T> {}

#[cfg(all(test, feature = "std"))]
mod test {
    use crate::Arena;

    use std::collections::HashSet;

    #[test]
    fn values_mut() {
        let mut arena = Arena::with_capacity(2);
        arena.insert(1);
        arena.insert(2);

        let mut values = HashSet::new();
        let mut iter = arena.values_mut();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        let first = iter.next().unwrap();
        *first *= 10;
        values.insert(*first);
        assert_eq!(iter.size_hint(), (1, Some(1)));

        let second = iter.next().unwrap();
        *second *= 10;
        values.insert(*second);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(values.contains(&10));
        assert!(values.contains(&20));
    }

    #[test]
    fn values_mut_rev() {
        let mut arena = Arena::with_capacity(2);
        arena.insert(1);
        arena.insert(2);

        let mut values = HashSet::new();
        let mut iter = arena.values_mut().rev();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        values.insert(*iter.next().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        values.insert(*iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(values.contains(&1));
        assert!(values.contains(&2));
    }

    #[test]
    fn values_mut_both_directions() {
        let mut arena = Arena::with_capacity(2);
        arena.insert(1);
        arena.insert(2);
        arena.insert(3);
        arena.insert(4);

        let mut values = HashSet::new();
        let mut iter = arena.values_mut();
        assert_eq!(iter.size_hint(), (4, Some(4)));

        values.insert(*iter.next().unwrap());
        assert_eq!(iter.size_hint(), (3, Some(3)));

        values.insert(*iter.next_back().unwrap());
        assert_eq!(iter.size_hint(), (2, Some(2)));

        values.insert(*iter.next_back().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        values.insert(*iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(values.contains(&1));
        assert!(values.contains(&2));
        assert!(values.contains(&3));
        assert!(values.contains(&4));
    }
}
