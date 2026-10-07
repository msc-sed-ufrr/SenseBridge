//! Fonte local: sensor DHT11 lido por um pino GPIO.

use std::fmt;
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{InputOutput, Level, Output, PinDriver};
use esp_idf_svc::hal::interrupt::IsrCriticalSection;
use esp_idf_svc::sys::{self, EspError};
use log::warn;
use sense_bridge::{LocalFailure, LocalReading, LocalSource};

/// O DHT11 precisa de 1 s depois de energizado antes da primeira leitura.
const POWER_UP: Duration = Duration::from_secs(1);

/// Pulso de início: o manual pede a linha em nível baixo por pelo menos 18 ms.
const START_PULSE_MS: u32 = 18;

/// Espera máxima por cada borda. As fases do protocolo duram de 20 µs a 80 µs.
const EDGE_TIMEOUT_US: u32 = 100;

/// Pulsos em nível alto acima deste limite são bits 1 (70 µs); abaixo, bits 0 (26–28 µs).
const ONE_THRESHOLD_US: u32 = 48;

/// Falhas seguidas que levam a desligar e religar o sensor, quando ele é alimentado por um
/// GPIO.
const POWER_CYCLE_AFTER: u32 = 3;

/// Tempo com o sensor desligado em cada reinício.
const POWER_OFF: Duration = Duration::from_secs(1);

/// DHT11 ligado a um pino em dreno aberto, com pull-up, e opcionalmente alimentado por um
/// GPIO.
///
/// A leitura distingue a etapa do protocolo em que falhou ([`Fault`]). A biblioteca recebe
/// apenas `NoResponse` ou `Checksum`; a etapa vai para o log em [`Dht11::after_cycle`], fora
/// da janela medida. Quem chama `read` respeita o intervalo mínimo de amostragem.
pub struct Dht11<'d> {
    data: PinDriver<'d, InputOutput>,
    power: Option<PinDriver<'d, Output>>,
    powered_at: Instant,
    /// Leituras seguidas sem sucesso.
    failures: u32,
    last_fault: Option<Fault>,
}

impl<'d> Dht11<'d> {
    pub fn new(
        mut data: PinDriver<'d, InputOutput>,
        mut power: Option<PinDriver<'d, Output>>,
    ) -> Result<Self, EspError> {
        // A linha fica solta, em nível alto, até a primeira leitura: o sensor interpreta um
        // nível baixo como pedido de leitura.
        data.set_high()?;
        if let Some(power) = power.as_mut() {
            power.set_high()?;
        }
        Ok(Self { data, power, powered_at: Instant::now(), failures: 0, last_fault: None })
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
        warn!("DHT11: {fault} (falha {} seguida)", self.failures);
        if self.power.is_some() && self.failures.is_multiple_of(POWER_CYCLE_AFTER) {
            self.power_cycle();
        }
    }

    /// Desliga o sensor por `POWER_OFF` e o religa: um DHT11 travado só volta a responder
    /// depois de perder a alimentação.
    fn power_cycle(&mut self) {
        let Some(power) = self.power.as_mut() else {
            return;
        };
        warn!(
            "DHT11: {} falhas seguidas; desligando o sensor por {} ms",
            self.failures,
            POWER_OFF.as_millis()
        );
        // DATA também em nível baixo: pelo pull-up, o sensor continuaria alimentado através do
        // pino de dados.
        let off = self.data.set_low().and(power.set_low());
        thread::sleep(POWER_OFF);
        let on = power.set_high().and(self.data.set_high());
        self.powered_at = Instant::now();
        if let Err(error) = off.and(on) {
            warn!("DHT11: erro no GPIO ao reiniciar o sensor: {error}");
        }
        self.wait_power_up();
    }

    /// Lê o quadro de 40 bits: umidade, decimal da umidade, temperatura, decimal da
    /// temperatura e checksum.
    fn read_frame(&mut self) -> Result<[u8; 4], Fault> {
        if self.data.is_low() {
            return Err(Fault::LineLow);
        }
        self.data.set_low().map_err(Fault::Pin)?;
        // Espera ativa: o escalonador tem resolução de 10 ms.
        Ets::delay_ms(START_PULSE_MS);

        // O quadro dura cerca de 4 ms, com bits distinguidos por pulsos de 26 µs e 70 µs. Uma
        // preempção pela tarefa do Wi-Fi, no mesmo núcleo, pode passar de 1 ms e corromper a
        // leitura; por isso as interrupções ficam mascaradas da liberação da linha até o
        // último bit. O pulso de início fica fora da seção crítica.
        let critical = IsrCriticalSection::new();
        let _guard = critical.enter();
        self.data.set_high().map_err(Fault::Pin)?;
        let line = Line::new(&self.data);

        // Resposta: 20–40 µs depois da liberação, o sensor mantém a linha em nível baixo por
        // 80 µs e em nível alto por mais 80 µs. A espera pelo nível alto cobre a subida pelo
        // pull-up, que não pode ser confundida com a resposta.
        line.wait_for(Level::High).ok_or(Fault::LineLow)?;
        line.wait_for(Level::Low).ok_or(Fault::NoAck)?;
        line.wait_for(Level::High).ok_or(Fault::AckBroken)?;
        line.wait_for(Level::Low).ok_or(Fault::AckBroken)?;

        // Cada bit: 50 µs em nível baixo e um pulso em nível alto cuja duração dá o valor.
        let mut frame = [0u8; 5];
        for bit in 0..40u8 {
            let truncated = Fault::Truncated { bits: bit };
            let rise = line.wait_for(Level::High).ok_or(truncated)?;
            let fall = line.wait_for(Level::Low).ok_or(truncated)?;
            let byte = &mut frame[usize::from(bit / 8)];
            *byte = *byte << 1 | u8::from(line.micros(rise, fall) > ONE_THRESHOLD_US);
        }

        let [data @ .., checksum] = frame;
        if data.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) == checksum {
            Ok(data)
        } else {
            Err(Fault::Checksum { frame })
        }
    }
}

