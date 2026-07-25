//! The CSS half of the `contrast` proof: what a stylesheet actually declares,
//! per theme, and what a declared value actually resolves to.
//!
//! The proof itself lives elsewhere and maps register records onto what this
//! crate returns. Nothing here knows about the register, and nothing here
//! decides whether a resolved value passes anything.
//!
//! This file is a module root and holds no logic. A sibling module (`colour`)
//! is authored separately and adds its own `pub mod` line here.

pub mod sheet;
