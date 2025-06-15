pub mod error;
pub mod config;

// Re-export error types
pub use error::{
    ProcessorError,
    ProcessorResult,
};

// Re-export config types
pub use config::{SETTINGS, Settings, ConnectionRule, Action}; 