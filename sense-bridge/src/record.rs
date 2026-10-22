//! Registro produzido a cada ciclo.

use crate::reading::{ExternalFailure, ExternalReading, LocalFailure, LocalReading};
use crate::state::{OperationState, Transition, determine_state};

/// Leitura externa acompanhada da sua idade no momento do ciclo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalSample {
    /// Leitura aprovada na validação.
    pub reading: ExternalReading,
    /// Tempo desde a obtenção da leitura, em milissegundos.
    pub age_ms: u64,
}

/// Resultado de um ciclo de aquisição.
///
/// Cada fonte aparece como `Ok` com a leitura validada ou como `Err` com o motivo da
/// indisponibilidade. Valores reprovados na validação nunca aparecem em `Ok`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    /// Identificador sequencial do ciclo, a partir de 0.
    pub cycle: u32,
    /// Instante do ciclo informado a [`SenseBridge::cycle`](crate::SenseBridge::cycle).
    pub timestamp_ms: u64,
    /// Leitura local validada ou o motivo da indisponibilidade.
    pub local: Result<LocalReading, LocalFailure>,
    /// Leitura externa validada, com a sua idade, ou o motivo da indisponibilidade.
    pub external: Result<ExternalSample, ExternalFailure>,
    /// Presente quando o estado difere do estado do ciclo anterior.
    pub transition: Option<Transition>,
}

impl Record {
    /// Estado de operação derivado da disponibilidade das fontes neste registro.
    pub fn state(&self) -> OperationState {
        determine_state(self.local.is_ok(), self.external.is_ok())
    }
}
