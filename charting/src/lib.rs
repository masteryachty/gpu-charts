// WebAssembly charting library for React integration

// Allow clippy warnings for this crate
#![allow(clippy::all)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// External crate aliases
extern crate nalgebra_glm as glm;

// Core modules
mod calcables;
mod controls;
mod drawables;
mod events;
mod line_graph;
mod renderer;
pub mod store_state;
mod wrappers;

// React integration module
#[cfg(target_arch = "wasm32")]
mod lib_react;

// Re-export the Chart class for React integration
#[cfg(target_arch = "wasm32")]
pub use lib_react::Chart;

// Also export manual_run for backward compatibility if needed
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn manual_run() {
    // This could be used for standalone mode if needed in the future
    // For now, just initialize logging
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            let _ = console_log::init_with_level(log::Level::Debug);
        }
    }
}
