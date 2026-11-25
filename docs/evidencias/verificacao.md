# Verificação do SenseBridge

Gerado por `scripts/evidencias.sh --firmware`. Não edite à mão.

| Item | Valor |
| --- | --- |
| Data (UTC) | 2026-11-25T18:03:03Z |
| Commit | `abbba1f` |
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

running 28 tests
test freeze::tests::a_change_restarts_the_period ... ok
test freeze::tests::a_local_failure_ends_the_sequence ... ok
test freeze::tests::any_change_restarts_the_count ... ok
test freeze::tests::repeated_reading_becomes_suspect_at_the_threshold ... ok
test freeze::tests::starts_without_suspicion ... ok
test freeze::tests::the_count_saturates ... ok
test freeze::tests::no_threshold_is_reached_without_a_reading ... ok
test freeze::tests::the_threshold_is_reached_once_per_period_while_the_reading_persists ... ok
test open_meteo::tests::member_order_and_whitespace_do_not_matter ... ok
test open_meteo::tests::invalid_or_incomplete_json_makes_the_response_malformed ... ok
test open_meteo::tests::missing_or_non_numeric_values_make_the_response_malformed ... ok
test open_meteo::tests::nesting_is_accepted_up_to_the_limit_and_rejected_beyond_it ... ok
test open_meteo::tests::non_200_status_is_a_service_error_even_with_a_valid_body ... ok
test open_meteo::tests::reads_current_values_and_ignores_same_keys_in_other_objects ... ok
test open_meteo::tests::unrelated_members_of_any_json_type_are_skipped ... ok
test open_meteo::tests::values_are_not_range_checked_here ... ok
test reading::tests::failure_names_are_stable ... ok
test state::tests::both_sources_available_returns_normal ... ok
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

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/frozen.rs (target/debug/deps/frozen)

running 3 tests
test c6_a_local_failure_ends_the_sequence_and_degrades_as_before ... ok
test c6_a_new_value_clears_the_suspicion ... ok
test c6_frozen_reading_is_suspect_but_stays_valid_and_normal ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/open_meteo.rs (target/debug/deps/open_meteo)

running 7 tests
test recorded_error_response_degrades_the_cycle ... ok
test recorded_error_response_is_a_service_error ... ok
test out_of_range_values_in_a_complete_response_are_rejected_by_validation ... ok
test request_path_matches_the_query_used_to_record_the_fixtures ... ok
test truncated_recorded_response_is_malformed ... ok
test recorded_response_yields_current_temperature_and_humidity ... ok
test recorded_response_drives_a_normal_cycle ... ok

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
test invalid_external_reading_makes_external_source_unavailable ... ok
test c4_simultaneous_failure_reports_error_and_keeps_polling ... ok
test failed_fetch_discards_previous_external_reading ... ok
test invalid_local_reading_makes_local_source_unavailable ... ok
test transitions_are_reported_only_when_state_changes ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running unittests src/main.rs (target/debug/deps/sense_bridge_console)

running 31 tests
test analysis::tests::identifies_scenario_and_repetition_by_file_name ... ok
test analysis::tests::rejects_runs_that_do_not_recover ... ok
test analysis::tests::reports_a_frozen_capture ... ok
test analysis::tests::rejects_frozen_runs_without_suspicion_restart_or_change ... ok
test analysis::tests::accepts_a_frozen_reading_flagged_and_cleared_by_the_restart ... ok
test analysis::tests::accepts_recovery_in_the_first_cycle_with_both_sources ... ok
test analysis::tests::finds_the_longest_run_of_equal_readings ... ok
test analysis::tests::reads_a_capture_file ... ok
test analysis::tests::finds_cycles_outside_the_scenario_state ... ok
test analysis::tests::reports_captures_and_totals_by_scenario ... ok
test analysis::tests::reports_runtime_ram_by_scenario_br_33 ... ok
test record::tests::a_header_without_the_trailing_columns_still_requires_every_field ... ok
test record::tests::classifies_serial_lines ... ok
test record::tests::follows_the_header_column_order ... ok
test record::tests::parses_external_failure ... ok
test record::tests::parses_local_failure_and_transition ... ok
test analysis::tests::calibration_skips_unreadable_captures ... ok
test record::tests::parses_normal_cycle_with_fetch ... ok
test record::tests::parses_recovery ... ok
test record::tests::parses_the_frozen_reading_columns ... ok
test record::tests::rejects_incomplete_or_garbled_rows ... ok
test record::tests::rows_without_the_trailing_columns_leave_them_empty ... ok
test tests::ends_lines_with_carriage_return ... ok
test summary::tests::counts_states_failures_and_extremes ... ok
test tests::parses_captures_to_analyze ... ok
test tests::parses_options ... ok
test tests::stamps_utc_dates ... ok
test view::tests::formats_times_and_sizes ... ok
test view::tests::formats_a_cycle_with_its_transition ... ok
test view::tests::marks_a_fetch_in_the_cycle ... ok
test view::tests::shows_a_sensor_restart_before_the_cycle ... ok

test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

   Doc-tests sense_bridge

running 3 tests
test sense-bridge/src/../README.md - (line 70) ... ok
test sense-bridge/src/../README.md - (line 92) ... ok
test sense-bridge/src/../README.md - (line 30) ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

all doctests ran in 0.66s; merged doctests compilation took 0.38s
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
    Packaged 25 files, 75.7KiB (21.5KiB compressed)
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
 941020	 193234	  19883	1154137	 119c59	target/xtensa-esp32-espidf/release/sense_bridge_esp32
```

Resultado: **ok**
