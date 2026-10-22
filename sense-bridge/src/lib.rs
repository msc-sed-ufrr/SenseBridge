#![no_std]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod bridge;
pub mod open_meteo;
pub mod reading;
pub mod record;
pub mod sim;
pub mod source;
pub mod state;
pub mod validation;

// API pública principal do SenseBridge
pub use bridge::{Config, SenseBridge};

pub use reading::{ExternalFailure, ExternalReading, InvalidReason, LocalFailure, LocalReading};

pub use record::{ExternalSample, Record};

pub use source::{ExternalSource, LocalSource};

pub use state::{OperationState, Transition, determine_state};

pub use validation::{Limits, Range, validate_external, validate_local};
