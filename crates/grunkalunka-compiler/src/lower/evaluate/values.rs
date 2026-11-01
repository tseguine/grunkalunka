//! Evaluation requests and typed continuation result carriers.
//!
//! These are compiler-private modes and values, not source JavaScript types
//! or a new runtime/IR representation. Only the expression work scheduler
//! owns the order in which they are created and consumed.
use super::*;

#[derive(Clone, Copy)]
pub(in crate::lower) enum Mode {
    Number,
    Predicate,
    Discard,
    // Use the common depth/aggregate guards before splitting a discarded logical.
    DiscardLogical,
    Arithmetic,
    Numeric,
    General,
    Managed,
    Object,
    Array,
    Function(usize),
    Promise,
    Pending,
    Settled,
    AsyncReturn,
    AsyncValue,
    Awaited(usize),
}

impl Mode {
    pub(super) fn managed(ty: Type) -> Self {
        match ty {
            Type::ManagedObject => Self::Object,
            Type::ManagedArray => Self::Array,
            Type::ManagedValue => Self::Managed,
            _ => unreachable!("managed result mode"),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Value {
    Number(ValueId),
    Predicate(ValueId),
    General(ValueId),
    Managed(ValueId),
    Object(ValueId),
    Array(ValueId),
    Selection(ValueId),
    Function(ValueId),
    Promise(ValueId),
    Pending(ValueId),
    Settled(ValueId),
}
impl Value {
    pub(super) fn managed(id: ValueId, ty: Type) -> Self {
        match ty {
            Type::ManagedObject => Self::Object(id),
            Type::ManagedArray => Self::Array(id),
            Type::ManagedValue => Self::Managed(id),
            _ => unreachable!("managed result value"),
        }
    }
    pub(super) fn id(self) -> ValueId {
        match self {
            Self::Number(id)
            | Self::Predicate(id)
            | Self::General(id)
            | Self::Managed(id)
            | Self::Object(id)
            | Self::Array(id)
            | Self::Selection(id)
            | Self::Function(id)
            | Self::Pending(id)
            | Self::Promise(id)
            | Self::Settled(id) => id,
        }
    }
    pub(super) fn general(self) -> ValueId {
        let Self::General(id) = self else {
            unreachable!("General continuation")
        };
        id
    }
    pub(super) fn promise(self) -> ValueId {
        let Self::Promise(id) = self else {
            unreachable!("Promise continuation")
        };
        id
    }
    pub(super) fn pending(self) -> ValueId {
        let Self::Pending(id) = self else {
            unreachable!("pending Promise continuation")
        };
        id
    }
    pub(super) fn number(self) -> ValueId {
        let Self::Number(id) = self else {
            unreachable!("Number continuation")
        };
        id
    }
    pub(super) fn predicate(self) -> ValueId {
        let Self::Predicate(id) = self else {
            unreachable!("Predicate continuation")
        };
        id
    }
}
