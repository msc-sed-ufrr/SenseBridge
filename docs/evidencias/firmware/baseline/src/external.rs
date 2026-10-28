use std::fmt::Write;
use std::hint::black_box;
use std::time::{Duration, Instant};

use esp_idf_svc::http::Method;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::sys::{ESP_ERR_HTTP_EAGAIN, ESP_ERR_TIMEOUT, EspError};
use esp_idf_svc::wifi::EspWifi;
use log::warn;

const HOST: &str = "api.open-meteo.com";
const MAX_BODY: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub enum ExternalFailure {
    NoConnectivity,
    Timeout,
    ServiceError,
}

pub struct OpenMeteo<'d> {
    wifi: EspWifi<'d>,
    url: String,
    timeout: Duration,
}

impl<'d> OpenMeteo<'d> {
    pub fn new(wifi: EspWifi<'d>, latitude: f32, longitude: f32, timeout: Duration) -> Self {
        let mut url = format!("http://{HOST}");
        write!(
            &mut url,
            "/v1/forecast?latitude={latitude}&longitude={longitude}\
             &current=temperature_2m,relative_humidity_2m"
        )
        .expect("escrita em String");

        Self { wifi, url, timeout }
    }

    pub fn is_online(&self) -> bool {
        self.wifi.is_up().unwrap_or(false)
    }

    fn get(&self, body: &mut [u8]) -> Result<(u16, usize), EspError> {
        let mut connection = EspHttpConnection::new(&Configuration {
            timeout: Some(self.timeout),
            ..Default::default()
        })?;

        connection.initiate_request(Method::Get, &self.url, &[])?;
        connection.initiate_response()?;

        let status = connection.status();
        let mut len = 0;

        while len < body.len() {
            match connection.read(&mut body[len..])? {
                0 => break,
                read => len += read,
            }
        }

        Ok((status, len))
    }

    pub fn fetch(&mut self) -> Result<(), ExternalFailure> {
        if !self.is_online() {
            return Err(ExternalFailure::NoConnectivity);
        }

        let start = Instant::now();
        let mut body = [0; MAX_BODY];
        let result = self.get(&mut body);

        if start.elapsed() > self.timeout {
            return Err(ExternalFailure::Timeout);
        }

        match result {
            Ok((status, len)) => {
                let _ = black_box((status, &body[..len]));

                if status == 200 {
                    Ok(())
                } else {
                    Err(ExternalFailure::ServiceError)
                }
            }
            Err(error) => {
                warn!("consulta a Open-Meteo falhou: {error}");
                Err(classify(error))
            }
        }
    }
}

fn classify(error: EspError) -> ExternalFailure {
    match error.code().wrapping_abs() {
        ESP_ERR_HTTP_EAGAIN | ESP_ERR_TIMEOUT => ExternalFailure::Timeout,
        _ => ExternalFailure::NoConnectivity,
    }
}
