//! Fonte local: sensor DHT11 lido por um pino GPIO.

use dht_sensor::{DhtError, dht11};
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{InputOutput, PinDriver};
use esp_idf_svc::sys::EspError;
use sense_bridge::{LocalFailure, LocalReading, LocalSource};

/// DHT11 ligado a um pino em dreno aberto, com pull-up.
///
/// O protocolo do sensor fica com a crate `dht-sensor`; aqui o resultado é apenas convertido
/// para o modelo da biblioteca. Quem chama `read` respeita o intervalo mínimo de amostragem.
pub struct Dht11<'d> {
    pin: PinDriver<'d, InputOutput>,
}

impl<'d> Dht11<'d> {
    pub fn new(mut pin: PinDriver<'d, InputOutput>) -> Result<Self, EspError> {
        // A linha fica solta, em nível alto, até a primeira leitura: o sensor interpreta um
        // nível baixo como pedido de leitura.
        pin.set_high()?;
        Ok(Self { pin })
    }
}

impl LocalSource for Dht11<'_> {
    fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        // `Ets` espera ativamente: o protocolo mede pulsos de dezenas de microssegundos, bem
        // abaixo da resolução do escalonador.
        match dht11::blocking::read(&mut Ets, &mut self.pin) {
            Ok(reading) => Ok(LocalReading::new(
                f32::from(reading.temperature),
                f32::from(reading.relative_humidity),
            )),
            Err(DhtError::ChecksumMismatch) => Err(LocalFailure::Checksum),
            Err(DhtError::Timeout | DhtError::PinError(_)) => Err(LocalFailure::NoResponse),
        }
    }
}
