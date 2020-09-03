use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct Generation(NonZeroU64);

impl Generation {
    #[must_use]
    pub(crate) fn first() -> Self {
        Generation(unsafe { NonZeroU64::new_unchecked(1) })
    }

    #[must_use]
    pub(crate) fn next(self) -> Self {
        let last_generation = self.0.get();
        let next_generation = last_generation
            .checked_add(1)
            .expect("u64 overflowed calculating next generation");

        // This is safe because any u64 + 1 that didn't overflow must not be
        // zero.
        Generation(unsafe { NonZeroU64::new_unchecked(next_generation) })
    }
}

#[cfg(test)]
mod test {
    use super::Generation;

    use std::num::NonZeroU64;

    #[test]
    fn first_and_next() {
        let first = Generation::first();
        assert_eq!(first.0.get(), 1);

        let second = first.next();
        assert_eq!(second.0.get(), 2);
    }

    #[test]
    #[should_panic(expected = "u64 overflowed calculating next generation")]
    fn panic_on_overflow() {
        let max = Generation(NonZeroU64::new(std::u64::MAX).unwrap());
        let _next = max.next();
    }
}
