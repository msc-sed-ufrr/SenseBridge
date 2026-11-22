//! Firmware de referência do SenseBridge no ESP32.
//!
//! DHT11 como fonte local, Open-Meteo como fonte externa e um registro por ciclo emitido em
//! CSV pela serial. Parâmetros e credenciais vêm de `cfg.toml` (ver `cfg.toml.example`).

mod csv;
mod external;
mod local;
mod timed;

mod config {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

use std::num::{NonZeroU32, NonZeroU64};
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{self, EspError};
use esp_idf_svc::wifi::{self, AuthMethod, ClientConfiguration, EspWifi, WifiEvent};
use log::{info, warn};
use sense_bridge::{Config, FreezeDetector, LocalFailure, LocalReading, SenseBridge};

use crate::csv::Timing;
use crate::external::OpenMeteo;
use crate::local::{Dht11, FreezeTest, Restart};
use crate::timed::Timed;

type Bridge = SenseBridge<Timed<Dht11<'static>>, Timed<OpenMeteo<'static>>>;

/// Espera máxima pela rede antes do primeiro ciclo. Um ponto de acesso pode levar vários
/// segundos para recusar a primeira autenticação.
const NETWORK_WAIT: Duration = Duration::from_secs(30);

fn main() -> Result<(), EspError> {
    sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // DHT11 no GPIO escolhido em `cfg.toml` (padrão: GPIO4), ligação descrita no README.
    // Com `dht11_power_gpio`, o VCC do sensor vem desse GPIO, que permite reiniciá-lo.
    match config::DHT11_POWER_GPIO {
        Some(power) => info!(
            "DHT11 no GPIO{}, alimentado pelo GPIO{power}",
            config::DHT11_GPIO
        ),
        None => info!("DHT11 no GPIO{}", config::DHT11_GPIO),
    }
    let (data_pin, power_pin) = config::dht11_pins(peripherals.pins);
    // Injeção de falha do C6 (`freeze_test_from_cycle`), desligada fora do experimento.
    let freeze_test = NonZeroU64::new(config::FREEZE_TEST_FROM_CYCLE).map(|from| FreezeTest {
        from: from.get(),
        reading: LocalReading::new(
            config::FREEZE_TEST_TEMPERATURE,
            config::FREEZE_TEST_HUMIDITY,
        ),
    });
    if freeze_test.is_some() {
        warn!(
            "C6: injeção de falha ligada (freeze_test_from_cycle = {}); não use este firmware \
             fora do experimento",
            config::FREEZE_TEST_FROM_CYCLE
        );
    }
    let dht11 = Dht11::new(
        PinDriver::input_output_od(data_pin, Pull::Up)?,
        power_pin.map(PinDriver::output).transpose()?,
        freeze_test,
    )?;

    // O ESP-IDF não reassocia sozinho: a cada queda, ou tentativa frustrada, uma nova
    // associação começa em segundo plano, sem depender do ciclo. A inscrição vale enquanto
    // `main` não retorna.
    let _reconnect = sysloop.subscribe::<WifiEvent, _>(|event| {
        if let WifiEvent::StaDisconnected(disconnected) = event {
            warn!(
                "Wi-Fi desconectado (motivo {}); nova associação em segundo plano",
                disconnected.reason()
            );
            unsafe { sys::esp_wifi_connect() };
        }
    })?;
    let wifi = start_wifi(EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?)?;
    let open_meteo = OpenMeteo::new(
        wifi,
        config::LATITUDE,
        config::LONGITUDE,
        Duration::from_millis(config::HTTP_TIMEOUT_MS),
    );
    wait_for_network(&open_meteo);
    dht11.wait_power_up();

    let mut bridge = SenseBridge::new(
        Timed::new(dht11),
        Timed::new(open_meteo),
        Config {
            external_refresh_ms: config::EXTERNAL_REFRESH_MS,
            ..Config::default()
        },
    );
    info!(
        "ciclo a cada {} ms, tempo limite {} ms, consulta externa a cada {} ms",
        config::CYCLE_INTERVAL_MS,
        config::HTTP_TIMEOUT_MS,
        config::EXTERNAL_REFRESH_MS
    );
    run(&mut bridge)
}

/// Configura e inicia o Wi-Fi em modo estação. A associação segue em segundo plano.
fn start_wifi(mut wifi: EspWifi<'static>) -> Result<EspWifi<'static>, EspError> {
    if config::WIFI_SSID.is_empty() {
        warn!("wifi_ssid vazio em cfg.toml: a fonte externa ficará sem conectividade");
    }
    let auth_method = if config::WIFI_PASSWORD.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };
    wifi.set_configuration(&wifi::Configuration::Client(ClientConfiguration {
        // Tamanhos validados no build.
        ssid: config::WIFI_SSID.try_into().unwrap(),
        password: config::WIFI_PASSWORD.try_into().unwrap(),
        auth_method,
        ..Default::default()
    }))?;
    wifi.start()?;
    if let Err(error) = wifi.connect() {
        warn!("falha ao iniciar a associação Wi-Fi: {error}");
    }
    Ok(wifi)
}

/// Aguarda a rede para que o primeiro ciclo não seja degradado só por causa do boot.
fn wait_for_network(open_meteo: &OpenMeteo) {
    let start = Instant::now();
    while !open_meteo.is_online() && start.elapsed() < NETWORK_WAIT {
        thread::sleep(Duration::from_millis(100));
    }
    if open_meteo.is_online() {
        info!("rede disponível após {} ms", start.elapsed().as_millis());
    } else {
        warn!(
            "rede indisponível após {} s; os ciclos começam assim mesmo",
            NETWORK_WAIT.as_secs()
        );
    }
}

/// Executa um ciclo a cada `CYCLE_INTERVAL_MS`, com cadência fixa a partir do primeiro.
fn run(bridge: &mut Bridge) -> ! {
    let interval = Duration::from_millis(config::CYCLE_INTERVAL_MS);
    let start = Instant::now();
    let mut deadline = start;
    // Suspeita de leitura congelada, opcional (`local_frozen_after`): não muda o estado.
    let mut freeze = NonZeroU32::new(config::LOCAL_FROZEN_AFTER).map(FreezeDetector::new);
    if let Some(freeze) = &freeze {
        info!(
            "suspeita de leitura congelada após {} ciclos iguais{}",
            freeze.threshold(),
            if config::LOCAL_FROZEN_RESTART {
                ", com reinício do DHT11"
            } else {
                ""
            }
        );
    }
    // Leitura que motivou o último reinício por suspeita, comparada no ciclo seguinte.
    let mut restarted_from = None;
    println!("{}", csv::HEADER);

    loop {
        let begun = Instant::now();
        let record = bridge.cycle(begun.duration_since(start).as_millis() as u64);
        let timing = Timing {
            cycle: begun.elapsed(),
            read: bridge.local_mut().take(),
            fetch: bridge.external_mut().take(),
        };
        // Fora da janela medida, como a emissão do CSV.
        if let Some(freeze) = &mut freeze {
            freeze.observe(&record.local);
        }
        let restart = bridge.local_mut().source_mut().take_restart();
        println!("{}", csv::row(&record, &timing, freeze.as_ref(), restart));
        // Fora da janela medida: o log da falha do DHT11 e, se preciso, o reinício do sensor.
        bridge.local_mut().source_mut().after_cycle();
        if let Some(freeze) = &freeze {
            restarted_from = react_to_freeze(
                freeze,
                &record.local,
                restarted_from,
                bridge.local_mut().source_mut(),
            );
        }

        deadline += interval;
        match deadline.checked_duration_since(Instant::now()) {
            Some(wait) => thread::sleep(wait),
            // Ciclo mais longo que o intervalo: o próximo começa já, sem acumular atraso.
            None => deadline = Instant::now(),
        }
    }
}

/// Reação à suspeita de leitura congelada, fora da janela medida. Registra o resultado do
/// reinício anterior, se houve, e, no ciclo em que a contagem completa um múltiplo do limiar,
/// avisa e, com `local_frozen_restart`, reinicia o DHT11. Devolve a leitura que motivou o
/// reinício, para a comparação no ciclo seguinte.
fn react_to_freeze(
    freeze: &FreezeDetector,
    local: &Result<LocalReading, LocalFailure>,
    restarted_from: Option<LocalReading>,
    dht11: &mut Dht11<'static>,
) -> Option<LocalReading> {
    if let Some(before) = restarted_from {
        match local {
            Ok(reading) if *reading == before => warn!(
                "DHT11: a leitura persistiu após o reinício ({} ciclos iguais): ambiente \
                 estável ou defeito que o reinício não resolve",
                freeze.run()
            ),
            Ok(_) => warn!("DHT11: a leitura mudou após o reinício, indício de sensor travado"),
            Err(_) => warn!("DHT11: sem leitura válida após o reinício"),
        }
    }

    let Ok(reading) = local else {
        return None;
    };
    if !freeze.reached_threshold() {
        return None;
    }
    warn!(
        "DHT11: mesma leitura há {} ciclos; suspeita de leitura congelada",
        freeze.run()
    );
    if !config::LOCAL_FROZEN_RESTART {
        return None;
    }
    dht11.power_cycle(Restart::Suspect, "suspeita de leitura congelada");
    Some(*reading)
}
