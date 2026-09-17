/// Temperatura (°C) e umidade relativa (%) obtidas da fonte local (DHT11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalReading {
    pub temperature: f32,
    pub humidity: f32,
}

impl LocalReading {
    pub const fn new(temperature: f32, humidity: f32) -> Self {
        Self {
            temperature,
            humidity,
        }
    }
}

/// Temperatura (°C) e umidade relativa (%) obtidas da fonte externa (Open-Meteo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalReading {
    pub temperature: f32,
    pub humidity: f32,
}

impl ExternalReading {
    pub const fn new(temperature: f32, humidity: f32) -> Self {
        Self {
            temperature,
            humidity,
        }
    }
}

/// Motivo pelo qual uma leitura foi reprovada na validação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidReason {
    /// Valor NaN ou infinito.
    NotANumber,
    /// Valor fora da faixa configurada.
    OutOfRange,
}

/// Motivo pelo qual a fonte local não forneceu leitura válida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFailure {
    /// O sensor não respondeu.
    NoResponse,
    /// A leitura chegou com erro de integridade (checksum).
    Checksum,
    /// A leitura foi obtida, mas reprovada na validação.
    Invalid(InvalidReason),
}

/// Motivo pelo qual a fonte externa não forneceu leitura válida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalFailure {
    /// Sem conexão de rede.
    NoConnectivity,
    /// A consulta excedeu o tempo limite.
    Timeout,
    /// O serviço respondeu com erro.
    ServiceError,
    /// A resposta não pôde ser interpretada ou não contém temperatura e umidade.
    MalformedResponse,
    /// A leitura foi obtida, mas reprovada na validação.
    Invalid(InvalidReason),
}
