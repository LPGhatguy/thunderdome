use serde::{de::Error as _, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::arena::Arena;
use crate::generation::Generation;

impl<T: Serialize> Serialize for Arena<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.storage().len()))?;

        for entry in self.storage() {
            let generation = entry.generation().to_u32();
            let value = entry.value();

            seq.serialize_element(&(generation, value))?;
        }

        seq.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Arena<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let sequence = <Vec<(u32, Option<T>)>>::deserialize(deserializer)?;
        let mut arena: Arena<T> = Arena::with_capacity(sequence.len());

        for (generation, value) in sequence {
            let generation = Generation::from_u32(generation)
                .ok_or_else(|| D::Error::custom(format!("Invalid generation {}", generation)))?;

            arena.push_slot(generation, value);
        }

        Ok(arena)
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use serde::{de::DeserializeOwned, Serialize};

    use crate::arena::{Arena, EmptyEntry, Entry, OccupiedEntry};
    use crate::free_pointer::FreePointer;
    use crate::generation::Generation;

    struct TestCase<T: 'static> {
        arena: Arena<T>,
        expected_json: &'static str,
        expected_storage: Vec<Entry<T>>,
    }

    fn test<'a, T: Serialize + DeserializeOwned + PartialEq + Debug>(case: TestCase<T>) {
        let json = serde_json::to_string(&case.arena).unwrap();
        assert_eq!(json, case.expected_json);

        let de: Arena<T> = serde_json::from_str(&json).unwrap();
        assert_eq!(de.storage(), &case.expected_storage);

        let expected_len = case
            .expected_storage
            .iter()
            .filter(|x| match x {
                Entry::Occupied(_) => true,
                Entry::Empty(_) => false,
            })
            .count();
        assert_eq!(de.len(), expected_len);
    }

    fn occupied<T>(generation: u32, value: T) -> Entry<T> {
        Entry::Occupied(OccupiedEntry {
            generation: Generation::from_u32(generation).unwrap(),
            value,
        })
    }

    fn empty<T>(generation: u32, next: Option<u32>) -> Entry<T> {
        Entry::Empty(EmptyEntry {
            generation: Generation::from_u32(generation).unwrap(),
            next_free: next.map(FreePointer::from_slot),
        })
    }

    #[test]
    fn round_trip_empty() {
        let arena: Arena<u32> = Arena::new();

        test(TestCase {
            arena,
            expected_json: "[]",
            expected_storage: vec![],
        });
    }

    #[test]
    fn all_occupied() {
        let mut arena: Arena<u32> = Arena::new();
        arena.insert(70);
        arena.insert(80);
        arena.insert(90);

        test(TestCase {
            arena,
            expected_json: "[[1,70],[1,80],[1,90]]",
            expected_storage: vec![occupied(1, 70), occupied(1, 80), occupied(1, 90)],
        });
    }

    #[test]
    fn inner_empty() {
        let mut arena: Arena<u32> = Arena::new();
        arena.insert(100);
        let second = arena.insert(101);
        arena.insert(102);
        arena.remove(second).unwrap();

        test(TestCase {
            arena,
            expected_json: "[[1,100],[1,null],[1,102]]",
            expected_storage: vec![occupied(1, 100), empty(1, None), occupied(1, 102)],
        });
    }

    #[test]
    fn trailing_empty() {
        let mut arena: Arena<u32> = Arena::new();
        arena.insert(10);
        arena.insert(11);
        let last = arena.insert(12);
        arena.remove(last).unwrap();

        test(TestCase {
            arena,
            expected_json: "[[1,10],[1,11],[1,null]]",
            expected_storage: vec![occupied(1, 10), occupied(1, 11), empty(1, None)],
        });
    }

    #[test]
    fn generations() {
        let mut arena: Arena<u32> = Arena::new();
        let mut handle = arena.insert(50);
        for i in 0..10 {
            arena.remove(handle);
            handle = arena.insert(50 + i);
        }

        test(TestCase {
            arena,
            expected_json: "[[11,59]]",
            expected_storage: vec![occupied(11, 59)],
        });
    }

    #[test]
    fn free_list() {
        let mut arena: Arena<u32> = Arena::new();
        let a = arena.insert(300);
        let b = arena.insert(400);
        let c = arena.insert(500);
        arena.remove(a).unwrap();
        arena.remove(b).unwrap();
        arena.remove(c).unwrap();

        test(TestCase {
            arena,
            expected_json: "[[1,null],[1,null],[1,null]]",
            expected_storage: vec![empty(1, None), empty(1, Some(0)), empty(1, Some(1))],
        })
    }
}
