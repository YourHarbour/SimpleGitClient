//! Toolkit-independent git engine: models, lane layout, diff parsing, async CLI.

pub mod diff;
pub mod graph;
pub mod models;
pub mod service;


pub use service::{GitService};
