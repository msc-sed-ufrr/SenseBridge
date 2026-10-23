//! Ciclo de aquisição: [`SenseBridge`] e os seus parâmetros ([`Config`]).

use crate::reading::{ExternalFailure, ExternalReading};
use crate::record::{ExternalSample, Record};
use crate::source::{ExternalSource, LocalSource};
use crate::state::{OperationState, Transition, determine_state};
use crate::validation::{Limits, validate_external, validate_local};

/// Parâmetros de operação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// Faixas aceitas para a fonte local.
    pub local_limits: Limits,
    /// Faixas aceitas para a fonte externa.
    pub external_limits: Limits,
    /// Intervalo entre consultas à fonte externa, em milissegundos. Dentro dele, a última
    /// leitura externa válida é reutilizada; `0` consulta a fonte em todo ciclo.
    pub external_refresh_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            local_limits: Limits::DHT11,
            external_limits: Limits::PLAUSIBLE,
            // Intervalo fechado em 15 min: PRD, D5a.
            external_refresh_ms: 15 * 60 * 1000,
        }
    }
}

/// Integra uma fonte local e uma fonte externa, produzindo um [`Record`] por ciclo.
pub struct SenseBridge<L, E> {
    local: L,
    external: E,
    config: Config,
    next_cycle: u32,
    state: Option<OperationState>,
    last_external: Option<(ExternalReading, u64)>,
}

impl<L, E> SenseBridge<L, E> {
    /// Cria a ponte entre as duas fontes. Nenhuma delas é consultada antes do primeiro
    /// [`SenseBridge::cycle`].
    pub fn new(local: L, external: E, config: Config) -> Self {
        Self {
            local,
            external,
            config,
            next_cycle: 0,
            state: None,
            last_external: None,
        }
    }

    /// Estado do último ciclo; `None` antes do primeiro ciclo.
    pub fn state(&self) -> Option<OperationState> {
        self.state
    }

    /// Acesso à fonte local, por exemplo para alterar uma fonte simulada entre ciclos.
    pub fn local_mut(&mut self) -> &mut L {
        &mut self.local
    }

    /// Acesso à fonte externa, por exemplo para alterar uma fonte simulada entre ciclos.
    pub fn external_mut(&mut self) -> &mut E {
        &mut self.external
    }
}

impl<L: LocalSource, E: ExternalSource> SenseBridge<L, E> {
    /// Executa um ciclo de aquisição no instante `now_ms` de um relógio monotônico.
    pub fn cycle(&mut self, now_ms: u64) -> Record {
        let local = self
            .local
            .read()
            .and_then(|reading| validate_local(reading, &self.config.local_limits));
        let external = self.acquire_external(now_ms);

        let state = determine_state(local.is_ok(), external.is_ok());
        let transition = self
            .state
            .filter(|&previous| previous != state)
            .map(|from| Transition { from, to: state });
        self.state = Some(state);

        let cycle = self.next_cycle;
        self.next_cycle += 1;

        Record {
            cycle,
            timestamp_ms: now_ms,
            local,
            external,
            transition,
        }
    }

    fn acquire_external(&mut self, now_ms: u64) -> Result<ExternalSample, ExternalFailure> {
        if let Some((reading, obtained_at)) = self.last_external {
            let age_ms = now_ms.saturating_sub(obtained_at);
            if age_ms < self.config.external_refresh_ms {
                return Ok(ExternalSample { reading, age_ms });
            }
        }

        // Uma consulta que falha descarta a leitura anterior, para que um valor obtido
        // antes da falha nunca seja apresentado como atual.
        let result = self
            .external
            .fetch()
            .and_then(|reading| validate_external(reading, &self.config.external_limits));
        self.last_external = result.ok().map(|reading| (reading, now_ms));
        result.map(|reading| ExternalSample { reading, age_ms: 0 })
    }
}
