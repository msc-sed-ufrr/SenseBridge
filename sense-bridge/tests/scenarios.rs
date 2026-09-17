//! Cenários C1–C5 do protocolo experimental, executados com fontes simuladas.

use sense_bridge::sim::{SimulatedExternal, SimulatedLocal};
use sense_bridge::{
    Config, ExternalFailure, ExternalReading, InvalidReason, LocalFailure, LocalReading,
    OperationState, Record, SenseBridge, Transition,
};

const LOCAL: LocalReading = LocalReading::new(25.0, 60.0);
const EXTERNAL: ExternalReading = ExternalReading::new(27.5, 70.0);

type Bridge = SenseBridge<SimulatedLocal, SimulatedExternal>;

/// Ponte com as duas fontes disponíveis, consultando a fonte externa em todo ciclo.
fn bridge() -> Bridge {
    bridge_with_refresh(0)
}

fn bridge_with_refresh(external_refresh_ms: u64) -> Bridge {
    SenseBridge::new(
        SimulatedLocal::new(Ok(LOCAL)),
        SimulatedExternal::new(Ok(EXTERNAL)),
        Config {
            external_refresh_ms,
            ..Config::default()
        },
    )
}

fn external_reading(record: &Record) -> Option<ExternalReading> {
    record.external.ok().map(|sample| sample.reading)
}

#[test]
fn c1_normal_operation_integrates_both_sources() {
    let mut bridge = bridge();

    let record = bridge.cycle(0);

    assert_eq!(record.state(), OperationState::Normal);
    assert_eq!(record.local, Ok(LOCAL));
    assert_eq!(external_reading(&record), Some(EXTERNAL));
    assert_eq!(record.transition, None);
}

#[test]
fn c2_local_failure_degrades_and_keeps_external_data() {
    let mut bridge = bridge();
    bridge.local_mut().set(Err(LocalFailure::NoResponse));

    let record = bridge.cycle(0);

    assert_eq!(record.state(), OperationState::DegradedLocal);
    assert_eq!(record.local, Err(LocalFailure::NoResponse));
    assert_eq!(external_reading(&record), Some(EXTERNAL));
}

#[test]
fn c3_external_failure_degrades_and_keeps_local_data() {
    let mut bridge = bridge();
    bridge.external_mut().set(Err(ExternalFailure::Timeout));

    let record = bridge.cycle(0);

    assert_eq!(record.state(), OperationState::DegradedExternal);
    assert_eq!(record.local, Ok(LOCAL));
    assert_eq!(record.external, Err(ExternalFailure::Timeout));
}

#[test]
fn c4_simultaneous_failure_reports_error_and_keeps_polling() {
    let mut bridge = bridge();
    bridge.local_mut().set(Err(LocalFailure::Checksum));
    bridge
        .external_mut()
        .set(Err(ExternalFailure::NoConnectivity));

    for now_ms in [0, 2_000, 4_000] {
        let record = bridge.cycle(now_ms);

        assert_eq!(record.state(), OperationState::Error);
        assert_eq!(record.local, Err(LocalFailure::Checksum));
        assert_eq!(record.external, Err(ExternalFailure::NoConnectivity));
    }
    assert_eq!(bridge.local_mut().calls(), 3);
    assert_eq!(bridge.external_mut().calls(), 3);
}

#[test]
fn c5_recovery_returns_to_normal_on_first_cycle_with_both_sources() {
    let mut bridge = bridge();
    bridge.cycle(0);

    bridge.local_mut().set(Err(LocalFailure::NoResponse));
    bridge
        .external_mut()
        .set(Err(ExternalFailure::NoConnectivity));
    let failure = bridge.cycle(2_000);
    assert_eq!(
        failure.transition,
        Some(Transition {
            from: OperationState::Normal,
            to: OperationState::Error,
        })
    );

    bridge.local_mut().set(Ok(LOCAL));
    bridge.external_mut().set(Ok(EXTERNAL));
    let recovery = bridge.cycle(4_000);

    assert_eq!(recovery.state(), OperationState::Normal);
    let transition = recovery
        .transition
        .expect("recuperação deve gerar transição");
    assert_eq!(transition.from, OperationState::Error);
    assert!(transition.is_recovery());
    assert_eq!(recovery.external.map(|sample| sample.age_ms), Ok(0));
}

