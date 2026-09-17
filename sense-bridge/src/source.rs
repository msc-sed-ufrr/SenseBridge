use crate::reading::{ExternalFailure, ExternalReading, LocalFailure, LocalReading};

/// Fonte local de dados ambientais, como o DHT11.
///
/// Quem chama [`SenseBridge::cycle`](crate::SenseBridge::cycle) define o intervalo entre
/// leituras e deve respeitar o intervalo mínimo de amostragem do sensor.
pub trait LocalSource {
    fn read(&mut self) -> Result<LocalReading, LocalFailure>;
}

/// Fonte externa de dados meteorológicos, como a Open-Meteo.
///
/// A implementação aplica o tempo limite da consulta e devolve
/// [`ExternalFailure::Timeout`] quando ele é excedido.
pub trait ExternalSource {
    fn fetch(&mut self) -> Result<ExternalReading, ExternalFailure>;
}
