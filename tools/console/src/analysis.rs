//! Correspondência entre o estado esperado e o observado nas capturas de C1–C5 (PRD, UC-008).
//!
//! O CSV do firmware não traz o cenário nem a repetição: eles vêm do nome do arquivo,
//! `c<cenário>-r<repetição>.csv`, como em `docs/evidencias/experimentos/c2-r1.csv`.

use std::fmt::{self, Write as _};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sense_bridge::{OperationState, determine_state};

use crate::record::{self, Columns, Cycle, Kind};
use crate::view::Style;

/// Cenário do protocolo experimental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Scenario {
    C1,
    C2,
    C3,
    C4,
    C5,
}

impl Scenario {
    fn from_number(number: u8) -> Option<Self> {
        match number {
            1 => Some(Self::C1),
            2 => Some(Self::C2),
            3 => Some(Self::C3),
            4 => Some(Self::C4),
            5 => Some(Self::C5),
            _ => None,
        }
    }

    /// Estado de todos os ciclos, pela tabela de cenários do README. `None` em C5, em que a
    /// falha é desfeita durante a captura.
    fn fixed_state(self) -> Option<OperationState> {
        match self {
            Self::C1 => Some(OperationState::Normal),
            Self::C2 => Some(OperationState::DegradedLocal),
            Self::C3 => Some(OperationState::DegradedExternal),
            Self::C4 => Some(OperationState::Error),
            Self::C5 => None,
        }
    }

    fn expected(self, cycle: &Cycle) -> OperationState {
        // Em C5, o ciclo em que a fonte religada volta a responder depende do hardware: espera-se
        // o estado da disponibilidade observada, e a recuperação é conferida à parte.
        self.fixed_state()
            .unwrap_or_else(|| determine_state(cycle.local.is_ok(), cycle.external.is_ok()))
    }
}

/// Cenário e repetição pelo nome do arquivo: `c2-r1.csv` é a repetição 1 de C2.
fn identify(path: &Path) -> Option<(Scenario, u32)> {
    let stem = path.file_stem()?.to_str()?;
    let (scenario, repetition) = stem.strip_prefix('c')?.split_once("-r")?;
    Some((
        Scenario::from_number(scenario.parse().ok()?)?,
        repetition.parse().ok()?,
    ))
}

struct Capture {
    /// Nome do arquivo sem a extensão: `c2-r1`.
    name: String,
    scenario: Scenario,
    repetition: u32,
    cycles: Vec<Cycle>,
}

impl Capture {
    fn read(path: &Path) -> io::Result<Self> {
        let invalid = |message: &str| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: {message}", path.display()),
            )
        };
        let (scenario, repetition) = identify(path)
            .ok_or_else(|| invalid("o nome não segue o padrão c<cenário>-r<repetição>.csv"))?;
        let text = fs::read_to_string(path).map_err(|error| {
            io::Error::new(error.kind(), format!("{}: {error}", path.display()))
        })?;

        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.is_empty());
        let columns = match lines.next() {
            Some((_, header)) if record::classify(header) == Kind::Header => Columns::parse(header),
            _ => return Err(invalid("a primeira linha não é o cabeçalho do CSV")),
        };
        let cycles = lines
            .map(|(index, line)| {
                columns.cycle(line).ok_or_else(|| {
                    invalid(&format!("a linha {} não é um ciclo íntegro", index + 1))
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        if cycles.is_empty() {
            return Err(invalid("a captura não tem ciclos"));
        }

        Ok(Self {
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            scenario,
            repetition,
            cycles,
        })
    }

    fn check(&self) -> Check<'_> {
        let divergences = self
            .cycles
            .iter()
            .filter_map(|cycle| {
                let expected = self.scenario.expected(cycle);
                (cycle.state != expected.as_str()).then_some(Divergence {
                    cycle: cycle.cycle,
                    expected,
                    observed: &cycle.state,
                })
            })
            .collect();
        Check {
            divergences,
            recovery: (self.scenario == Scenario::C5).then(|| recovery(&self.cycles)),
        }
    }
}

/// Ciclo em que o estado observado difere do esperado.
struct Divergence<'a> {
    cycle: u64,
    expected: OperationState,
    observed: &'a str,
}

