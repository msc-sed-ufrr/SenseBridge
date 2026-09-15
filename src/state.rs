#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    Normal,
    DegradedLocal,
    DegradedExternal,
    Error,
}