//! Fontes simuladas para testes e exemplos sem hardware.

use crate::reading::{ExternalFailure, ExternalReading, LocalFailure, LocalReading};
use crate::source::{ExternalSource, LocalSource};

/// Fonte que devolve sempre o mesmo resultado, até ser alterado com [`Simulated::set`].
#[derive(Debug, Clone, Copy)]
pub struct Simulated<T, F> {
    result: Result<T, F>,
    calls: u32,
}

pub type SimulatedLocal = Simulated<LocalReading, LocalFailure>;
pub type SimulatedExternal = Simulated<ExternalReading, ExternalFailure>;

impl<T: Copy, F: Copy> Simulated<T, F> {
    pub const fn new(result: Result<T, F>) -> Self {
        Self { result, calls: 0 }
    }

    pub fn set(&mut self, result: Result<T, F>) {
        self.result = result;
    }

    /// Quantas vezes a fonte foi consultada.
    pub fn calls(&self) -> u32 {
        self.calls
    }

    fn next(&mut self) -> Result<T, F> {
        self.calls += 1;
        self.result
    }
}

impl LocalSource for SimulatedLocal {
    fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        self.next()
    }
}

impl ExternalSource for SimulatedExternal {
    fn fetch(&mut self) -> Result<ExternalReading, ExternalFailure> {
        self.next()
    }
}
