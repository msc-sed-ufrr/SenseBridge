//! Integração resiliente de dados ambientais de uma fonte local e de uma fonte externa.
//!
//! A cada [`SenseBridge::cycle`], a biblioteca consulta as duas fontes, valida as leituras
//! e produz um [`Record`] com a leitura ou o motivo da falha de cada fonte, o
//! [`OperationState`] resultante e a [`Transition`] em relação ao ciclo anterior.
//!
//! As fontes são abstraídas pelas traits [`LocalSource`] e [`ExternalSource`]; o firmware
//! fornece as implementações reais e os testes usam as fontes de [`sim`].
//!
//! ```
//! use sense_bridge::sim::{SimulatedExternal, SimulatedLocal};
//! use sense_bridge::{Config, ExternalReading, LocalFailure, LocalReading, OperationState, SenseBridge};
//!
//! let local = SimulatedLocal::new(Ok(LocalReading::new(25.0, 60.0)));
//! let external = SimulatedExternal::new(Ok(ExternalReading::new(27.0, 70.0)));
//! let mut bridge = SenseBridge::new(local, external, Config::default());
//!
//! assert_eq!(bridge.cycle(0).state(), OperationState::Normal);
//!
//! bridge.local_mut().set(Err(LocalFailure::NoResponse));
//! let record = bridge.cycle(2_000);
//! assert_eq!(record.state(), OperationState::DegradedLocal);
//! assert!(record.external.is_ok());
//! ```
#![cfg_attr(not(test), no_std)]

mod bridge;
mod reading;
mod record;
pub mod sim;
mod source;
mod state;
mod validation;

pub use bridge::{Config, SenseBridge};
pub use reading::{ExternalFailure, ExternalReading, InvalidReason, LocalFailure, LocalReading};
pub use record::{ExternalSample, Record};
pub use source::{ExternalSource, LocalSource};
pub use state::{OperationState, Transition, determine_state};
pub use validation::{Limits, Range, validate_external, validate_local};

// Executa os exemplos do README como doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
