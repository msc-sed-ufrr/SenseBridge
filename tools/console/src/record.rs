//! Classificação das linhas da serial e interpretação das linhas de ciclo.

/// Colunas emitidas pelo firmware, usadas enquanto o cabeçalho não chega (console aberto
/// depois do boot). Vem do mesmo arquivo que o `HEADER` do firmware.
pub const DEFAULT_HEADER: &str = include!("../../../firmware/esp32/src/csv_header.in");

/// O que uma linha da serial representa.
#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    /// Cabeçalho do CSV, emitido a cada boot.
    Header,
    /// Candidata a linha de ciclo: começa por um número.
    Data,
    /// Log de aviso do ESP-IDF (`W (`).
    Warning,
    /// Log de erro do ESP-IDF (`E (`), pânico ou exceção.
    Error,
    /// Banner da ROM após um reset do chip.
    Reset,
    /// Qualquer outra linha: logs informativos, boot, mensagens do `espflash`.
    Other,
}

pub fn classify(line: &str) -> Kind {
    if line.starts_with("cycle,") {
        Kind::Header
    } else if line.starts_with(|c: char| c.is_ascii_digit()) {
        Kind::Data
    } else if line.starts_with("W (") {
        Kind::Warning
    } else if line.starts_with("E (")
        || line.contains("Guru Meditation")
        || line.contains("panicked")
    {
        Kind::Error
    } else if line.starts_with("rst:") {
        Kind::Reset
    } else {
        Kind::Other
    }
}

/// Nomes das colunas, na ordem do cabeçalho.
#[derive(Debug, Clone)]
pub struct Columns(Vec<String>);

impl Columns {
    pub fn parse(header: &str) -> Self {
        Self(header.split(',').map(str::to_owned).collect())
    }

    /// O cabeçalho, como o firmware o emite.
    pub fn header(&self) -> String {
        self.0.join(",")
    }

    /// Interpreta uma linha de ciclo. `None` quando o número de campos não bate com o
    /// cabeçalho ou uma coluna numérica não é um número: a linha não é um ciclo íntegro.
    pub fn cycle(&self, line: &str) -> Option<Cycle> {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != self.0.len() {
            return None;
        }
        let get = |name: &str| {
            self.0
                .iter()
                .position(|column| column == name)
                .map_or("", |index| fields[index])
        };
        let failure = |name: &str| Some(get(name).to_owned()).filter(|text| !text.is_empty());

        let local = match failure("local_failure") {
            Some(failure) => Err(failure),
            None => Ok(Reading {
                temperature: get("local_temperature_c").parse().ok()?,
                humidity: get("local_humidity_pct").parse().ok()?,
            }),
        };
        let external = match failure("external_failure") {
            Some(failure) => Err(failure),
            None => Ok(External {
                reading: Reading {
                    temperature: get("external_temperature_c").parse().ok()?,
                    humidity: get("external_humidity_pct").parse().ok()?,
                },
                age_ms: get("external_age_ms").parse().ok()?,
            }),
        };
        let transition = failure("transition").map(|text| Transition {
            text: text.replace("->", " → "),
            recovery: get("recovery") == "true",
        });

        Some(Cycle {
            cycle: get("cycle").parse().ok()?,
            timestamp_ms: get("timestamp_ms").parse().ok()?,
            state: get("state").to_owned(),
            local,
            external,
            transition,
            fetch_us: optional(get("fetch_us"))?,
            cycle_us: get("cycle_us").parse().ok()?,
            heap_free: get("heap_free").parse().ok()?,
            heap_min_free: get("heap_min_free").parse().ok()?,
            stack_min_free: get("stack_min_free").parse().ok()?,
        })
    }
}

impl Default for Columns {
    fn default() -> Self {
        Self::parse(DEFAULT_HEADER)
    }
}

