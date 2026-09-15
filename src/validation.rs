use crate::state::OperationState;

pub fn determine_state(
    local_available: bool,
    external_available: bool,
) -> OperationState {
    match (local_available, external_available) {
        (true, true) => OperationState::Normal,
        (false, true) => OperationState::DegradedLocal,
        (true, false) => OperationState::DegradedExternal,
        (false, false) => OperationState::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::OperationState;

    #[test]
    fn both_sources_available_returns_normal() {
        assert_eq!(
            determine_state(true, true),
            OperationState::Normal
        );
    }

    #[test]
    fn local_source_unavailable_returns_degraded_local() {
        assert_eq!(
            determine_state(false, true),
            OperationState::DegradedLocal
        );
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
        assert_eq!(
            determine_state(false, false),
            OperationState::Error
        );
    }
}