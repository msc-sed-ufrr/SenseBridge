//! Correspondência entre o estado esperado e o observado nas capturas de C1–C6 (UC-008)
//! e, para calibrar o limiar da suspeita de leitura congelada, as sequências de leituras
//! locais iguais em qualquer captura.
//!
//! O CSV do firmware não traz o cenário nem a repetição: eles vêm do nome do arquivo,
//! `c<cenário>-r<repetição>.csv`, como em `docs/evidencias/experimentos/c2-r1.csv`.

use std::fmt::{self, Write as _};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sense_bridge::{OperationState, determine_state};

use crate::record::{self, Columns, Cycle, Kind};
use crate::view::{Style, clock};

/// Cenário do protocolo experimental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Scenario {
    C1,
    C2,
    C3,
    C4,
    C5,
    /// Leitura local congelada por injeção de falha, com suspeita e reinício do sensor (D14).
    C6,
}

impl Scenario {
    fn from_number(number: u8) -> Option<Self> {
        match number {
            1 => Some(Self::C1),
            2 => Some(Self::C2),
            3 => Some(Self::C3),
            4 => Some(Self::C4),
            5 => Some(Self::C5),
            6 => Some(Self::C6),
            _ => None,
        }
    }

    /// Estado de todos os ciclos, pela tabela de cenários do README. `None` em C5, em que a
    /// falha é desfeita durante a captura, e em C6, em que o reinício do sensor pode custar uma
    /// leitura.
    fn fixed_state(self) -> Option<OperationState> {
        match self {
            Self::C1 => Some(OperationState::Normal),
            Self::C2 => Some(OperationState::DegradedLocal),
            Self::C3 => Some(OperationState::DegradedExternal),
            Self::C4 => Some(OperationState::Error),
            Self::C5 | Self::C6 => None,
        }
    }

    fn expected(self, cycle: &Cycle) -> OperationState {
        // Em C5, o ciclo em que a fonte religada volta a responder depende do hardware: espera-se
        // o estado da disponibilidade observada, e a recuperação é conferida à parte. Em C6, a
        // leitura congelada é válida e não muda o estado; a suspeita é conferida à parte.
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
        let cycles = read_cycles(path)?;
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
            frozen: (self.scenario == Scenario::C6).then(|| frozen(&self.cycles)),
        }
    }
}

/// Ciclos de um CSV gravado pelo console, com o cabeçalho na primeira linha.
fn read_cycles(path: &Path) -> io::Result<Vec<Cycle>> {
    let invalid = |message: &str| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {message}", path.display()),
        )
    };
    let text = fs::read_to_string(path)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;

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
            columns
                .cycle(line)
                .ok_or_else(|| invalid(&format!("a linha {} não é um ciclo íntegro", index + 1)))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if cycles.is_empty() {
        return Err(invalid("a captura não tem ciclos"));
    }
    Ok(cycles)
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
    /// Em C6, a suspeita e o reinício ou por que não valem.
    frozen: Option<Result<Frozen, &'static str>>,
}

impl Check<'_> {
    fn passed(&self) -> bool {
        self.divergences.is_empty()
            && !matches!(self.recovery, Some(Err(_)))
            && !matches!(self.frozen, Some(Err(_)))
    }
}

/// Resultado de C6 que passou na conferência.
#[derive(Debug, PartialEq, Eq)]
struct Frozen {
    /// Primeiro ciclo com a leitura marcada como suspeita.
    suspect: u64,
    /// Limiar do detector, inferido desse ciclo: a contagem em que a suspeita começa.
    threshold: u64,
    /// Primeiro ciclo depois do reinício por suspeita, já com leitura nova.
    restarted: u64,
}

