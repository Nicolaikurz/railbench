//! Pure Rust core engine for RAILbench.
//!
//! Shared by both the WebAssembly kernel (`railbench-wasm`) and future native Rust desktop builds.

pub mod geometry;
pub mod io;
pub mod ramp;
pub mod simulation;
pub mod snapping;
pub mod terrain;
pub mod types;
