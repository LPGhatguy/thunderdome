# Thunderdome Changelog

## Unreleased Changes
* Added `Arena::with_capacity` for preallocating space.
* Added `Arena::len`, `Arena::capacity`, and `Arena::is_empty`.
* Improved panic-on-wrap guarantees, especially around unsafe code.
* Simplified and documented implementation.

## 0.1.0 (2020-09-02)
* Initial release
* Pretty much completely untested
* You probably shouldn't use this version