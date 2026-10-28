mod external;
mod local;

mod config {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{self, EspError};
use esp_idf_svc::wifi::{self, AuthMethod, ClientConfiguration, EspWifi, WifiEvent};
use log::{info, warn};

use crate::external::OpenMeteo;
use crate::local::Dht11;

const NETWORK_WAIT: Duration = Duration::from_secs(30);

fn main() -> Result<(), EspError> {
    sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    match config::DHT11_POWER_GPIO {
        Some(power) => info!(
            "DHT11 no GPIO{}, alimentado pelo GPIO{power}",
            config::DHT11_GPIO
        ),
        None => info!("DHT11 no GPIO{}", config::DHT11_GPIO),
    }

    let (data_pin, power_pin) = config::dht11_pins(peripherals.pins);

    let dht11 = Dht11::new(
        PinDriver::input_output_od(data_pin, Pull::Up)?,
        power_pin.map(PinDriver::output).transpose()?,
    )?;

    let _reconnect = sysloop.subscribe::<WifiEvent, _>(|event| {
        if let WifiEvent::StaDisconnected(disconnected) = event {
            warn!(
                "Wi-Fi desconectado (motivo {}); nova associacao em segundo plano",
                disconnected.reason()
            );
            unsafe { sys::esp_wifi_connect() };
        }
    })?;

    let wifi = start_wifi(EspWifi::new(
        peripherals.modem,
        sysloop.clone(),
        Some(nvs),
    )?)?;

    let open_meteo = OpenMeteo::new(
        wifi,
        config::LATITUDE,
        config::LONGITUDE,
        Duration::from_millis(config::HTTP_TIMEOUT_MS),
    );

    wait_for_network(&open_meteo);
    dht11.wait_power_up();

    info!(
        "baseline: ciclo a cada {} ms, timeout {} ms",
        config::CYCLE_INTERVAL_MS,
        config::HTTP_TIMEOUT_MS
    );

    run(dht11, open_meteo)
}

fn start_wifi(mut wifi: EspWifi<'static>) -> Result<EspWifi<'static>, EspError> {
    if config::WIFI_SSID.is_empty() {
        warn!("wifi_ssid vazio em cfg.toml");
    }

    let auth_method = if config::WIFI_PASSWORD.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };

    wifi.set_configuration(&wifi::Configuration::Client(ClientConfiguration {
        ssid: config::WIFI_SSID.try_into().unwrap(),
        password: config::WIFI_PASSWORD.try_into().unwrap(),
        auth_method,
        ..Default::default()
    }))?;

    wifi.start()?;

    if let Err(error) = wifi.connect() {
        warn!("falha ao iniciar a associacao Wi-Fi: {error}");
    }

    Ok(wifi)
}

fn wait_for_network(open_meteo: &OpenMeteo) {
    let start = Instant::now();

    while !open_meteo.is_online() && start.elapsed() < NETWORK_WAIT {
        thread::sleep(Duration::from_millis(100));
    }

    if open_meteo.is_online() {
        info!("rede disponivel apos {} ms", start.elapsed().as_millis());
    } else {
        warn!(
            "rede indisponivel apos {} s; os ciclos comecam assim mesmo",
            NETWORK_WAIT.as_secs()
        );
    }
}

fn run(mut dht11: Dht11<'static>, mut open_meteo: OpenMeteo<'static>) -> ! {
    let interval = Duration::from_millis(config::CYCLE_INTERVAL_MS);
    let mut deadline = Instant::now();
    let mut cycle: u64 = 0;

    println!("cycle,read_us,fetch_us,cycle_us,heap_free,heap_min_free,stack_min_free");

    loop {
        let begun = Instant::now();

        let read_start = Instant::now();
        let local = dht11.read();
        let read_us = read_start.elapsed().as_micros();

        let fetch_start = Instant::now();
        let external = open_meteo.fetch();
        let fetch_us = fetch_start.elapsed().as_micros();

        let _ = black_box(local);
        let _ = black_box(external);

        let cycle_us = begun.elapsed().as_micros();

        let (heap_free, heap_min_free, stack_min_free) = unsafe {
            (
                sys::esp_get_free_heap_size(),
                sys::esp_get_minimum_free_heap_size(),
                sys::uxTaskGetStackHighWaterMark(core::ptr::null_mut()),
            )
        };

        println!(
            "{cycle},{read_us},{fetch_us},{cycle_us},{heap_free},{heap_min_free},{stack_min_free}"
        );

        dht11.after_cycle();
        cycle += 1;

        deadline += interval;
        match deadline.checked_duration_since(Instant::now()) {
            Some(wait) => thread::sleep(wait),
            None => deadline = Instant::now(),
        }
    }
}