/// Leitura congelada em C6 (BR-18, D14): a captura tem as colunas do detector; a suspeita
/// coincide, em todo ciclo, com a contagem igual ou acima do limiar; depois da primeira
/// suspeita há um reinício por suspeita; e o ciclo seguinte a ele traz uma leitura nova.
fn frozen(cycles: &[Cycle]) -> Result<Frozen, &'static str> {
    if cycles.iter().all(|cycle| cycle.local_repeat.is_none()) {
        return Err("sem as colunas do detector (local_frozen_after = 0?)");
    }
    let first = cycles
        .iter()
        .position(|cycle| cycle.local_suspect == Some(true))
        .ok_or("nenhuma leitura marcada como suspeita")?;
    let threshold = cycles[first]
        .local_repeat
        .ok_or("suspeita sem a contagem de leituras iguais")?;
    let consistent = cycles.iter().all(|cycle| match cycle.local_repeat {
        Some(run) => cycle.local_suspect == Some(run >= threshold),
        None => cycle.local_suspect.is_none(),
    });
    if !consistent {
        return Err("suspeita inconsistente com a contagem de leituras iguais");
    }
    let restart = cycles[first + 1..]
        .iter()
        .find(|cycle| cycle.local_restart.as_deref() == Some("suspect"))
        .ok_or("sem reinício por suspeita (local_frozen_restart = false?)")?;
    match (&restart.local, restart.local_repeat) {
        (Err(_), _) => Err("sem leitura válida depois do reinício"),
        (Ok(_), Some(1)) => Ok(Frozen {
            suspect: cycles[first].cycle,
            threshold,
            restarted: restart.cycle,
        }),
        _ => Err("a leitura persistiu depois do reinício"),
    }
}

fn both_valid(cycle: &Cycle) -> bool {
    cycle.local.is_ok() && cycle.external.is_ok()
}

/// Recuperação em C5 (BR-30): a captura começa com falha, e o primeiro ciclo com as duas
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

/// Maior sequência de ciclos seguidos com a mesma leitura local válida: o número de ciclos e o
/// intervalo, em ms, entre o primeiro e o último. Uma falha local encerra a sequência, como no
/// `FreezeDetector`.
fn longest_repeat(cycles: &[Cycle]) -> Option<(usize, u64)> {
    let same = |a: &Cycle, b: &Cycle| matches!((&a.local, &b.local), (Ok(x), Ok(y)) if x == y);
    cycles
        .chunk_by(same)
        .filter(|run| run[0].local.is_ok())
        .map(|run| {
            let span = run[run.len() - 1].timestamp_ms - run[0].timestamp_ms;
            (run.len(), span)
        })
        .max()
}

/// Para calibrar o limiar de `local_frozen_after`: a maior sequência de leituras locais iguais
/// de cada captura, com qualquer nome, e a maior de todas. Sem conferência de cenário.
pub fn repeats(files: &[PathBuf], out: &mut impl Write, style: Style) -> io::Result<()> {
    writeln!(
        out,
        "{}",
        style.bold(&format!(
            "{:<32}  {:>7}  {:>16}",
            "captura", "válidas", "maior sequência"
        ))
    )?;
    let mut longest: Option<(usize, u64, String)> = None;
    for path in files {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        // Uma captura vazia ou sem cabeçalho, como a de uma sessão interrompida, não impede a
        // calibração com as demais.
        let cycles = match read_cycles(path) {
            Ok(cycles) => cycles,
            Err(error) => {
                let message = error.to_string();
                let reason = message
                    .split_once(": ")
                    .map_or(message.as_str(), |(_, r)| r);
                writeln!(out, "{name:<32}  ignorada: {reason}")?;
                continue;
            }
        };
        let valid = cycles.iter().filter(|cycle| cycle.local.is_ok()).count();
        let text = match longest_repeat(&cycles) {
            Some((run, span)) => {
                if longest.as_ref().is_none_or(|(best, ..)| run > *best) {
                    longest = Some((run, span, name.clone()));
                }
                format!("{run} ({})", clock(span))
            }
            None => "—".to_owned(),
        };
        writeln!(out, "{name:<32}  {valid:>7}  {text:>16}")?;
    }
    let total = match longest {
        Some((run, span, name)) => format!(
            "── maior sequência: {run} ciclos iguais ({}), em {name}; o limiar deve ficar bem \
             acima",
            clock(span)
        ),
        None => "── nenhuma leitura local válida".to_owned(),
    };
    writeln!(out, "{}", style.bold(&total))
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
    memory(captures, out, style)?;
    Ok(total.passed == total.captures)
}

