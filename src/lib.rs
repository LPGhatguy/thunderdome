/*!
[![GitHub CI Status](https://github.com/LPGhatguy/thunderdome/workflows/CI/badge.svg)](https://github.com/Roblox/rbx-dom/actions)
[![thunderdome on crates.io](https://img.shields.io/crates/v/thunderdome.svg)](https://crates.io/crates/thunderdome)
[![thunderdome docs](https://img.shields.io/badge/docs-docs.rs-orange.svg)](https://docs.rs/thunderdome)

Thunderdome is a ~gladitorial~ generational arena inspired by
[generational-arena](https://crates.io/crates/generational-arena),
[slotmap](https://crates.io/crates/slotmap), and
[slab](https://crates.io/crates/slab). It provides constant time insertion,
lookup, and removal via small (8 byte) keys returned from `Arena`.

Thunderdome's key type, `Index`, is still 8 bytes when put inside of an
`Option<T>` thanks to Rust's `NonZero*` types.

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
| `size_of::<Index>()`         | 8           | 16                 | 8       | 8    |
| `size_of::<Option<Index>>()` | 8           | 24                 | 8       | 16   |
| Non-`Copy` Values            | Yes         | Yes                | Sorta¹  | Yes  |
| no-std support               | No          | Yes                | No      | No   |
| Serde support                | No          | Yes                | Yes     | No   |
| [Immune to ABA Problem][ABA] | Yes         | Yes                | Yes     | No   |

* Sizes calculated on rustc `1.44.0-x86_64-pc-windows-msvc`
* See [the Thunderdome comparison
  Cargo.toml](https://github.com/LPGhatguy/thunderdome/blob/main/comparison/Cargo.toml)
  for versions of each library tested.

1. slotmap's `SlotMap` and `HopSlotMap` require values to be `Copy` on stable
  Rust versions. slotmap's `DenseSlotMap` type supports non-`Copy` types on
  stable, but has different performance trade-offs.

## Minimum Supported Rust Version (MSRV)

Thunderdome supports Rust 1.34.1 and newer. Until Thunderdome reaches 1.0,
changes to the MSRV will require major version bumps. After 1.0, MSRV changes
will only require minor version bumps, but will need significant justification.

[ABA]: https://en.wikipedia.org/wiki/ABA_problem
*/

#![forbid(missing_docs)]

mod arena;
mod free_pointer;
mod generation;

pub use crate::arena::{Arena, Drain, Index};
