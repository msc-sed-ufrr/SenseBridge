//! Cenário C6, leitura local congelada, executado com fontes simuladas.
//!
//! Um sensor travado em um valor plausível passa pela validação de faixa. O
//! `FreezeDetector` marca a suspeita sem mudar o estado de operação, que continua sendo
//! função da disponibilidade das fontes em cada ciclo.

use std::num::NonZeroU32;

use sense_bridge::sim::{SimulatedExternal, SimulatedLocal};
use sense_bridge::{
    Config, ExternalReading, FreezeDetector, LocalFailure, LocalReading, OperationState,
    SenseBridge,
};

const FROZEN: LocalReading = LocalReading::new(24.0, 37.0);
const EXTERNAL: ExternalReading = ExternalReading::new(27.5, 70.0);
const THRESHOLD: u32 = 4;

fn bridge() -> SenseBridge<SimulatedLocal, SimulatedExternal> {
    SenseBridge::new(
        SimulatedLocal::new(Ok(FROZEN)),
        SimulatedExternal::new(Ok(EXTERNAL)),
        Config {
            external_refresh_ms: 0,
            ..Config::default()
        },
    )
}

fn detector() -> FreezeDetector {
    FreezeDetector::new(NonZeroU32::new(THRESHOLD).unwrap())
}

#[test]
fn c6_frozen_reading_is_suspect_but_stays_valid_and_normal() {
    let mut bridge = bridge();
    let mut detector = detector();

    for cycle in 1..=THRESHOLD + 2 {
        let record = bridge.cycle(u64::from(cycle) * 5_000);
        detector.observe(&record.local);

        // A leitura repetida continua válida e o estado não muda.
        assert_eq!(record.local, Ok(FROZEN));
        assert_eq!(record.state(), OperationState::Normal);
        assert_eq!(detector.run(), cycle);
        assert_eq!(detector.is_suspect(), cycle >= THRESHOLD);
    }
}

#[test]
fn c6_a_new_value_clears_the_suspicion() {
    let mut bridge = bridge();
    let mut detector = detector();
    for cycle in 0..THRESHOLD {
        detector.observe(&bridge.cycle(u64::from(cycle) * 5_000).local);
    }
    assert!(detector.is_suspect());

    bridge.local_mut().set(Ok(LocalReading::new(25.0, 37.0)));
    detector.observe(&bridge.cycle(100_000).local);

    assert!(!detector.is_suspect());
    assert_eq!(detector.run(), 1);
}

#[test]
fn c6_a_local_failure_ends_the_sequence_and_degrades_as_before() {
    let mut bridge = bridge();
    let mut detector = detector();
    for cycle in 0..THRESHOLD {
        detector.observe(&bridge.cycle(u64::from(cycle) * 5_000).local);
    }

    bridge.local_mut().set(Err(LocalFailure::NoResponse));
    let record = bridge.cycle(100_000);
    detector.observe(&record.local);

    assert_eq!(record.state(), OperationState::DegradedLocal);
    assert!(!detector.is_suspect());
    assert_eq!(detector.run(), 0);
}
