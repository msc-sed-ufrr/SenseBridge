//! Medição da duração de cada consulta às fontes, feita fora da biblioteca (ADR-003).

use std::time::{Duration, Instant};

use sense_bridge::{
    ExternalFailure, ExternalReading, ExternalSource, LocalFailure, LocalReading, LocalSource,
};

/// Envolve uma fonte e registra a duração da última consulta.
pub struct Timed<S> {
    source: S,
    last: Option<Duration>,
}

impl<S> Timed<S> {
    pub fn new(source: S) -> Self {
        Self { source, last: None }
    }

    /// Duração da consulta feita desde a chamada anterior; `None` se a fonte não foi
    /// consultada, como quando a leitura externa é reutilizada.
    pub fn take(&mut self) -> Option<Duration> {
        self.last.take()
    }

    fn measure<T>(&mut self, call: impl FnOnce(&mut S) -> T) -> T {
        let start = Instant::now();
        let result = call(&mut self.source);
        self.last = Some(start.elapsed());
        result
    }
}

impl<S: LocalSource> LocalSource for Timed<S> {
    fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        self.measure(S::read)
    }
}

impl<S: ExternalSource> ExternalSource for Timed<S> {
    fn fetch(&mut self) -> Result<ExternalReading, ExternalFailure> {
        self.measure(S::fetch)
    }
}
