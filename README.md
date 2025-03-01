# Rust `Map` based on `Vec` for serialization purposes

[![Crate](https://img.shields.io/crates/v/serde_map.svg)](https://crates.io/crates/serde_map)
[![API](https://docs.rs/serde_map/badge.svg)](https://docs.rs/serde_map)

Made mainly for deserialization speedup, because deserializing to an actual `HashMap` is a lot slower. It also saves the original order of the data.

Usage examples: to deserialize and then `.into_iter()`; or to transfer data between different storages.

Also it has a trait `SerdeMapStrategy`, which helps to process data (currently only keys) at the serializing/deserializing stage, before saving to the inner `Vec` (example in docs).