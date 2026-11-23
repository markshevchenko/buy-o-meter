use domain::{OperationId, OperationKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Feed,
    Create(OperationKind),
    Edit(OperationId),
}
