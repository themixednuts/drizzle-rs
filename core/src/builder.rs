/// Marker trait for executable builder states.
///
/// This is an extension point for driver crates to opt in builder state
/// markers that represent complete, executable queries (for example, to
/// enable set operations or prepared statements on those states).
pub trait ExecutableState {}

#[derive(Debug, Clone)]
pub struct BuilderInit;

impl ExecutableState for BuilderInit {}
