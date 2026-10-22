# Verificação do SenseBridge

Gerado por `scripts/evidencias.sh --firmware`. Não edite à mão.

| Item | Valor |
| --- | --- |
| Data (UTC) | 2026-10-22T01:57:58Z |
| Commit | `86055dc` |
| Sistema | Darwin arm64 |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| Rust do firmware | rustc 1.97.0-nightly (8ea53bcd7 2026-07-08) (1.97.0.0) |
| ESP-IDF | v5.5.5 |

## Formatação

```text
$ cargo fmt --check
```

Resultado: **ok**

## Análise estática

```text
$ cargo clippy --all-targets -- -D warnings
```

Resultado: **ok**

## Testes

```text
$ cargo test
     Running unittests src/lib.rs (target/debug/deps/sense_bridge)

running 20 tests
test open_meteo::tests::non_200_status_is_a_service_error_even_with_a_valid_body ... ok
test open_meteo::tests::missing_or_non_numeric_values_make_the_response_malformed ... ok
test open_meteo::tests::reads_current_values_and_ignores_same_keys_in_other_objects ... ok
test open_meteo::tests::unrelated_members_of_any_json_type_are_skipped ... ok
test reading::tests::failure_names_are_stable ... ok
test state::tests::both_sources_available_returns_normal ... ok
test open_meteo::tests::member_order_and_whitespace_do_not_matter ... ok
test open_meteo::tests::invalid_or_incomplete_json_makes_the_response_malformed ... ok
test open_meteo::tests::nesting_is_accepted_up_to_the_limit_and_rejected_beyond_it ... ok
test open_meteo::tests::values_are_not_range_checked_here ... ok
test state::tests::both_sources_unavailable_returns_error ... ok
test state::tests::external_source_unavailable_returns_degraded_external ... ok
test state::tests::local_source_unavailable_returns_degraded_local ... ok
test state::tests::only_transitions_into_normal_are_recoveries ... ok
test state::tests::state_names_are_stable ... ok
test validation::tests::invalid_readings_carry_the_reason_for_their_source ... ok
test validation::tests::non_finite_values_are_rejected ... ok
test validation::tests::valid_readings_are_returned_unchanged ... ok
test validation::tests::values_just_outside_range_are_rejected ... ok
test validation::tests::values_on_range_boundaries_are_accepted ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/open_meteo.rs (target/debug/deps/open_meteo)

running 7 tests
test recorded_error_response_is_a_service_error ... ok
test out_of_range_values_in_a_complete_response_are_rejected_by_validation ... ok
test recorded_response_yields_current_temperature_and_humidity ... ok
test request_path_matches_the_query_used_to_record_the_fixtures ... ok
test recorded_error_response_degrades_the_cycle ... ok
test recorded_response_drives_a_normal_cycle ... ok
test truncated_recorded_response_is_malformed ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/scenarios.rs (target/debug/deps/scenarios)

running 12 tests
test c1_normal_operation_integrates_both_sources ... ok
test c3_external_failure_degrades_and_keeps_local_data ... ok
test c2_local_failure_degrades_and_keeps_external_data ... ok
test c5_recovery_returns_to_normal_on_first_cycle_with_both_sources ... ok
test divergence_between_sources_does_not_change_state ... ok
test cycle_ids_increase_by_one ... ok
test external_reading_is_reused_within_refresh_interval ... ok
test failed_fetch_discards_previous_external_reading ... ok
test invalid_external_reading_makes_external_source_unavailable ... ok
test c4_simultaneous_failure_reports_error_and_keeps_polling ... ok
test invalid_local_reading_makes_local_source_unavailable ... ok
test transitions_are_reported_only_when_state_changes ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running unittests src/main.rs (target/debug/deps/sense_bridge_console)

running 14 tests
test tests::ends_lines_with_carriage_return ... ok
test record::tests::classifies_serial_lines ... ok
test record::tests::rejects_incomplete_or_garbled_rows ... ok
test record::tests::follows_the_header_column_order ... ok
test record::tests::parses_recovery ... ok
test record::tests::parses_normal_cycle_with_fetch ... ok
test tests::parses_options ... ok
test record::tests::parses_external_failure ... ok
test record::tests::parses_local_failure_and_transition ... ok
test tests::stamps_utc_dates ... ok
test summary::tests::counts_states_failures_and_extremes ... ok
test view::tests::formats_times_and_sizes ... ok
test view::tests::formats_a_cycle_with_its_transition ... ok
test view::tests::marks_a_fetch_in_the_cycle ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

   Doc-tests sense_bridge

running 2 tests
test sense-bridge/src/../README.md - (line 70) ... ok
test sense-bridge/src/../README.md - (line 30) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

all doctests ran in 0.88s; merged doctests compilation took 0.46s
```

Resultado: **ok**

## Documentação da API

```text
$ RUSTDOCFLAGS='-D warnings' cargo doc -p sense_bridge --no-deps
```

Resultado: **ok**

## Pacote para o crates.io

```text
$ cargo package -p sense_bridge --allow-dirty
    Packaged 23 files, 64.7KiB (18.8KiB compressed)
```

Resultado: **ok**

## Formatação do firmware

```text
$ rustfmt --edition 2024 --check firmware/esp32/src/main.rs firmware/esp32/build.rs
```

Resultado: **ok**

## Análise estática do firmware

```text
$ cd firmware/esp32 && cargo clippy --release
```

Resultado: **ok**

## Tamanho do firmware (release)

```text
$ cd firmware/esp32 && cargo build --release && xtensa-esp32-elf-size target/xtensa-esp32-espidf/release/sense_bridge_esp32
   text	   data	    bss	    dec	    hex	filename
 939016	 192642	  19883	1151541	 119235	target/xtensa-esp32-espidf/release/sense_bridge_esp32
```

Resultado: **ok**
