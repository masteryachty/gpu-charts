//! GPU compute shader infrastructure for data processing

pub mod compute_processor;
pub mod mid_price_calculator;

pub use compute_processor::{ComputeInfrastructure, ComputeProcessor, ComputeResult};
pub use mid_price_calculator::MidPriceCalculator;
