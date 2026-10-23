//! Fonte local: sensor DHT11 lido por um pino GPIO, com a crate `dht11-trace`.

use std::thread;
use std::time::{Duration, Instant};

use dht11_trace::{Fault, Platform};
use embedded_hal::digital::{ErrorType, OutputPin};
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{InputOutput, Output, PinDriver};
use esp_idf_svc::hal::interrupt::IsrCriticalSection;
use esp_idf_svc::sys::{self, EspError};
use log::warn;
use sense_bridge::{LocalFailure, LocalReading, LocalSource};

/// O DHT11 precisa de 1 s depois de energizado antes da primeira leitura.
const POWER_UP: Duration = Duration::from_secs(1);

/// Falhas seguidas que levam a desligar e religar o sensor, quando ele é alimentado por um
/// GPIO.
const POWER_CYCLE_AFTER: u32 = 3;

type Data<'d> = PinDriver<'d, InputOutput>;

/// Erro dos pinos do `esp-idf-hal` pelo `embedded-hal`.
type PinError = <Data<'static> as ErrorType>::Error;

/// DHT11 ligado a um pino em dreno aberto, com pull-up, e opcionalmente alimentado por um
/// GPIO.
///
/// A leitura distingue a etapa do protocolo em que falhou ([`Fault`]). A biblioteca recebe
/// apenas `NoResponse` ou `Checksum`; a etapa vai para o log em [`Dht11::after_cycle`], fora
/// da janela medida. Quem chama `read` respeita o intervalo mínimo de amostragem.
pub struct Dht11<'d> {
    driver: dht11_trace::Dht11<Data<'d>, Ets, Esp32, Power<'d>>,
    /// Há um GPIO alimentando o sensor, e o reinício vale a pena.
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
        // A linha fica solta, em nível alto, até a primeira leitura: o sensor interpreta um
        // nível baixo como pedido de leitura.
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

    /// Aguarda o sensor ficar pronto depois de energizado. Com VCC no 3V3, conta a partir de
    /// [`Dht11::new`], depois do boot.
    pub fn wait_power_up(&self) {
        if let Some(wait) = POWER_UP.checked_sub(self.powered_at.elapsed()) {
            thread::sleep(wait);
        }
    }

    /// Chamado depois de cada ciclo, fora da janela medida: registra a etapa em que a última
    /// leitura falhou e, a cada `POWER_CYCLE_AFTER` falhas seguidas, reinicia o sensor.
    pub fn after_cycle(&mut self) {
        let Some(fault) = self.last_fault.take() else {
            return;
        };
        let failures = self.driver.failures();
        warn!("DHT11: {} (falha {failures} seguida)", describe(&fault));
        if self.switchable && failures >= POWER_CYCLE_AFTER {
            // Um DHT11 travado só volta a responder depois de perder a alimentação. A crate
            // mantém o DATA em nível baixo enquanto o sensor está desligado e espera o tempo de
            // energização antes de devolver.
            warn!("DHT11: {failures} falhas seguidas; desligando o sensor por 1000 ms");
            if let Err(error) = self.driver.power_cycle() {
                warn!("DHT11: erro no GPIO ao reiniciar o sensor: {error}");
            }
        }
    }
}

impl LocalSource for Dht11<'_> {
    fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        match self.driver.read() {
            // Partes inteiras, com o bit 7 da temperatura como sinal: a conversão da crate
            // `dht-sensor` 0.3, mantida para que as leituras sigam comparáveis.
            Ok(reading) => {
                self.last_fault = None;
                Ok(LocalReading::new(
                    f32::from(reading.temperature()),
                    f32::from(reading.humidity()),
                ))
            }
            Err(fault) => {
                // Só o checksum inválido não é `NoResponse`: a etapa fica no log, e
                // `local_failure` continua comparável às capturas anteriores.
                let failure = match fault {
                    Fault::Checksum { .. } => LocalFailure::Checksum,
                    _ => LocalFailure::NoResponse,
                };
                self.last_fault = Some(fault);
                Err(failure)
            }
        }
    }
}

/// Etapa em que a leitura falhou, nos termos que as capturas já registram.
fn describe(fault: &Fault<PinError>) -> String {
    match fault {
        Fault::LineLow => {
            "linha DATA presa em nível baixo (DATA em curto com o GND ou sensor travado)".into()
        }
        Fault::NoAck => "sem resposta ao pulso de início \
                         (sensor sem alimentação, DATA desconectado ou sensor travado)"
            .into(),
        Fault::AckBroken => "resposta ao pulso de início incompleta".into(),
        Fault::Truncated { bits } => format!("quadro interrompido após {bits} de 40 bits"),
        Fault::Checksum { frame } => format!("checksum inválido no quadro {frame:02x?}"),
        Fault::ZeroFrame => {
            "quadro com os 40 bits em 0 (pulsos medidos curtos demais: confira o contador)".into()
        }
        Fault::Pin(error) => format!("erro no GPIO: {error}"),
        // Etapas que versões futuras da crate venham a distinguir.
        other => format!("{other}"),
    }
}

/// Contador de ciclos da CPU, que segue contando com as interrupções mascaradas, e uma seção
/// crítica de ISR, que tira a tarefa do Wi-Fi e as interrupções deste núcleo durante o quadro.
/// O quadro dura cerca de 4 ms, com bits distinguidos por pulsos de 26 µs e 70 µs; uma
/// preempção pela tarefa do Wi-Fi pode passar de 1 ms e corromper a leitura.
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

/// GPIO que alimenta o sensor, quando `dht11_power_gpio` está configurado. Sem ele, ligar e
/// desligar não fazem nada, e [`Dht11::after_cycle`] nunca pede o reinício.
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
