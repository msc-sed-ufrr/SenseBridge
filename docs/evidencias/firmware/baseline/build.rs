//! Prepara o ambiente do ESP-IDF e gera `config.rs` a partir de `cfg.toml`.
//!
//! As credenciais do Wi-Fi e os parâmetros do experimento ficam em `cfg.toml`, fora do
//! controle de versão. Os valores são validados aqui, de modo que uma configuração inválida
//! falha no build, e não no dispositivo.

use std::{env, fs, path::Path};

use toml::{Table, Value};

const CFG_FILE: &str = "cfg.toml";

/// Intervalo mínimo de amostragem do DHT11, segundo o manual do sensor.
const DHT11_MIN_SAMPLING_MS: i64 = 1000;

/// GPIOs do ESP32 que servem ao DHT11: bidirecionais, com pull-up interno, fora da Flash
/// (6–11) e da UART0 (1 e 3), que transporta o CSV. Os de 34 a 39 são só de entrada. Os
/// mesmos servem à alimentação do sensor.
const DHT11_GPIOS: [i64; 20] = [
    0, 2, 4, 5, 12, 13, 14, 15, 16, 17, 18, 19, 21, 22, 23, 25, 26, 27, 32, 33,
];

/// Pinos de strapping: o nível no reset escolhe o modo de boot ou a tensão da Flash, e o
/// pull-up do DHT11 pode impedir a gravação ou o boot.
const STRAPPING_GPIOS: [i64; 5] = [0, 2, 5, 12, 15];

fn main() {
    embuild::espidf::sysenv::output();

    println!("cargo:rerun-if-changed={CFG_FILE}");
    let table = match fs::read_to_string(CFG_FILE) {
        Ok(text) => text
            .parse::<Table>()
            .unwrap_or_else(|error| fail(&format!("{CFG_FILE} inválido: {error}"))),
        Err(_) => {
            println!(
                "cargo:warning={CFG_FILE} ausente: usando valores padrão e SSID vazio; \
                 copie cfg.toml.example para configurar o Wi-Fi"
            );
            Table::new()
        }
    };

    let config = Config::from_table(table);
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("config.rs");
    fs::write(out, config.to_rust()).unwrap();
}

struct Config {
    wifi_ssid: String,
    wifi_password: String,
    latitude: f64,
    longitude: f64,
    cycle_interval_ms: i64,
    http_timeout_ms: i64,
    external_refresh_ms: i64,
    dht11_gpio: i64,
    dht11_power_gpio: Option<i64>,
}

impl Config {
    fn from_table(mut table: Table) -> Self {
        let mut take = |key: &str| table.remove(key);

        let config = Self {
            wifi_ssid: string(take("wifi_ssid"), "wifi_ssid", ""),
            wifi_password: string(take("wifi_password"), "wifi_password", ""),
            latitude: float(take("latitude"), "latitude", 2.82),
            longitude: float(take("longitude"), "longitude", -60.67),
            cycle_interval_ms: integer(take("cycle_interval_ms"), "cycle_interval_ms", 5000),
            http_timeout_ms: integer(take("http_timeout_ms"), "http_timeout_ms", 3000),
            external_refresh_ms: integer(
                take("external_refresh_ms"),
                "external_refresh_ms",
                15 * 60 * 1000,
            ),
            dht11_gpio: integer(take("dht11_gpio"), "dht11_gpio", 4),
            dht11_power_gpio: take("dht11_power_gpio").map(|value| match value {
                Value::Integer(number) => number,
                _ => fail("dht11_power_gpio deve ser um inteiro"),
            }),
        };

        if let Some(key) = table.keys().next() {
            fail(&format!("chave desconhecida em {CFG_FILE}: {key}"));
        }
        config.validate();
        config
    }

