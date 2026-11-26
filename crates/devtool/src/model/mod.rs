//! Pure domain state models and event reduction trait.
//!
//! Models are intentionally free of runtime I/O and GUI framework dependencies so
//! both CLI commands and a future `gpui` shell (`Entity<Model>`) can share them.

#![allow(dead_code)]

pub mod debug;
pub mod engine;
pub mod env;
pub mod project;
pub mod tasks;

pub trait DomainModel {
    type Event;

    /// Apply a domain event and return whether UI-visible state changed.
    fn apply(&mut self, event: Self::Event) -> bool;
}
