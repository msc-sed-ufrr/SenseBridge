//! Fonte externa: API Open-Meteo consultada por HTTP sobre Wi-Fi.

use std::time::{Duration, Instant};

use esp_idf_svc::http::Method;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::sys::{ESP_ERR_HTTP_EAGAIN, ESP_ERR_TIMEOUT, EspError};
use esp_idf_svc::wifi::EspWifi;
use log::warn;
use sense_bridge::{ExternalFailure, ExternalReading, ExternalSource, open_meteo};

/// Maior corpo de resposta aceito. A resposta das condições atuais tem cerca de 400 bytes.
const MAX_BODY: usize = 1024;

/// Consulta a Open-Meteo pela rede Wi-Fi, aplicando o tempo limite à consulta inteira.
///
/// A interpretação da resposta fica na biblioteca (`sense_bridge::open_meteo`); aqui ficam
/// apenas a conectividade e o transporte.
pub struct OpenMeteo<'d> {
    wifi: EspWifi<'d>,
    url: String,
    timeout: Duration,
}

impl<'d> OpenMeteo<'d> {
    pub fn new(wifi: EspWifi<'d>, latitude: f32, longitude: f32, timeout: Duration) -> Self {
        let mut url = format!("http://{}", open_meteo::HOST);
        open_meteo::write_path(&mut url, latitude, longitude).expect("escrita em String");
        Self { wifi, url, timeout }
    }

    pub fn is_online(&self) -> bool {
        self.wifi.is_up().unwrap_or(false)
    }

    /// Faz o GET e copia o corpo para `body`, devolvendo o status e o tamanho lido.
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
}

impl ExternalSource for OpenMeteo<'_> {
    fn fetch(&mut self) -> Result<ExternalReading, ExternalFailure> {
        if !self.is_online() {
            // Sem rede, a consulta falha no próprio ciclo. A reassociação segue em segundo
            // plano, e a fonte volta no primeiro ciclo depois dela.
            if !self.wifi.is_connected().unwrap_or(false) {
                self.wifi.connect().ok();
            }
            return Err(ExternalFailure::NoConnectivity);
        }

        let start = Instant::now();
        let mut body = [0; MAX_BODY];
        let result = self.get(&mut body);

        // O `timeout_ms` do ESP-IDF vale por operação de socket; o tempo limite do PRD vale
        // para a consulta inteira, e uma resposta que chega depois dele não é usada.
        if start.elapsed() > self.timeout {
            return Err(ExternalFailure::Timeout);
        }
        match result {
            Ok((status, len)) => open_meteo::interpret(status, &body[..len]),
            Err(error) => {
                warn!("consulta à Open-Meteo falhou: {error}");
                Err(classify(error))
            }
        }
    }
}

/// Converte um erro de transporte do cliente HTTP em tipo de falha externa.
///
/// Erros de leitura chegam com o código negativo (`-ESP_ERR_HTTP_EAGAIN`); os de conexão, com o
/// código positivo.
fn classify(error: EspError) -> ExternalFailure {
    match error.code().wrapping_abs() {
        ESP_ERR_HTTP_EAGAIN | ESP_ERR_TIMEOUT => ExternalFailure::Timeout,
        // DNS, conexão TCP recusada ou interrompida: o serviço não foi alcançado.
        _ => ExternalFailure::NoConnectivity,
    }
}
