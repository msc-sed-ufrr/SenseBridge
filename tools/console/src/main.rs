//! Console de host que acompanha os ciclos do firmware SenseBridge.
//!
//! Lê a saída do `espflash monitor` pela entrada padrão, mostra cada ciclo formatado, com
//! o estado em cor e as transições destacadas, e grava a captura bruta e o CSV:
//!
//! ```sh
//! espflash monitor --non-interactive | cargo run -q -p sense_bridge_console
//! ```

mod record;
mod summary;
mod view;

use std::fs::{self, File};
use std::io::{self, BufRead, IsTerminal, LineWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::record::{Columns, Kind};
use crate::summary::Summary;
use crate::view::Style;

/// Um resumo a cada 12 ciclos: um minuto com o ciclo padrão de 5 s.
const SUMMARY_EVERY: u64 = 12;

/// Tag dos logs do próprio firmware, mostrados mesmo sem `--logs`.
const FIRMWARE_TAG: &str = ") sense_bridge_esp32";

const USAGE: &str = "\
Uso: espflash monitor --non-interactive | sense_bridge_console [opções]

Opções:
  --logs            mostra todos os logs do ESP-IDF, esmaecidos
  --out <prefixo>   grava em <prefixo>.txt e <prefixo>.csv (padrão: capturas/<data UTC>)
  --no-save         não grava a captura
  -h, --help        mostra esta ajuda";

struct Options {
    logs: bool,
    out: Option<PathBuf>,
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&options) {
        // O terminal ou o pipe de saída fechou: nada mais a mostrar.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("erro: {error}");
            ExitCode::FAILURE
        }
        Ok(()) => ExitCode::SUCCESS,
    }
}

/// `Ok(None)`: ajuda pedida.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Option<Options>, String> {
    let mut options = Options {
        logs: false,
        out: Some(default_prefix()),
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--logs" => options.logs = true,
            "--no-save" => options.out = None,
            "--out" => {
                let prefix = args.next().ok_or("--out precisa de um prefixo")?;
                options.out = Some(PathBuf::from(prefix));
            }
            "-h" | "--help" => return Ok(None),
            other => return Err(format!("opção desconhecida: {other}")),
        }
    }
    Ok(Some(options))
}

fn default_prefix() -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    PathBuf::from("capturas").join(utc_stamp(seconds))
}

/// `AAAA-MM-DDTHH-MM-SS` em UTC, sem `:` para servir de nome de arquivo.
fn utc_stamp(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let time = unix_seconds % 86_400;
    // Conversão de dias desde 1970-01-01 para data civil (algoritmo de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}",
        time / 3600,
        time / 60 % 60,
        time % 60
    )
}

/// Arquivos da captura, gravados linha a linha para que um Ctrl-C não perca nada.
struct Capture {
    raw: LineWriter<File>,
    csv: LineWriter<File>,
    csv_header: bool,
}

impl Capture {
    fn create(prefix: &Path) -> io::Result<Self> {
        if let Some(dir) = prefix.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        // Acrescenta a extensão, em vez de trocar: `--out c1.r2` grava `c1.r2.txt`.
        let file = |extension: &str| File::create(format!("{}.{extension}", prefix.display()));
        Ok(Self {
            raw: LineWriter::new(file("txt")?),
            csv: LineWriter::new(file("csv")?),
            csv_header: false,
        })
    }

    /// O CSV recebe um único cabeçalho, mesmo com vários boots na mesma captura.
    fn row(&mut self, columns: &Columns, line: &str) -> io::Result<()> {
        if !self.csv_header {
            writeln!(self.csv, "{}", columns.header())?;
            self.csv_header = true;
        }
        writeln!(self.csv, "{line}")
    }
}