#[test]
fn transitions_are_reported_only_when_state_changes() {
    let mut bridge = bridge();

    assert_eq!(bridge.state(), None);
    assert_eq!(bridge.cycle(0).transition, None);
    assert_eq!(bridge.cycle(2_000).transition, None);

    bridge
        .external_mut()
        .set(Err(ExternalFailure::ServiceError));
    assert!(bridge.cycle(4_000).transition.is_some());
    assert_eq!(bridge.cycle(6_000).transition, None);
    assert_eq!(bridge.state(), Some(OperationState::DegradedExternal));
}

#[test]
fn cycle_ids_increase_by_one() {
    let mut bridge = bridge();

    let ids: Vec<u32> = (0..3).map(|i| bridge.cycle(i * 2_000).cycle).collect();

    assert_eq!(ids, [0, 1, 2]);
}

#[test]
fn invalid_local_reading_makes_local_source_unavailable() {
    let mut bridge = bridge();
    bridge
        .local_mut()
        .set(Ok(LocalReading::new(f32::NAN, 60.0)));

    let record = bridge.cycle(0);

    assert_eq!(
        record.local,
        Err(LocalFailure::Invalid(InvalidReason::NotANumber))
    );
    assert_eq!(record.state(), OperationState::DegradedLocal);
}

#[test]
fn invalid_external_reading_makes_external_source_unavailable() {
    let mut bridge = bridge();
    bridge
        .external_mut()
        .set(Ok(ExternalReading::new(25.0, 120.0)));

    let record = bridge.cycle(0);

    assert_eq!(
        record.external,
        Err(ExternalFailure::Invalid(InvalidReason::OutOfRange))
    );
    assert_eq!(record.state(), OperationState::DegradedExternal);
}

#[test]
fn divergence_between_sources_does_not_change_state() {
    let mut bridge = bridge();
    bridge.local_mut().set(Ok(LocalReading::new(18.0, 85.0)));
    bridge
        .external_mut()
        .set(Ok(ExternalReading::new(35.0, 30.0)));

    assert_eq!(bridge.cycle(0).state(), OperationState::Normal);
}

#[test]
fn external_reading_is_reused_within_refresh_interval() {
    let mut bridge = bridge_with_refresh(10_000);

    let first = bridge.cycle(0);
    let reused = bridge.cycle(6_000);
    let refreshed = bridge.cycle(10_000);

    assert_eq!(first.external.map(|sample| sample.age_ms), Ok(0));
    assert_eq!(reused.external.map(|sample| sample.age_ms), Ok(6_000));
    assert_eq!(refreshed.external.map(|sample| sample.age_ms), Ok(0));
    assert_eq!(bridge.external_mut().calls(), 2);
}

#[test]
fn failed_fetch_discards_previous_external_reading() {
    let mut bridge = bridge_with_refresh(10_000);
    bridge.cycle(0);

    bridge.external_mut().set(Err(ExternalFailure::Timeout));
    assert_eq!(bridge.cycle(10_000).external, Err(ExternalFailure::Timeout));

    // Ainda dentro do intervalo contado a partir da leitura antiga: sem leitura guardada,
    // a fonte é consultada de novo em vez de reaproveitar o valor anterior à falha.
    let fresh = ExternalReading::new(30.0, 55.0);
    bridge.external_mut().set(Ok(fresh));
    let record = bridge.cycle(12_000);

    assert_eq!(external_reading(&record), Some(fresh));
    assert_eq!(record.external.map(|sample| sample.age_ms), Ok(0));
    assert_eq!(bridge.external_mut().calls(), 3);
}
