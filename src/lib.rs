/*!
Generational arena type inspired by
[generational-arena](https://crates.io/crates/generational-arena) and
[slotmap](https://crates.io/crates/slotmap).

## Basic Examples

```rust
# use thunderdome::{Arena, Index};
let mut arena = Arena::new();

let foo = arena.insert("Foo");
let bar = arena.insert("Bar");

assert_eq!(arena[foo], "Foo");
assert_eq!(arena[bar], "Bar");

arena[bar] = "Replaced";
assert_eq!(arena[bar], "Replaced");

let foo_value = arena.remove(foo);
assert_eq!(foo_value, Some("Foo"));

// The slot previously used by foo will be reused for baz
let baz = arena.insert("Baz");
assert_eq!(arena[baz], "Baz");

// foo is no longer a valid key
assert_eq!(arena.get(foo), None);
```

## Comparison With Similar Crates

| Feature                      | Thunderdome | generational-arena | slotmap | slab |
|------------------------------|-------------|--------------------|---------|------|
| Generational Indices         | Yes         | Yes                | Yes     | No   |
| `size_of::<Index>()`         | 16          | 16                 | 8       | 8    |
| `size_of::<Option<Index>>()` | 16          | 24                 | 8       | 16   |
| Non-`Copy` Values            | Yes         | Yes                | Sorta¹  | Yes  |
| no-std support               | No          | Yes                | No      | No   |
| Serde support                | No          | Yes                | Yes     | No   |
| Should be used               | No          | Yes                | Yes     | Yes  |

* Sizes calculated on rustc `1.44.0-x86_64-pc-windows-msvc`

1. slotmap's `SlotMap` and `HopSlotMap` require values to be `Copy` on stable
  Rust versions. slotmap's `DenseSlotMap` type supports non-`Copy` types on
  stable, but has different performance trade-offs.
*/

#![forbid(missing_docs)]

mod arena;
mod free_pointer;
mod generation;

pub use arena::{Arena, Drain, Index};
