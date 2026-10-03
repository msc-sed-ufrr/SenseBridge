//! Fonte local: sensor DHT11 lido por um pino GPIO.

use dht_sensor::{DhtError, dht11};
use embedded_hal::delay::DelayNs;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{InputOutput, PinDriver};
use esp_idf_svc::hal::interrupt::{IsrCriticalSection, IsrCriticalSectionGuard};
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
        let critical = IsrCriticalSection::new();
        let mut delay = FrameDelay { critical: &critical, guard: None };
        let result = dht11::blocking::read(&mut delay, &mut self.pin);
        drop(delay);
        match result {
            Ok(reading) => Ok(LocalReading::new(
                f32::from(reading.temperature),
                f32::from(reading.relative_humidity),
            )),
            Err(DhtError::ChecksumMismatch) => Err(LocalFailure::Checksum),
            Err(DhtError::Timeout | DhtError::PinError(_)) => Err(LocalFailure::NoResponse),
        }
    }
}

/// Atraso do protocolo que mascara as interrupções durante o quadro de dados.
///
/// O quadro dura cerca de 4 ms, com bits distinguidos por pulsos de 24 µs e 71 µs e lidos
/// por espera ativa. Uma preempção pela tarefa do Wi-Fi, no mesmo núcleo, pode passar de
/// 1 ms e corromper a leitura. Na `dht-sensor` 0.3, o único `delay_ms` é o pulso de início
/// (18 ms); o quadro começa logo depois, então a seção crítica abre ao fim desse atraso e
/// fecha quando o adaptador é descartado. O pulso de início fica fora dela.
struct FrameDelay<'a> {
    critical: &'a IsrCriticalSection,
    guard: Option<IsrCriticalSectionGuard<'a>>,
}

impl DelayNs for FrameDelay<'_> {
    // `Ets` espera ativamente: o protocolo mede pulsos de dezenas de microssegundos, bem
    // abaixo da resolução do escalonador.
    fn delay_ns(&mut self, ns: u32) {
        Ets.delay_ns(ns);
    }

    fn delay_us(&mut self, us: u32) {
        Ets::delay_us(us);
    }

    fn delay_ms(&mut self, ms: u32) {
        Ets::delay_ms(ms);
        self.guard.get_or_insert_with(|| self.critical.enter());
    }
}