/// Campo numérico que pode vir vazio. `None` externo: o campo não é um número.
fn optional(text: &str) -> Option<Option<u64>> {
    if text.is_empty() {
        Some(None)
    } else {
        text.parse().ok().map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct External {
    pub reading: Reading,
    pub age_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// `Normal → DegradedLocal`.
    pub text: String,
    pub recovery: bool,
}

/// Um ciclo do firmware, com as falhas mantidas como o texto emitido (`NoResponse`, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct Cycle {
    pub cycle: u64,
    pub timestamp_ms: u64,
    pub state: String,
    pub local: Result<Reading, String>,
    pub external: Result<External, String>,
    pub transition: Option<Transition>,
    /// Duração da consulta externa; vazia quando a leitura anterior foi reutilizada.
    pub fetch_us: Option<u64>,
    pub cycle_us: u64,
    pub heap_free: u64,
    pub heap_min_free: u64,
    pub stack_min_free: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Linhas reais do ESP32, capturadas com o DHT11 no GPIO4.
    const NORMAL_WITH_FETCH: &str =
        "0,21,Normal,27.0,41.0,,29.1,76.0,0,,,,21911,1460235,1482282,206832,201476,12436";
    const LOCAL_FAILURE: &str = "1,5005,DegradedLocal,,,NoResponse,29.1,76.0,4984,,\
        Normal->DegradedLocal,false,22132,,22254,206948,201476,12436";
    const RECOVERY: &str = "3,15005,Normal,27.0,39.0,,29.1,76.0,14984,,\
        DegradedLocal->Normal,true,1577150,,1577345,206884,199164,11876";

    #[test]
    fn classifies_serial_lines() {
        assert_eq!(classify(DEFAULT_HEADER), Kind::Header);
        assert_eq!(classify(NORMAL_WITH_FETCH), Kind::Data);
        assert_eq!(
            classify("W (3302) sense_bridge_esp32: Wi-Fi desconectado (motivo 2)"),
            Kind::Warning
        );
        assert_eq!(
            classify("E (100) esp_http_client: Connection failed"),
            Kind::Error
        );
        assert_eq!(
            classify("rst:0x1 (POWERON_RESET),boot:0x17 (SPI_FAST_FLASH_BOOT)"),
            Kind::Reset
        );
        assert_eq!(
            classify("I (642) sense_bridge_esp32: DHT11 no GPIO4"),
            Kind::Other
        );
        assert_eq!(
            classify("[2026-09-24T01:15:05Z INFO ] Connecting..."),
            Kind::Other
        );
    }

    #[test]
    fn parses_normal_cycle_with_fetch() {
        let cycle = Columns::default().cycle(NORMAL_WITH_FETCH).unwrap();
        assert_eq!(cycle.cycle, 0);
        assert_eq!(cycle.state, "Normal");
        assert_eq!(
            cycle.local,
            Ok(Reading {
                temperature: 27.0,
                humidity: 41.0
            })
        );
        assert_eq!(
            cycle.external,
            Ok(External {
                reading: Reading {
                    temperature: 29.1,
                    humidity: 76.0
                },
                age_ms: 0
            })
        );
        assert_eq!(cycle.transition, None);
        assert_eq!(cycle.fetch_us, Some(1_460_235));
        assert_eq!(cycle.cycle_us, 1_482_282);
        assert_eq!(cycle.stack_min_free, 12_436);
    }

    #[test]
    fn parses_local_failure_and_transition() {
        let cycle = Columns::default().cycle(LOCAL_FAILURE).unwrap();
        assert_eq!(cycle.state, "DegradedLocal");
        assert_eq!(cycle.local, Err("NoResponse".to_owned()));
        assert_eq!(cycle.fetch_us, None);
        assert_eq!(
            cycle.transition,
            Some(Transition {
                text: "Normal → DegradedLocal".to_owned(),
                recovery: false
            })
        );
    }

    #[test]
    fn parses_recovery() {
        let cycle = Columns::default().cycle(RECOVERY).unwrap();
        assert!(cycle.transition.unwrap().recovery);
    }

    #[test]
    fn parses_external_failure() {
        let line = "40,200005,DegradedExternal,26.0,38.0,,,,,NoConnectivity,\
            Normal->DegradedExternal,false,22010,1200,23300,206000,199000,11800";
        let cycle = Columns::default().cycle(line).unwrap();
        assert_eq!(cycle.external, Err("NoConnectivity".to_owned()));
        assert_eq!(cycle.fetch_us, Some(1200));
    }

    #[test]
    fn follows_the_header_column_order() {
        let columns =
            Columns::parse(&DEFAULT_HEADER.replace("cycle,timestamp_ms", "timestamp_ms,cycle"));
        let line = NORMAL_WITH_FETCH.replacen("0,21", "21,0", 1);
        let cycle = columns.cycle(&line).unwrap();
        assert_eq!((cycle.cycle, cycle.timestamp_ms), (0, 21));
    }

    #[test]
    fn rejects_incomplete_or_garbled_rows() {
        let columns = Columns::default();
        assert_eq!(columns.cycle("0,21,Normal"), None);
        assert_eq!(
            columns.cycle(&NORMAL_WITH_FETCH.replace("1482282", "14x2282")),
            None
        );
    }
}
