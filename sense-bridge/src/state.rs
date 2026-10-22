/// Estado de operação determinado pela disponibilidade das fontes em um ciclo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    /// Ambas as fontes disponíveis.
    Normal,
    /// Fonte local indisponível; apenas a externa fornece dados.
    DegradedLocal,
    /// Fonte externa indisponível; apenas a local fornece dados.
    DegradedExternal,
    /// Nenhuma fonte disponível.
    Error,
}

impl OperationState {
    /// Nome estável do estado, para serializar registros. Ao contrário da saída de `Debug`,
    /// faz parte da API: mudar um nome muda o formato dos registros já gravados.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::DegradedLocal => "DegradedLocal",
            Self::DegradedExternal => "DegradedExternal",
            Self::Error => "Error",
        }
    }
}

pub fn determine_state(local_available: bool, external_available: bool) -> OperationState {
    match (local_available, external_available) {
        (true, true) => OperationState::Normal,
        (false, true) => OperationState::DegradedLocal,
        (true, false) => OperationState::DegradedExternal,
        (false, false) => OperationState::Error,
    }
}

/// Mudança de estado entre dois ciclos consecutivos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub from: OperationState,
    pub to: OperationState,
}

impl Transition {
    /// Retorno à operação normal a partir de um estado degradado ou de erro.
    pub fn is_recovery(&self) -> bool {
        self.from != OperationState::Normal && self.to == OperationState::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sources_available_returns_normal() {
        assert_eq!(determine_state(true, true), OperationState::Normal);
    }

    #[test]
    fn local_source_unavailable_returns_degraded_local() {
        assert_eq!(determine_state(false, true), OperationState::DegradedLocal);
    }

    #[test]
    fn external_source_unavailable_returns_degraded_external() {
        assert_eq!(
            determine_state(true, false),
            OperationState::DegradedExternal
        );
    }

    #[test]
    fn both_sources_unavailable_returns_error() {
        assert_eq!(determine_state(false, false), OperationState::Error);
    }

    #[test]
    fn state_names_are_stable() {
        assert_eq!(OperationState::Normal.as_str(), "Normal");
        assert_eq!(OperationState::DegradedLocal.as_str(), "DegradedLocal");
        assert_eq!(
            OperationState::DegradedExternal.as_str(),
            "DegradedExternal"
        );
        assert_eq!(OperationState::Error.as_str(), "Error");
    }

    #[test]
    fn only_transitions_into_normal_are_recoveries() {
        use OperationState::*;

        for from in [DegradedLocal, DegradedExternal, Error] {
            assert!(Transition { from, to: Normal }.is_recovery());
            assert!(
                !Transition {
                    from: Normal,
                    to: from
                }
                .is_recovery()
            );
        }
        assert!(
            !Transition {
                from: Error,
                to: DegradedLocal
            }
            .is_recovery()
        );
    }
}
