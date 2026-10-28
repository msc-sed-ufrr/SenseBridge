use std::thread;
use std::time::{Duration, Instant};

use dht11_trace::{Fault, Platform};
use embedded_hal::digital::{ErrorType, OutputPin};
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{InputOutput, Output, PinDriver};
use esp_idf_svc::hal::interrupt::IsrCriticalSection;
use esp_idf_svc::sys::{self, EspError};
use log::warn;

const POWER_UP: Duration = Duration::from_secs(1);
const POWER_CYCLE_AFTER: u32 = 3;

type Data<'d> = PinDriver<'d, InputOutput>;
type PinError = <Data<'static> as ErrorType>::Error;

#[derive(Debug, Clone, Copy)]
pub struct LocalReading {
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum LocalFailure {
    NoResponse,
    Checksum,
}

pub struct Dht11<'d> {
    driver: dht11_trace::Dht11<Data<'d>, Ets, Esp32, Power<'d>>,
    switchable: bool,
    powered_at: Instant,
    last_fault: Option<Fault<PinError>>,
}

impl<'d> Dht11<'d> {
    pub fn new(data: Data<'d>, power: Option<PinDriver<'d, Output>>) -> Result<Self, EspError> {
        let switchable = power.is_some();
        let platform = Esp32 {
            section: IsrCriticalSection::new(),
        };

        let driver = dht11_trace::Dht11::new(data, Ets, platform)
            .and_then(|driver| driver.with_power(Power(power)))
            .map_err(|error| error.cause())?;

        Ok(Self {
            driver,
            switchable,
            powered_at: Instant::now(),
            last_fault: None,
        })
    }

    pub fn wait_power_up(&self) {
        if let Some(wait) = POWER_UP.checked_sub(self.powered_at.elapsed()) {
            thread::sleep(wait);
        }
    }

    pub fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        match self.driver.read() {
            Ok(reading) => {
                self.last_fault = None;
                Ok(LocalReading {
                    temperature: f32::from(reading.temperature()),
                    humidity: f32::from(reading.humidity()),
                })
            }
            Err(fault) => {
                let failure = match fault {
                    Fault::Checksum { .. } => LocalFailure::Checksum,
                    _ => LocalFailure::NoResponse,
                };
                self.last_fault = Some(fault);
                Err(failure)
            }
        }
    }

    pub fn after_cycle(&mut self) {
        let Some(fault) = self.last_fault.take() else {
            return;
        };

        let failures = self.driver.failures();
        warn!("DHT11: {} (falha {failures} seguida)", describe(&fault));

        if self.switchable && failures >= POWER_CYCLE_AFTER {
            warn!("DHT11: {failures} falhas seguidas; reiniciando sensor");
            if let Err(error) = self.driver.power_cycle() {
                warn!("DHT11: erro no GPIO ao reiniciar o sensor: {error}");
            }
        }
    }
}

fn describe(fault: &Fault<PinError>) -> String {
    match fault {
        Fault::LineLow => "linha DATA presa em nivel baixo".into(),
        Fault::NoAck => "sem resposta ao pulso de inicio".into(),
        Fault::AckBroken => "resposta ao pulso de inicio incompleta".into(),
        Fault::Truncated { bits } => format!("quadro interrompido apos {bits} de 40 bits"),
        Fault::Checksum { frame } => format!("checksum invalido no quadro {frame:02x?}"),
        Fault::ZeroFrame => "quadro com os 40 bits em 0".into(),
        Fault::Pin(error) => format!("erro no GPIO: {error}"),
        other => format!("{other}"),
    }
}

struct Esp32 {
    section: IsrCriticalSection,
}

impl Platform for Esp32 {
    fn ticks(&self) -> u32 {
        unsafe { sys::xthal_get_ccount() }
    }

    fn ticks_per_us(&self) -> u32 {
        unsafe { sys::esp_rom_get_cpu_ticks_per_us() }
    }

    fn uninterrupted<R>(&self, read: impl FnOnce() -> R) -> R {
        let _guard = self.section.enter();
        read()
    }
}

struct Power<'d>(Option<PinDriver<'d, Output>>);

impl ErrorType for Power<'_> {
    type Error = PinError;
}

impl OutputPin for Power<'_> {
    fn set_low(&mut self) -> Result<(), PinError> {
        self.0.as_mut().map_or(Ok(()), OutputPin::set_low)
    }

    fn set_high(&mut self) -> Result<(), PinError> {
        self.0.as_mut().map_or(Ok(()), OutputPin::set_high)
    }
}
