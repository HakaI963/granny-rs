//! Granny — Rust reimplementation of `com.dvloper.granny` 1.8.12.
//!
//! The rules here were recovered from the shipped IL2CPP build; see
//! `docs/SPEC.md` for the mapping from decompiled methods to Rust functions.

#[cfg(target_os = "android")]
pub mod android;

pub mod app;
pub mod assets;
pub mod entities;
pub mod game;
pub mod render;
pub mod scene;

pub use game::{AiState, Difficulty, Door, DoorState, Item, Pickup, Player, World};
