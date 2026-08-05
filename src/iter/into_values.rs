use core::iter::{ExactSizeIterator, FusedIterator};

use super::IntoIter;

/// See [`Arena::into_values`](crate::Arena::into_values).
#[derive(Clone, Debug)]
pub struct IntoValues<T> {
    pub(crate) inner: IntoIter<T>,
}

impl<T> Iterator for IntoValues<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, value)| value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoValues<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, value)| value)
    }
}

impl<T> FusedIterator for IntoValues<T> {}
impl<T> ExactSizeIterator for IntoValues<T> {}

#[cfg(all(test, feature = "std"))]
mod test {
    use crate::Arena;

    use std::collections::HashSet;

    #[test]
    fn into_values() {
        let mut arena = Arena::with_capacity(2);
        arena.insert(1);
        arena.insert(2);

        let mut values = HashSet::new();
        let mut iter = arena.into_values();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        values.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        values.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(values.len(), 2);
        assert!(values.contains(&1));
        assert!(values.contains(&2));
    }

    #[test]
    fn into_values_rev() {
        let mut arena = Arena::with_capacity(2);
        arena.insert(1);
        arena.insert(2);

        let mut values = HashSet::new();
        let mut iter = arena.into_values().rev();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        values.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        values.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert!(values.contains(&1));
        assert!(values.contains(&2));
    }

    #[test]
    fn into_values_both_directions() {
        let mut arena = Arena::with_capacity(2);
        arena.insert(1);
        arena.insert(2);
        arena.insert(3);
        arena.insert(4);

        let mut values = HashSet::new();
        let mut iter = arena.into_values();
        assert_eq!(iter.size_hint(), (4, Some(4)));

        values.insert(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (3, Some(3)));

        values.insert(iter.next_back().unwrap());
        assert_eq!(iter.size_hint(), (2, Some(2)));

        values.insert(iter.next_back().unwrap());
        assert_eq!(iter.size_hint(), (1, Some(1)));

        values.insert(iter.next().unwrap());
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
