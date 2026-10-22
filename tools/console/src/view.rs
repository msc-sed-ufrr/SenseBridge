//! Formatação dos ciclos para o terminal.

use sense_bridge::OperationState;

use crate::record::{Cycle, Reading};

/// Cores ANSI, desligadas fora de um terminal ou com `NO_COLOR`.
#[derive(Clone, Copy)]
pub struct Style {
    pub color: bool,
}

impl Style {
    fn paint(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }

    pub fn bold(self, text: &str) -> String {
        self.paint("1", text)
    }

    pub fn dim(self, text: &str) -> String {
        self.paint("2", text)
    }

    pub fn red(self, text: &str) -> String {
        self.paint("31", text)
    }

    pub fn green(self, text: &str) -> String {
        self.paint("32", text)
    }

    pub fn yellow(self, text: &str) -> String {
        self.paint("33", text)
    }

    pub fn cyan(self, text: &str) -> String {
        self.paint("36", text)
    }

    /// Cor de um estado da biblioteca, reconhecido pelo nome que o firmware emite.
    pub fn state(self, state: &str, text: &str) -> String {
        const NORMAL: &str = OperationState::Normal.as_str();
        const ERROR: &str = OperationState::Error.as_str();
        match state {
            NORMAL => self.green(text),
            ERROR => self.red(text),
            _ => self.yellow(text),
        }
    }
}

pub fn header(style: Style) -> String {
    style.bold(&format!(
        "{:>5}  {:<8}  {:<16}  {:<12}  {:<21}  {:>7}",
        "#", "tempo", "estado", "local", "externo", "ciclo"
    ))
}

/// Uma linha por ciclo e, quando houve transição, uma segunda linha com ela.
pub fn cycle(cycle: &Cycle, style: Style) -> String {
    // Cada coluna é alinhada antes de receber cor, para que os escapes não contem na largura.
    let local = match &cycle.local {
        Ok(reading) => format!("{:<12}", reading_text(reading)),
        Err(failure) => style.red(&format!("{failure:<12}")),
    };
    let fetch = cycle.fetch_us.map(|us| format!("↻ {}", duration(us)));
    let external = match &cycle.external {
        Ok(external) => {
            let detail = fetch.unwrap_or_else(|| format!("({})", age(external.age_ms)));
            format!(
                "{:<21}",
                format!("{:<11} {detail}", reading_text(&external.reading))
            )
        }
        Err(failure) => {
            let text = match fetch {
                Some(fetch) => format!("{:<21}", format!("{failure} {fetch}")),
                None => format!("{failure:<21}"),
            };
            // Só o nome da falha, em ASCII, fica em vermelho.
            format!("{}{}", style.red(failure), &text[failure.len()..])
        }
    };
    let mut line = format!(
        "{:>5}  {}  {}  {local}  {external}  {:>7}",
        cycle.cycle,
        clock(cycle.timestamp_ms),
        style.state(&cycle.state, &format!("{:<16}", cycle.state)),
        duration(cycle.cycle_us),
    );
    if let Some(transition) = &cycle.transition {
        let recovery = if transition.recovery {
            " (recuperação)"
        } else {
            ""
        };
        line.push('\n');
        line.push_str(&style.state(
            &cycle.state,
            &format!("{:>5}  ↳ {}{recovery}", "", transition.text),
        ));
    }
    line
}

fn reading_text(reading: &Reading) -> String {
    format!("{:.1}°C {:.0}%", reading.temperature, reading.humidity)
}

/// `HH:MM:SS` desde o primeiro ciclo.
pub fn clock(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// Duração curta: `22 ms` ou `1.35 s`.
pub fn duration(us: u64) -> String {
    if us < 1_000_000 {
        format!("{} ms", us / 1000)
    } else {
        format!("{:.2} s", us as f64 / 1e6)
    }
}

/// Idade de uma leitura: `12 s`, `3 min` ou `2 h`.
pub fn age(ms: u64) -> String {
    let seconds = ms / 1000;
    if seconds < 60 {
        format!("{seconds} s")
    } else if seconds < 3600 {
        format!("{} min", seconds / 60)
    } else {
        format!("{} h", seconds / 3600)
    }
}

/// Bytes em KB, com uma casa.
pub fn kilobytes(bytes: u64) -> String {
    format!("{:.1} KB", bytes as f64 / 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Columns;

    const PLAIN: Style = Style { color: false };

    #[test]
    fn formats_times_and_sizes() {
        assert_eq!(clock(3_725_000), "01:02:05");
        assert_eq!(duration(22_254), "22 ms");
        assert_eq!(duration(1_482_282), "1.48 s");
        assert_eq!(age(4_984), "4 s");
        assert_eq!(age(139_984), "2 min");
        assert_eq!(age(7_200_000), "2 h");
        assert_eq!(kilobytes(206_848), "202.0 KB");
    }

    #[test]
    fn formats_a_cycle_with_its_transition() {
        let row = "13,65005,DegradedLocal,,,NoResponse,29.1,76.0,64984,,\
            Normal->DegradedLocal,false,22132,,22254,206948,201476,12436";
        let cycle = Columns::default().cycle(row).unwrap();
        assert_eq!(
            super::cycle(&cycle, PLAIN),
            "   13  00:01:05  DegradedLocal     NoResponse    29.1°C 76%  (1 min)      22 ms\n\
             \x20      ↳ Normal → DegradedLocal"
        );
    }

    #[test]
    fn marks_a_fetch_in_the_cycle() {
        let row = "0,21,Normal,27.0,41.0,,29.1,76.0,0,,,,21911,1460235,1482282,206832,201476,12436";
        let cycle = Columns::default().cycle(row).unwrap();
        assert!(super::cycle(&cycle, PLAIN).contains("29.1°C 76%  ↻ 1.46 s"));
    }
}
