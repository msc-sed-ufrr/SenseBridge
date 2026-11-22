//! Uma linha CSV por ciclo, emitida pela serial depois do ciclo, fora da janela medida.
//!
//! Estados e falhas saem pelos nomes estáveis da biblioteca (`as_str`), que o console do host
//! reconhece.

use std::fmt::Write;
use std::time::Duration;

use esp_idf_svc::sys;
use sense_bridge::{FreezeDetector, Record};

use crate::local::Restart;

pub const HEADER: &str = include!("csv_header.in");

/// Duração das operações de um ciclo. `read` e `fetch` ficam vazias quando a fonte não foi
/// consultada; a validação e a integração correspondem a `cycle - read - fetch`.
pub struct Timing {
    pub read: Option<Duration>,
    pub fetch: Option<Duration>,
    pub cycle: Duration,
}

/// `freeze` já observou `record`. Sem ele (`local_frozen_after = 0`), `local_repeat` e
/// `local_suspect` ficam vazias. `restart` é o reinício do sensor feito depois do ciclo
/// anterior, se houve.
pub fn row(
    record: &Record,
    timing: &Timing,
    freeze: Option<&FreezeDetector>,
    restart: Option<Restart>,
) -> String {
    let mut line = String::with_capacity(208);
    // Escrever em uma String não falha.
    let _ = write_row(&mut line, record, timing, freeze, restart);
    line
}

fn write_row(
    line: &mut String,
    record: &Record,
    timing: &Timing,
    freeze: Option<&FreezeDetector>,
    restart: Option<Restart>,
) -> std::fmt::Result {
    write!(
        line,
        "{},{},{},",
        record.cycle,
        record.timestamp_ms,
        record.state().as_str()
    )?;
    match record.local {
        Ok(reading) => write!(line, "{:.1},{:.1},,", reading.temperature, reading.humidity)?,
        Err(failure) => write!(line, ",,{},", failure.as_str())?,
    }
    match record.external {
        Ok(sample) => write!(
            line,
            "{:.1},{:.1},{},,",
            sample.reading.temperature, sample.reading.humidity, sample.age_ms
        )?,
        Err(failure) => write!(line, ",,,{},", failure.as_str())?,
    }
    match record.transition {
        Some(transition) => write!(
            line,
            "{}->{},{},",
            transition.from.as_str(),
            transition.to.as_str(),
            transition.is_recovery()
        )?,
        None => line.push_str(",,"),
    }
    write!(
        line,
        "{},{},{},",
        micros(timing.read),
        micros(timing.fetch),
        timing.cycle.as_micros()
    )?;

    // Memória livre agora e o mínimo desde o boot; folga mínima da pilha da tarefa principal.
    let (heap_free, heap_min_free, stack_min_free) = unsafe {
        (
            sys::esp_get_free_heap_size(),
            sys::esp_get_minimum_free_heap_size(),
            sys::uxTaskGetStackHighWaterMark(core::ptr::null_mut()),
        )
    };
    write!(line, "{heap_free},{heap_min_free},{stack_min_free},")?;

    match freeze {
        Some(freeze) => write!(line, "{},{},", freeze.run(), freeze.is_suspect())?,
        None => line.push_str(",,"),
    }
    line.push_str(restart.map_or("", Restart::as_str));
    Ok(())
}

fn micros(duration: Option<Duration>) -> String {
    duration.map_or_else(String::new, |duration| duration.as_micros().to_string())
}
