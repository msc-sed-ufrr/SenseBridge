//! Percorre os cenários C1–C5 com fontes simuladas e imprime um registro por ciclo em CSV.
//!
//! ```sh
//! cargo run -p sense_bridge --example simulate
//! ```

use sense_bridge::sim::{SimulatedExternal, SimulatedLocal};
use sense_bridge::{
    Config, ExternalFailure, ExternalReading, LocalFailure, LocalReading, Record, SenseBridge,
};

const CYCLES_PER_SCENARIO: usize = 3;
const CYCLE_INTERVAL_MS: u64 = 2_000;

fn main() {
    let local_up: Result<LocalReading, LocalFailure> = Ok(LocalReading::new(25.0, 60.0));
    let local_down = Err(LocalFailure::NoResponse);
    let external_up: Result<ExternalReading, ExternalFailure> =
        Ok(ExternalReading::new(27.5, 70.0));
    let external_down = Err(ExternalFailure::NoConnectivity);

    let scenarios = [
        ("C1", local_up, external_up),
        ("C2", local_down, external_up),
        ("C3", local_up, external_down),
        ("C4", local_down, external_down),
        ("C5", local_up, external_up),
    ];

    let config = Config {
        external_refresh_ms: 0,
        ..Config::default()
    };
    let mut bridge = SenseBridge::new(
        SimulatedLocal::new(local_up),
        SimulatedExternal::new(external_up),
        config,
    );

    println!("scenario,cycle,timestamp_ms,state,local,external,transition");
    let mut now_ms = 0;
    for (scenario, local, external) in scenarios {
        bridge.local_mut().set(local);
        bridge.external_mut().set(external);

        for _ in 0..CYCLES_PER_SCENARIO {
            print_row(scenario, &bridge.cycle(now_ms));
            now_ms += CYCLE_INTERVAL_MS;
        }
    }
}

fn print_row(scenario: &str, record: &Record) {
    let local = match record.local {
        Ok(reading) => format!("{:.1}C/{:.1}%", reading.temperature, reading.humidity),
        Err(failure) => format!("{failure:?}"),
    };
    let external = match record.external {
        Ok(sample) => format!(
            "{:.1}C/{:.1}%",
            sample.reading.temperature, sample.reading.humidity
        ),
        Err(failure) => format!("{failure:?}"),
    };
    let transition = match record.transition {
        Some(t) if t.is_recovery() => format!("{:?}->{:?} (recovery)", t.from, t.to),
        Some(t) => format!("{:?}->{:?}", t.from, t.to),
        None => String::new(),
    };

    println!(
        "{scenario},{},{},{:?},{local},{external},{transition}",
        record.cycle,
        record.timestamp_ms,
        record.state()
    );
}