/// RAM em execução por cenário, das colunas do próprio CSV (BR-33): o menor `heap_free`, o
/// menor `heap_min_free` e o menor `stack_min_free` entre todos os ciclos do cenário, como na
/// Tabela 5 do relatório entregue. É a RAM do firmware inteiro: sem um firmware-base rodando o
/// mesmo cenário, não isola o custo da biblioteca.
fn memory(captures: &[Capture], out: &mut impl Write, style: Style) -> io::Result<()> {
    writeln!(
        out,
        "\n{}",
        style.bold(&format!(
            "{:<8}  {:>6}  {:>16}  {:>22}  {:>18}",
            "cenário", "ciclos", "menor heap livre", "menor heap desde boot", "menor folga pilha"
        ))
    )?;
    for group in captures.chunk_by(|a, b| a.scenario == b.scenario) {
        let cycles: Vec<&Cycle> = group.iter().flat_map(|c| &c.cycles).collect();
        // `read_cycles` recusa captura sem ciclos, então o grupo nunca fica vazio.
        let min = |column: fn(&Cycle) -> u64| cycles.iter().map(|c| column(c)).min().unwrap_or(0);
        writeln!(
            out,
            "{:<8}  {:>6}  {:>14} B  {:>20} B  {:>16} B",
            format!("{:?}", group[0].scenario),
            cycles.len(),
            min(|c| c.heap_free),
            min(|c| c.heap_min_free),
            min(|c| c.stack_min_free)
        )?;
    }
    Ok(())
}

