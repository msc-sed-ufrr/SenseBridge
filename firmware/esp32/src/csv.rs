//! Uma linha CSV por ciclo, emitida pela serial depois do ciclo, fora da janela medida.

use std::fmt::Write;
use std::time::Duration;

use esp_idf_svc::sys;
use sense_bridge::Record;

pub const HEADER: &str = "cycle,timestamp_ms,state,\
    local_temperature_c,local_humidity_pct,local_failure,\
    external_temperature_c,external_humidity_pct,external_age_ms,external_failure,\
    transition,recovery,read_us,fetch_us,cycle_us,heap_free,heap_min_free,stack_min_free";

/// Duração das operações de um ciclo. `read` e `fetch` ficam vazias quando a fonte não foi
/// consultada; a validação e a integração correspondem a `cycle - read - fetch`.
pub struct Timing {
    pub read: Option<Duration>,
    pub fetch: Option<Duration>,
    pub cycle: Duration,
}

pub fn row(record: &Record, timing: &Timing) -> String {
    let mut line = String::with_capacity(192);
    // Escrever em uma String não falha.
    let _ = write_row(&mut line, record, timing);
    line
}

fn write_row(line: &mut String, record: &Record, timing: &Timing) -> std::fmt::Result {
    write!(line, "{},{},{:?},", record.cycle, record.timestamp_ms, record.state())?;
    match record.local {
        Ok(reading) => write!(line, "{:.1},{:.1},,", reading.temperature, reading.humidity)?,
        Err(failure) => write!(line, ",,{failure:?},")?,
    }
    match record.external {
        Ok(sample) => write!(
            line,
            "{:.1},{:.1},{},,",
            sample.reading.temperature, sample.reading.humidity, sample.age_ms
        )?,
        Err(failure) => write!(line, ",,,{failure:?},")?,
    }
    match record.transition {
        Some(transition) => write!(
            line,
            "{:?}->{:?},{},",
            transition.from,
            transition.to,
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
    write!(line, "{heap_free},{heap_min_free},{stack_min_free}")
}

fn micros(duration: Option<Duration>) -> String {
    duration.map_or_else(String::new, |duration| duration.as_micros().to_string())
}
