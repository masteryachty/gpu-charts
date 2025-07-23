//! JavaScript/WASM bridge for GPU Charts
//!
//! This crate provides the main entry point for the web application,
//! orchestrating the data manager and renderer modules.

// WebGPU initialization module
pub mod webgpu_init;

// Main implementation
mod lib_impl;
pub use lib_impl::*;