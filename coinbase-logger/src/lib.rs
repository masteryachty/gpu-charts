pub mod connection;
pub mod data_types;
pub mod file_handlers;
pub mod websocket;

pub use connection::ConnectionHandler;
pub use data_types::TickerData;
pub use file_handlers::FileHandles;

use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;