/// Uma linha por captura e, abaixo dela, uma por sequência de ciclos com a mesma divergência.
fn line(capture: &Capture, check: &Check, style: Style) -> String {
    // Cada coluna é alinhada antes de receber cor, para que os escapes não contem na largura.
    let expected = match (capture.scenario.fixed_state(), capture.scenario) {
        (Some(state), _) => style.state(state.as_str(), &format!("{:<16}", state.as_str())),
        (None, Scenario::C6) => format!("{:<16}", "congelamento"),
        (None, _) => format!("{:<16}", "recuperação"),
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
    let _ = match &check.frozen {
        Some(Ok(frozen)) => write!(
            line,
            ", suspeita no ciclo {} (limiar {}), leitura nova no ciclo {} após o reinício",
            frozen.suspect, frozen.threshold, frozen.restarted
        ),
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
        assert_eq!(id("c6-r2.csv"), Some((Scenario::C6, 2)));
        for name in [
            "c7-r1.csv",
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
             ── total: 2/3 capturas aprovadas, 9/12 ciclos no estado esperado\n\
             \n\
             cenário   ciclos  menor heap livre   menor heap desde boot   menor folga pilha\n\
             C1             1          206948 B                201476 B             12436 B\n\
             C2             5          206948 B                201476 B             12436 B\n\
             C5             6          206948 B                201476 B             12436 B\n"
        );
    }

    #[test]
    fn reports_runtime_ram_by_scenario_br_33() {
        // Duas repetições de C2: cada mínimo vem de um ciclo ou de uma captura diferente.
        let with_ram = |number: u64, heap: u64, heap_min: u64, stack_min: u64| Cycle {
            heap_free: heap,
            heap_min_free: heap_min,
            stack_min_free: stack_min,
            ..cycle(number, "DegradedLocal", "NoResponse", "", "")
        };
        let captures = [
            capture(
                "c2-r1",
                vec![
                    with_ram(0, 200_000, 199_000, 12_500),
                    with_ram(1, 201_000, 198_500, 12_400),
                ],
            ),
            capture("c2-r2", vec![with_ram(0, 202_000, 199_200, 12_380)]),
        ];
        let mut out = Vec::new();
        memory(&captures, &mut out, PLAIN).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "\ncenário   ciclos  menor heap livre   menor heap desde boot   menor folga pilha\n\
             C2             3          200000 B                198500 B             12380 B\n"
        );
    }

    /// C6: leitura congelada a partir do ciclo 2, suspeita no ciclo 6 (limiar 5), reinício
    /// depois dele e leitura nova no ciclo 7.
    fn frozen_capture() -> Capture {
        let row = |number: u64, reading: &str, run: u64, restart: &str| {
            let suspect = run >= 5;
            let line = format!(
                "{number},{},Normal,{reading},,29.1,76.0,0,,,,22000,,22254,206948,201476,12436,\
                 {run},{suspect},{restart}",
                number * 5000
            );
            Columns::default().cycle(&line).unwrap()
        };
        let cycles = vec![
            row(0, "30.0,31.0", 1, ""),
            row(1, "30.0,31.0", 2, ""),
            row(2, "40.0,80.0", 1, ""),
            row(3, "40.0,80.0", 2, ""),
            row(4, "40.0,80.0", 3, ""),
            row(5, "40.0,80.0", 4, ""),
            row(6, "40.0,80.0", 5, ""),
            row(7, "30.0,31.0", 1, "suspect"),
            row(8, "30.0,31.0", 2, ""),
        ];
        capture("c6-r1", cycles)
    }

    #[test]
    fn accepts_a_frozen_reading_flagged_and_cleared_by_the_restart() {
        let capture = frozen_capture();
        let check = capture.check();
        assert!(check.divergences.is_empty());
        assert_eq!(
            check.frozen,
            Some(Ok(Frozen {
                suspect: 6,
                threshold: 5,
                restarted: 7
            }))
        );
        assert!(check.passed());
    }

    #[test]
    fn rejects_frozen_runs_without_suspicion_restart_or_change() {
        let reason = |capture: Capture| capture.check().frozen.unwrap().unwrap_err();

        let mut no_detector = frozen_capture();
        for cycle in &mut no_detector.cycles {
            cycle.local_repeat = None;
            cycle.local_suspect = None;
        }
        assert_eq!(
            reason(no_detector),
            "sem as colunas do detector (local_frozen_after = 0?)"
        );

        let mut no_restart = frozen_capture();
        no_restart.cycles[7].local_restart = None;
        assert_eq!(
            reason(no_restart),
            "sem reinício por suspeita (local_frozen_restart = false?)"
        );

        let mut persisted = frozen_capture();
        persisted.cycles[7].local_repeat = Some(6);
        persisted.cycles[7].local_suspect = Some(true);
        assert_eq!(reason(persisted), "a leitura persistiu depois do reinício");

        let mut inconsistent = frozen_capture();
        // Contagem acima do limiar sem a suspeita marcada.
        inconsistent.cycles[8].local_repeat = Some(6);
        assert_eq!(
            reason(inconsistent),
            "suspeita inconsistente com a contagem de leituras iguais"
        );

        let mut never = frozen_capture();
        for cycle in &mut never.cycles {
            cycle.local_suspect = Some(false);
        }
        assert_eq!(reason(never), "nenhuma leitura marcada como suspeita");
    }

    #[test]
    fn reports_a_frozen_capture() {
        let mut out = Vec::new();
        assert!(report(&[frozen_capture()], &mut out, PLAIN).unwrap());
        assert!(String::from_utf8(out).unwrap().contains(
            "c6-r1     congelamento          9/9  ok, suspeita no ciclo 6 (limiar 5), leitura nova \
             no ciclo 7 após o reinício"
        ));
    }

    #[test]
    fn finds_the_longest_run_of_equal_readings() {
        let cycles = frozen_capture().cycles;
        // Ciclos 2 a 6, de 10 s a 30 s.
        assert_eq!(longest_repeat(&cycles), Some((5, 20_000)));

        let mut interrupted = cycles.clone();
        interrupted[4] = cycle(4, "DegradedLocal", "NoResponse", "", "");
        assert_eq!(longest_repeat(&interrupted), Some((2, 5_000)));

        let failures = vec![cycle(0, "DegradedLocal", "NoResponse", "", "")];
        assert_eq!(longest_repeat(&failures), None);
    }

    #[test]
    fn calibration_skips_unreadable_captures() {
        let dir = std::env::temp_dir().join(format!("sense_bridge_repeats_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let empty = dir.join("vazia.csv");
        fs::write(&empty, "").unwrap();
        let long = dir.join("longa.csv");
        let row = |n: u64| {
            format!(
                "{n},{},Normal,27.0,41.0,,29.1,76.0,0,,,,21911,,22282,206832,201476,12436",
                n * 5000
            )
        };
        let rows: Vec<String> = (0..4).map(row).collect();
        fs::write(
            &long,
            format!("{}\n{}\n", record::DEFAULT_HEADER, rows.join("\n")),
        )
        .unwrap();

        let mut out = Vec::new();
        repeats(&[empty, long], &mut out, PLAIN).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(
            "vazia                             ignorada: a primeira linha não é o cabeçalho do CSV"
        ));
        assert!(text.contains("longa                                   4      4 (00:00:15)"));
        assert!(text.contains("── maior sequência: 4 ciclos iguais (00:00:15), em longa"));
        fs::remove_dir_all(&dir).unwrap();
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
