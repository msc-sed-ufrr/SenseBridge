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

impl InvalidReason {
    /// Nome estável do motivo, para serializar registros. Ao contrário da saída de `Debug`,
    /// faz parte da API: mudar um nome muda o formato dos registros já gravados.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotANumber => "NotANumber",
            Self::OutOfRange => "OutOfRange",
        }
    }
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

impl LocalFailure {
    /// Nome estável da falha, para serializar registros; a reprovação traz o motivo, como em
    /// `Invalid(OutOfRange)`. Faz parte da API, ao contrário da saída de `Debug`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoResponse => "NoResponse",
            Self::Checksum => "Checksum",
            Self::Invalid(InvalidReason::NotANumber) => "Invalid(NotANumber)",
            Self::Invalid(InvalidReason::OutOfRange) => "Invalid(OutOfRange)",
        }
    }
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

impl ExternalFailure {
    /// Nome estável da falha, para serializar registros; a reprovação traz o motivo, como em
    /// `Invalid(OutOfRange)`. Faz parte da API, ao contrário da saída de `Debug`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoConnectivity => "NoConnectivity",
            Self::Timeout => "Timeout",
            Self::ServiceError => "ServiceError",
            Self::MalformedResponse => "MalformedResponse",
            Self::Invalid(InvalidReason::NotANumber) => "Invalid(NotANumber)",
            Self::Invalid(InvalidReason::OutOfRange) => "Invalid(OutOfRange)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_names_are_stable() {
        assert_eq!(InvalidReason::OutOfRange.as_str(), "OutOfRange");
        assert_eq!(LocalFailure::NoResponse.as_str(), "NoResponse");
        assert_eq!(LocalFailure::Checksum.as_str(), "Checksum");
        assert_eq!(
            LocalFailure::Invalid(InvalidReason::NotANumber).as_str(),
            "Invalid(NotANumber)"
        );
        assert_eq!(ExternalFailure::NoConnectivity.as_str(), "NoConnectivity");
        assert_eq!(ExternalFailure::Timeout.as_str(), "Timeout");
        assert_eq!(ExternalFailure::ServiceError.as_str(), "ServiceError");
        assert_eq!(
            ExternalFailure::MalformedResponse.as_str(),
            "MalformedResponse"
        );
        assert_eq!(
            ExternalFailure::Invalid(InvalidReason::OutOfRange).as_str(),
            "Invalid(OutOfRange)"
        );
    }
}