struct Check<'a> {
    divergences: Vec<Divergence<'a>>,
    /// Em C5, o ciclo da recuperação ou por que ela não vale.
    recovery: Option<Result<u64, &'static str>>,
}

impl Check<'_> {
    fn passed(&self) -> bool {
        self.divergences.is_empty() && !matches!(self.recovery, Some(Err(_)))
    }
}

fn both_valid(cycle: &Cycle) -> bool {
    cycle.local.is_ok() && cycle.external.is_ok()
}

/// Recuperação em C5 (PRD, §12): a captura começa com falha, e o primeiro ciclo com as duas
/// fontes válidas está em `Normal` e marcado como recuperação. Devolve o número desse ciclo.
fn recovery(cycles: &[Cycle]) -> Result<u64, &'static str> {
    if cycles.first().is_some_and(both_valid) {
        return Err("começa sem falha");
    }
    let cycle = cycles
        .iter()
        .find(|cycle| both_valid(cycle))
        .ok_or("as fontes não voltaram")?;
    let recovered = cycle.state == OperationState::Normal.as_str()
        && cycle
            .transition
            .as_ref()
            .is_some_and(|transition| transition.recovery);
    if recovered {
        Ok(cycle.cycle)
    } else {
        Err("o primeiro ciclo com as duas fontes válidas não é uma recuperação")
    }
}

/// Capturas aprovadas e ciclos no estado esperado.
#[derive(Default)]
struct Tally {
    captures: usize,
    passed: usize,
    cycles: usize,
    matched: usize,
}

impl Tally {
    fn add(&mut self, capture: &Capture, check: &Check) {
        self.captures += 1;
        self.passed += usize::from(check.passed());
        self.cycles += capture.cycles.len();
        self.matched += capture.cycles.len() - check.divergences.len();
    }
}

impl fmt::Display for Tally {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}/{} capturas aprovadas, {}/{} ciclos no estado esperado",
            self.passed, self.captures, self.matched, self.cycles
        )
    }
}

/// Lê as capturas e escreve a conferência de cada uma, de cada cenário e do conjunto.
/// Devolve `false` se alguma captura divergir do esperado.
pub fn run(files: &[PathBuf], out: &mut impl Write, style: Style) -> io::Result<bool> {
    let mut captures = files
        .iter()
        .map(|path| Capture::read(path))
        .collect::<io::Result<Vec<_>>>()?;
    captures.sort_by_key(|capture| (capture.scenario, capture.repetition));
    report(&captures, out, style)
}

fn report(captures: &[Capture], out: &mut impl Write, style: Style) -> io::Result<bool> {
    writeln!(
        out,
        "{}",
        style.bold(&format!(
            "{:<8}  {:<16}  {:>7}  resultado",
            "captura", "esperado", "ciclos"
        ))
    )?;
    let mut total = Tally::default();
    for group in captures.chunk_by(|a, b| a.scenario == b.scenario) {
        let mut tally = Tally::default();
        for capture in group {
            let check = capture.check();
            tally.add(capture, &check);
            total.add(capture, &check);
            writeln!(out, "{}", line(capture, &check, style))?;
        }
        writeln!(
            out,
            "{}",
            style.cyan(&format!("── {:?}: {tally}", group[0].scenario))
        )?;
    }
    writeln!(out, "{}", style.bold(&format!("── total: {total}")))?;
    Ok(total.passed == total.captures)
}

