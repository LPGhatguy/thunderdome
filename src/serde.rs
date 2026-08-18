use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::arena::{Arena, OccupiedSlot, Slot, VacantSlot};
use crate::free_pointer::FreePointer;
use crate::generation::Generation;

impl<T: Serialize> Serialize for Arena<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.storage.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Arena<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut storage: Vec<Slot<T>> = Deserialize::deserialize(deserializer)?;

        if storage.len() >= u32::MAX as usize {
            return Err(de::Error::custom("too many slots for u32 index type"));
        }

        let mut len: u32 = 0;
        let mut next_free = None;

        for (i, slot) in storage.iter_mut().enumerate().rev() {
            match slot {
                Slot::Occupied(_) => {
                    len = len
                        .checked_add(1)
                        .expect("u32 len counter overflowed while deserializing");
                }
                Slot::Vacant(vacant) => {
                    vacant.next_free = next_free;
                    next_free = Some(FreePointer::from_slot(i as u32));
                }
            }
        }

        Ok(Self {
            len,
            storage,
            first_free: next_free,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct SlotSer<T> {
    generation: Generation,
    value: Option<T>,
}

impl<T: Serialize> Serialize for Slot<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let slot_ser = SlotSer {
            generation: self.generation(),
            value: self.get_value(self.generation()),
        };

        slot_ser.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Slot<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let slot_ser: SlotSer<T> = Deserialize::deserialize(deserializer)?;

        Ok(match slot_ser.value {
            Some(value) => Slot::Occupied(OccupiedSlot {
                generation: slot_ser.generation,
                value,
            }),
            None => Slot::Vacant(VacantSlot {
                generation: slot_ser.generation,
                next_free: None,
            }),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::arena::Index;

    #[test]
    fn empty_arena_roundtrip() {
        // It's worth it to test roundtrip for empty arena just to confirm we
        // didn't mess up first free pointer.

        let arena = Arena::<i16>::new();

        // Serialize to JSON and then deserialize back.
        let serialized = serde_json::to_string(&arena).unwrap();
        let deserialized: Arena<i16> = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.is_empty());
        assert_eq!(deserialized.len(), 0);
        assert!(deserialized.first_free.is_none());
    }

    #[test]
    fn contiguous_arena_roundtrip() {
        let mut arena = Arena::new();
        let a = arena.insert("a");
        let b = arena.insert("b");
        let c = arena.insert("c");

        let serialized = serde_json::to_string(&arena).unwrap();
        let deserialized: Arena<&str> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized.get(a), Some(&"a"));
        assert_eq!(deserialized.get(b), Some(&"b"));
        assert_eq!(deserialized.get(c), Some(&"c"));
    }

    #[test]
    fn arena_with_hole_roundtrip() {
        let mut arena = Arena::new();
        let a = arena.insert(1);
        let b = arena.insert(2);
        let c = arena.insert(3);
        arena.remove(b); // `b` is now the hole.

        let serialized = serde_json::to_string(&arena).unwrap();
        let mut deserialized: Arena<i16> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized.get(a), Some(&1));
        assert_eq!(deserialized.get(b), None);
        assert_eq!(deserialized.get(c), Some(&3));

        // Vacant slot is not skipped but is immediately reused even after serde
        // roundtrip.
        let d = deserialized.insert(4);
        assert_eq!(d.slot(), b.slot());
        assert_eq!(deserialized.get(d), Some(&4));
        assert_eq!(deserialized.len(), 3);
    }

    #[test]
    fn arena_roundtrip_generations_survive() {
        let mut arena = Arena::new();

        let first = arena.insert("first");
        arena.remove(first);

        let second = arena.insert("second");
        // `first` and `second` occupy the same slot but different generations.
        assert_eq!(first.slot(), second.slot());
        assert_ne!(first.generation(), second.generation());

        let serialized = serde_json::to_string(&arena).unwrap();
        let deserialized: Arena<&str> = serde_json::from_str(&serialized).unwrap();

        // No ABA problem.
        assert_eq!(deserialized.get(first), None);
        assert_eq!(deserialized.get(second), Some(&"second"));
    }

    #[test]
    fn zero_generation_slot_is_err() {
        // Zero-valued generations are invalid. Let's check if noone can inject
        // them through deseriallization.
        let result = serde_json::from_str::<Arena<i16>>(r#"[{"generation":0,"value":1}]"#);
        assert!(result.is_err());
    }
}