impl LocalSource for Dht11<'_> {
    fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        let result = self.read_frame();
        self.last_fault = result.err();
        match result {
            // Como na `dht-sensor` 0.3, usada até aqui: partes inteiras, com o bit 7 da
            // temperatura como sinal.
            Ok([humidity, _, temperature, _]) => {
                self.failures = 0;
                let magnitude = f32::from(temperature & 0x7F);
                let sign = if temperature & 0x80 == 0 { 1.0 } else { -1.0 };
                Ok(LocalReading::new(sign * magnitude, f32::from(humidity)))
            }
            Err(fault) => {
                self.failures += 1;
                Err(fault.into())
            }
        }
    }
}

/// Etapa em que a leitura falhou, mais fina que [`LocalFailure`].
#[derive(Debug, Clone, Copy)]
pub enum Fault {
    /// A linha ficou em nível baixo sem o pulso de início, antes dele ou depois da liberação.
    LineLow,
    /// O sensor não respondeu ao pulso de início.
    NoAck,
    /// A resposta começou, mas não seguiu os 80 µs + 80 µs do manual.
    AckBroken,
    /// O quadro parou depois de `bits` dos 40 bits.
    Truncated { bits: u8 },
    /// O quadro chegou completo, mas a soma não confere.
    Checksum { frame: [u8; 5] },
    /// O driver do GPIO falhou.
    Pin(EspError),
}

impl From<Fault> for LocalFailure {
    /// O mesmo mapeamento de antes do diagnóstico por etapa: só o checksum inválido não é
    /// `NoResponse`.
    fn from(fault: Fault) -> Self {
        match fault {
            Fault::Checksum { .. } => LocalFailure::Checksum,
            _ => LocalFailure::NoResponse,
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::LineLow => f.write_str(
                "linha DATA presa em nível baixo (DATA em curto com o GND ou sensor travado)",
            ),
            Fault::NoAck => f.write_str(
                "sem resposta ao pulso de início \
                 (sensor sem alimentação, DATA desconectado ou sensor travado)",
            ),
            Fault::AckBroken => f.write_str("resposta ao pulso de início incompleta"),
            Fault::Truncated { bits } => write!(f, "quadro interrompido após {bits} de 40 bits"),
            Fault::Checksum { frame } => write!(f, "checksum inválido no quadro {frame:02x?}"),
            Fault::Pin(error) => write!(f, "erro no GPIO: {error}"),
        }
    }
}

/// Linha de dados medida pelo contador de ciclos da CPU, que segue contando com as
/// interrupções mascaradas.
struct Line<'a, 'd> {
    pin: &'a PinDriver<'d, InputOutput>,
    ticks_per_us: u32,
}

impl<'a, 'd> Line<'a, 'd> {
    fn new(pin: &'a PinDriver<'d, InputOutput>) -> Self {
        Self { pin, ticks_per_us: unsafe { sys::esp_rom_get_cpu_ticks_per_us() } }
    }

    /// Espera a linha chegar a `level` e devolve o instante, em ciclos da CPU, ou `None`
    /// depois de `EDGE_TIMEOUT_US`.
    fn wait_for(&self, level: Level) -> Option<u32> {
        let start = ticks();
        loop {
            let now = ticks();
            if self.pin.get_level() == level {
                return Some(now);
            }
            if now.wrapping_sub(start) > EDGE_TIMEOUT_US * self.ticks_per_us {
                return None;
            }
        }
    }

    /// Microssegundos entre dois instantes. O contador dá a volta a cada 18 s ou mais, muito
    /// além de um quadro.
    fn micros(&self, from: u32, to: u32) -> u32 {
        to.wrapping_sub(from) / self.ticks_per_us
    }
}

fn ticks() -> u32 {
    unsafe { sys::xthal_get_ccount() }
}