/// Uma linha por captura e, abaixo dela, uma por sequência de ciclos com a mesma divergência.
fn line(capture: &Capture, check: &Check, style: Style) -> String {
    // Cada coluna é alinhada antes de receber cor, para que os escapes não contem na largura.
    let expected = match capture.scenario.fixed_state() {
        Some(state) => style.state(state.as_str(), &format!("{:<16}", state.as_str())),
        None => format!("{:<16}", "recuperação"),
    };
    let total = capture.cycles.len();
    let matched = format!("{}/{total}", total - check.divergences.len());
    let verdict = if check.passed() {
        style.green("ok")
    } else {
        style.red("divergente")
    };
    let mut line = format!("{:<8}  {expected}  {matched:>7}  {verdict}", capture.name);
    // Escrever em uma String não falha.
    let _ = match check.recovery {
        Some(Ok(cycle)) => write!(line, ", recuperação no ciclo {cycle}"),
        Some(Err(reason)) => write!(line, ": {reason}"),
        None => Ok(()),
    };
    let same_run = |a: &Divergence, b: &Divergence| {
        b.cycle == a.cycle + 1 && b.expected == a.expected && b.observed == a.observed
    };
    for run in check.divergences.chunk_by(same_run) {
        let (first, last) = (&run[0], &run[run.len() - 1]);
        let cycles = if run.len() == 1 {
            format!("ciclo {}", first.cycle)
        } else {
            format!("ciclos {}–{}", first.cycle, last.cycle)
        };
        let _ = write!(
            line,
            "\n{:<8}  {cycles}: esperado {}, observado {}",
            "",
            first.expected.as_str(),
            first.observed
        );
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Style = Style { color: false };

    /// Ciclo com as falhas indicadas; uma falha vazia é uma leitura válida.
    fn cycle(number: u64, state: &str, local: &str, external: &str, transition: &str) -> Cycle {
        let local = match local {
            "" => "27.0,41.0,".to_owned(),
            failure => format!(",,{failure}"),
        };
        let external = match external {
            "" => "29.1,76.0,4984,".to_owned(),
            failure => format!(",,,{failure}"),
        };
        let recovery = match transition {
            "" => "",
            _ if transition.ends_with("->Normal") => "true",
            _ => "false",
        };
        let row = format!(
            "{number},{},{state},{local},{external},{transition},{recovery},\
             22000,,22254,206948,201476,12436",
            number * 5000
        );
        Columns::default().cycle(&row).unwrap()
    }

    fn capture(name: &str, cycles: Vec<Cycle>) -> Capture {
        let (scenario, repetition) = identify(Path::new(name)).unwrap();
        Capture {
            name: name.to_owned(),
            scenario,
            repetition,
            cycles,
        }
    }

    /// C5 com o DHT11 desligado até o ciclo 2 e respondendo de novo no ciclo `back`.
    fn recovery_capture(back: u64) -> Capture {
        let cycles = (0..6)
            .map(|number| match number {
                n if n < back => cycle(n, "DegradedLocal", "NoResponse", "", ""),
                n if n == back => cycle(n, "Normal", "", "", "DegradedLocal->Normal"),
                n => cycle(n, "Normal", "", "", ""),
            })
            .collect();
        capture("c5-r1", cycles)
    }

    #[test]
    fn identifies_scenario_and_repetition_by_file_name() {
        let id = |name: &str| identify(Path::new(name));
        assert_eq!(
            id("docs/evidencias/experimentos/c2-r1.csv"),
            Some((Scenario::C2, 1))
        );
        assert_eq!(id("c5-r10.csv"), Some((Scenario::C5, 10)));
        for name in [
            "c6-r1.csv",
            "c0-r1.csv",
            "c1.csv",
            "c1-r1-b.csv",
            "2026-10-07T01-14-01.csv",
        ] {
            assert_eq!(id(name), None, "{name}");
        }
    }

    #[test]
    fn finds_cycles_outside_the_scenario_state() {
        let capture = capture(
            "c2-r1",
            vec![
                cycle(0, "DegradedLocal", "NoResponse", "", ""),
                cycle(1, "Normal", "", "", "DegradedLocal->Normal"),
            ],
        );
        let check = capture.check();
        assert!(!check.passed());
        assert_eq!(check.divergences.len(), 1);
        assert_eq!(check.divergences[0].cycle, 1);
        assert_eq!(check.divergences[0].expected, OperationState::DegradedLocal);
        assert_eq!(check.divergences[0].observed, "Normal");
    }

    #[test]
    fn accepts_recovery_in_the_first_cycle_with_both_sources() {
        // O DHT11 religado depois do ciclo 2 pode só responder no ciclo 4.
        for back in [3, 4] {
            let capture = recovery_capture(back);
            let check = capture.check();
            assert!(check.divergences.is_empty());
            assert_eq!(check.recovery, Some(Ok(back)));
            assert!(check.passed());
        }
    }

    #[test]
    fn rejects_runs_that_do_not_recover() {
        let no_failure = recovery_capture(0);
        assert_eq!(no_failure.check().recovery, Some(Err("começa sem falha")));

        let never_back = capture(
            "c5-r1",
            vec![cycle(0, "DegradedLocal", "NoResponse", "", "")],
        );
        assert_eq!(
            never_back.check().recovery,
            Some(Err("as fontes não voltaram"))
        );

        let mut unflagged = recovery_capture(3);
        unflagged.cycles[3].transition = None;
        let check = unflagged.check();
        assert!(check.divergences.is_empty());
        assert!(matches!(check.recovery, Some(Err(_))));
        assert!(!check.passed());
    }

    #[test]
    fn reports_captures_and_totals_by_scenario() {
        let captures = [
            capture("c1-r1", vec![cycle(0, "Normal", "", "", "")]),
            capture(
                "c2-r1",
                vec![
                    cycle(0, "DegradedLocal", "NoResponse", "", ""),
                    cycle(1, "Normal", "", "", "DegradedLocal->Normal"),
                    cycle(2, "Normal", "", "", ""),
                    cycle(
                        3,
                        "DegradedLocal",
                        "NoResponse",
                        "",
                        "Normal->DegradedLocal",
                    ),
                    cycle(4, "Normal", "", "", "DegradedLocal->Normal"),
                ],
            ),
            recovery_capture(4),
        ];
        let mut out = Vec::new();
        assert!(!report(&captures, &mut out, PLAIN).unwrap());
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "captura   esperado           ciclos  resultado\n\
             c1-r1     Normal                1/1  ok\n\
             ── C1: 1/1 capturas aprovadas, 1/1 ciclos no estado esperado\n\
             c2-r1     DegradedLocal         2/5  divergente\n\
             \x20         ciclos 1–2: esperado DegradedLocal, observado Normal\n\
             \x20         ciclo 4: esperado DegradedLocal, observado Normal\n\
             ── C2: 0/1 capturas aprovadas, 2/5 ciclos no estado esperado\n\
             c5-r1     recuperação           6/6  ok, recuperação no ciclo 4\n\
             ── C5: 1/1 capturas aprovadas, 6/6 ciclos no estado esperado\n\
             ── total: 2/3 capturas aprovadas, 9/12 ciclos no estado esperado\n"
        );
    }

    #[test]
    fn reads_a_capture_file() {
        let dir = std::env::temp_dir().join(format!("sense_bridge_console_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, text: &str| {
            let path = dir.join(name);
            fs::write(&path, text).unwrap();
            path
        };
        let row = "0,21,Normal,27.0,41.0,,29.1,76.0,0,,,,21911,1460235,1482282,206832,201476,12436";

        let capture = Capture::read(&write(
            "c1-r2.csv",
            &format!("{}\n{row}\n", record::DEFAULT_HEADER),
        ))
        .unwrap();
        assert_eq!(
            (capture.name.as_str(), capture.scenario, capture.repetition),
            ("c1-r2", Scenario::C1, 2)
        );
        assert_eq!(capture.cycles.len(), 1);

        for (name, text) in [
            (
                "captura.csv",
                format!("{}\n{row}\n", record::DEFAULT_HEADER),
            ),
            ("c1-r3.csv", format!("{row}\n")),
            ("c1-r4.csv", format!("{}\n", record::DEFAULT_HEADER)),
            (
                "c1-r5.csv",
                format!("{}\n0,21,Normal\n", record::DEFAULT_HEADER),
            ),
        ] {
            let error = Capture::read(&write(name, &text)).err().unwrap();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{name}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
