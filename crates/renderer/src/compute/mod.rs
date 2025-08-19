//! GPU compute shader infrastructure for data processing

pub mod close_extractor;
pub mod compute_processor;
pub mod ema_calculator;
pub mod mid_price_calculator;

pub use close_extractor::CloseExtractor;
pub use compute_processor::{ComputeInfrastructure, ComputeProcessor, ComputeResult};
pub use ema_calculator::{EmaCalculator, EmaPeriod};
pub use mid_price_calculator::MidPriceCalculator;
