//! Interpretação de respostas reais da Open-Meteo, gravadas em `tests/fixtures/open_meteo/`.

use sense_bridge::open_meteo::{self, HOST};
use sense_bridge::sim::{SimulatedExternal, SimulatedLocal};
use sense_bridge::{
    Config, ExternalFailure, ExternalReading, InvalidReason, LocalReading, OperationState,
    SenseBridge,
};

const CURRENT: &[u8] = include_bytes!("fixtures/open_meteo/current.json");
const ERROR_LATITUDE: &[u8] = include_bytes!("fixtures/open_meteo/error_latitude.json");

fn bridge_with(
    external: Result<ExternalReading, ExternalFailure>,
) -> SenseBridge<SimulatedLocal, SimulatedExternal> {
    SenseBridge::new(
        SimulatedLocal::new(Ok(LocalReading::new(25.0, 60.0))),
        SimulatedExternal::new(external),
        Config::default(),
    )
}

#[test]
fn request_path_matches_the_query_used_to_record_the_fixtures() {
    let mut path = String::new();
    open_meteo::write_path(&mut path, 2.82, -60.67).unwrap();

    assert_eq!(HOST, "api.open-meteo.com");
    assert_eq!(
        path,
        "/v1/forecast?latitude=2.82&longitude=-60.67&current=temperature_2m,relative_humidity_2m"
    );
}

#[test]
fn recorded_response_yields_current_temperature_and_humidity() {
    assert_eq!(
        open_meteo::interpret(200, CURRENT),
        Ok(ExternalReading::new(33.7, 51.0))
    );
}

#[test]
fn recorded_error_response_is_a_service_error() {
    assert_eq!(
        open_meteo::interpret(400, ERROR_LATITUDE),
        Err(ExternalFailure::ServiceError)
    );
    // O corpo de erro, se chegasse com status 200, não traz `current`.
    assert_eq!(
        open_meteo::interpret(200, ERROR_LATITUDE),
        Err(ExternalFailure::MalformedResponse)
    );
}

#[test]
fn truncated_recorded_response_is_malformed() {
    for len in [0, 1, CURRENT.len() / 2, CURRENT.len() - 1] {
        assert_eq!(
            open_meteo::interpret(200, &CURRENT[..len]),
            Err(ExternalFailure::MalformedResponse),
            "{len} bytes"
        );
    }
}

#[test]
fn recorded_response_drives_a_normal_cycle() {
    let mut bridge = bridge_with(open_meteo::interpret(200, CURRENT));
    let record = bridge.cycle(0);

    assert_eq!(record.state(), OperationState::Normal);
    assert_eq!(
        record.external.map(|sample| sample.reading),
        Ok(ExternalReading::new(33.7, 51.0))
    );
}

#[test]
fn recorded_error_response_degrades_the_cycle() {
    let mut bridge = bridge_with(open_meteo::interpret(400, ERROR_LATITUDE));
    let record = bridge.cycle(0);

    assert_eq!(record.state(), OperationState::DegradedExternal);
    assert_eq!(record.external, Err(ExternalFailure::ServiceError));
}

#[test]
fn out_of_range_values_in_a_complete_response_are_rejected_by_validation() {
    let body = br#"{"current":{"temperature_2m":1e999,"relative_humidity_2m":51}}"#;
    let mut bridge = bridge_with(open_meteo::interpret(200, body));

    assert_eq!(
        bridge.cycle(0).external,
        Err(ExternalFailure::Invalid(InvalidReason::NotANumber))
    );
}