    fn validate(&self) {
        if self.wifi_ssid.len() > 32 {
            fail("wifi_ssid tem mais de 32 bytes");
        }
        if self.wifi_password.len() > 64 {
            fail("wifi_password tem mais de 64 bytes");
        }
        if !(-90.0..=90.0).contains(&self.latitude) {
            fail("latitude fora de [-90, 90]");
        }
        if !(-180.0..=180.0).contains(&self.longitude) {
            fail("longitude fora de [-180, 180]");
        }
        if self.cycle_interval_ms < DHT11_MIN_SAMPLING_MS {
            fail(&format!(
                "cycle_interval_ms deve ser de pelo menos {DHT11_MIN_SAMPLING_MS} ms, \
                 o intervalo de amostragem do DHT11"
            ));
        }
        if self.http_timeout_ms <= 0 || self.http_timeout_ms >= self.cycle_interval_ms {
            fail("http_timeout_ms deve ser positivo e menor que cycle_interval_ms");
        }
        if self.external_refresh_ms < 0 {
            fail("external_refresh_ms não pode ser negativo");
        }
        if !DHT11_GPIOS.contains(&self.dht11_gpio) {
            fail(&format!(
                "dht11_gpio deve ser um destes GPIOs: {DHT11_GPIOS:?}"
            ));
        }
        if STRAPPING_GPIOS.contains(&self.dht11_gpio) {
            println!(
                "cargo:warning=dht11_gpio = {} é pino de strapping: se a gravação ou o boot \
                 falharem, desconecte o DATA do DHT11 durante o reset ou escolha outro GPIO",
                self.dht11_gpio
            );
        }
        if let Some(power) = self.dht11_power_gpio {
            if !DHT11_GPIOS.contains(&power) {
                fail(&format!(
                    "dht11_power_gpio deve ser um destes GPIOs: {DHT11_GPIOS:?}"
                ));
            }
            if power == self.dht11_gpio {
                fail("dht11_power_gpio deve ser diferente de dht11_gpio");
            }
            if STRAPPING_GPIOS.contains(&power) {
                println!(
                    "cargo:warning=dht11_power_gpio = {power} é pino de strapping: se a \
                     gravação ou o boot falharem, desconecte o VCC do DHT11 durante o reset ou \
                     escolha outro GPIO"
                );
            }
        }
    }

    fn to_rust(&self) -> String {
        format!(
            "pub const WIFI_SSID: &str = {:?};\n\
             pub const WIFI_PASSWORD: &str = {:?};\n\
             pub const LATITUDE: f32 = {:?};\n\
             pub const LONGITUDE: f32 = {:?};\n\
             pub const CYCLE_INTERVAL_MS: u64 = {};\n\
             pub const HTTP_TIMEOUT_MS: u64 = {};\n\
             pub const EXTERNAL_REFRESH_MS: u64 = {};\n\
             pub const DHT11_GPIO: u8 = {gpio};\n\
             pub const DHT11_POWER_GPIO: Option<u8> = {power:?};\n\
             pub fn dht11_pins(pins: esp_idf_svc::hal::gpio::Pins) -> (\
             esp_idf_svc::hal::gpio::AnyIOPin<'static>, \
             Option<esp_idf_svc::hal::gpio::AnyOutputPin<'static>>) \
             {{ (pins.gpio{gpio}.into(), {power_pin}) }}\n",
            self.wifi_ssid,
            self.wifi_password,
            self.latitude as f32,
            self.longitude as f32,
            self.cycle_interval_ms,
            self.http_timeout_ms,
            self.external_refresh_ms,
            gpio = self.dht11_gpio,
            power = self.dht11_power_gpio,
            power_pin = self.dht11_power_gpio.map_or_else(
                || "None".to_owned(),
                |gpio| format!("Some(pins.gpio{gpio}.into())")
            ),
        )
    }
}

fn string(value: Option<Value>, key: &str, default: &str) -> String {
    match value {
        None => default.to_owned(),
        Some(Value::String(text)) => text,
        Some(_) => fail(&format!("{key} deve ser um texto")),
    }
}

fn float(value: Option<Value>, key: &str, default: f64) -> f64 {
    match value {
        None => default,
        Some(Value::Float(number)) => number,
        Some(Value::Integer(number)) => number as f64,
        Some(_) => fail(&format!("{key} deve ser um número")),
    }
}

fn integer(value: Option<Value>, key: &str, default: i64) -> i64 {
    match value {
        None => default,
        Some(Value::Integer(number)) => number,
        Some(_) => fail(&format!("{key} deve ser um inteiro")),
    }
}

fn fail(message: &str) -> ! {
    eprintln!("erro de configuração: {message}");
    std::process::exit(1);
}