fn run(options: &Options) -> io::Result<()> {
    let terminal = io::stdout().is_terminal();
    let style = Style {
        color: terminal && std::env::var_os("NO_COLOR").is_none(),
    };
    let mut out: Box<dyn Write> = if terminal {
        Box::new(CrLf(io::stdout().lock()))
    } else {
        Box::new(io::stdout().lock())
    };
    let mut capture = match &options.out {
        Some(prefix) => {
            let capture = Capture::create(prefix)?;
            writeln!(
                out,
                "{}",
                style.dim(&format!("captura: {}.{{txt,csv}}", prefix.display()))
            )?;
            Some(capture)
        }
        None => None,
    };

    let mut columns: Option<Columns> = None;
    let mut summary = Summary::default();
    let mut last_cycle: Option<u64> = None;
    let mut input = io::stdin().lock();
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        if input.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        // Ruído na serial durante o reset não é UTF-8 válido.
        let text = String::from_utf8_lossy(&buffer);
        let line = text.trim_end_matches(['\r', '\n']);
        if let Some(capture) = &mut capture {
            writeln!(capture.raw, "{line}")?;
        }

        match record::classify(line) {
            Kind::Header => {
                columns = Some(Columns::parse(line));
                writeln!(out, "{}", view::header(style))?;
            }
            Kind::Data => {
                let columns = columns.get_or_insert_with(|| {
                    // Console aberto depois do boot: o cabeçalho já passou.
                    let _ = writeln!(out, "{}", view::header(style));
                    Columns::default()
                });
                let Some(cycle) = columns.cycle(line) else {
                    if options.logs {
                        writeln!(out, "{}", style.dim(line))?;
                    }
                    continue;
                };
                // Numeração que volta sem banner de reset: o dispositivo reiniciou.
                if last_cycle.is_some_and(|last| cycle.cycle <= last) {
                    restart(&mut out, &mut summary, style, "numeração reiniciada")?;
                }
                last_cycle = Some(cycle.cycle);
                if let Some(capture) = &mut capture {
                    capture.row(columns, line)?;
                }
                summary.add(&cycle);
                writeln!(out, "{}", view::cycle(&cycle, style))?;
                if summary.cycles() % SUMMARY_EVERY == 0 {
                    writeln!(out, "{}", summary.render(style))?;
                }
            }
            Kind::Warning => writeln!(out, "{}", style.yellow(line))?,
            Kind::Error => writeln!(out, "{}", style.red(line))?,
            Kind::Reset => {
                last_cycle = None;
                restart(&mut out, &mut summary, style, line)?;
            }
            Kind::Other if options.logs || line.contains(FIRMWARE_TAG) => {
                writeln!(out, "{}", style.dim(line))?;
            }
            Kind::Other => {}
        }
    }

    if summary.cycles() % SUMMARY_EVERY != 0 {
        writeln!(out, "{}", summary.render(style))?;
    }
    Ok(())
}

/// Termina cada linha com `\r\n`.
///
/// O `espflash monitor` põe em modo bruto o terminal que divide com o console, mesmo com
/// `--non-interactive`, e o terminal deixa de converter `\n` em `\r\n`. Sem o `\r`, cada
/// linha começaria na coluna em que a anterior terminou.
struct CrLf<W>(W);

impl<W: Write> Write for CrLf<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for chunk in buf.split_inclusive(|&byte| byte == b'\n') {
            match chunk.strip_suffix(b"\n") {
                Some(line) => {
                    self.0.write_all(line)?;
                    self.0.write_all(b"\r\n")?;
                }
                None => self.0.write_all(chunk)?,
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// Fecha o resumo do boot anterior e marca o reinício.
fn restart(
    out: &mut impl Write,
    summary: &mut Summary,
    style: Style,
    reason: &str,
) -> io::Result<()> {
    if summary.cycles() % SUMMARY_EVERY != 0 {
        writeln!(out, "{}", summary.render(style))?;
    }
    *summary = Summary::default();
    writeln!(
        out,
        "{}",
        style.bold(&format!("── reinício do dispositivo: {reason}"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_utc_dates() {
        assert_eq!(utc_stamp(0), "1970-01-01T00-00-00");
        assert_eq!(utc_stamp(951_782_400), "2000-02-29T00-00-00");
        assert_eq!(utc_stamp(1_790_125_505), "2026-09-23T01-05-05");
    }

    #[test]
    fn ends_lines_with_carriage_return() {
        let mut out = CrLf(Vec::new());
        write!(out, "0  Normal\n   ↳ Normal → DegradedLocal\nsem fim").unwrap();
        assert_eq!(
            out.0,
            "0  Normal\r\n   ↳ Normal → DegradedLocal\r\nsem fim".as_bytes()
        );
    }

    #[test]
    fn parses_options() {
        let args = |list: &[&str]| parse_args(list.iter().map(|arg| arg.to_string()));
        assert!(args(&[]).unwrap().unwrap().out.is_some());
        assert!(args(&["--no-save"]).unwrap().unwrap().out.is_none());
        let options = args(&["--logs", "--out", "c1"]).unwrap().unwrap();
        assert!(options.logs);
        assert_eq!(options.out, Some(PathBuf::from("c1")));
        assert!(args(&["--help"]).unwrap().is_none());
        assert!(args(&["--out"]).is_err());
        assert!(args(&["--verbose"]).is_err());
    }
}
