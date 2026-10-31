//! Physical value encodings belong to the Wasm backend. Source types and the
//! verifier describe logical values; emitters share this policy for stack lanes,
//! locals, suspended storage, captured cells and closure argument frames.
use crate::ir::Type;

// FulfillPromise changes the logical kind of an immutable SSA pair, not its
// physical bits. Sharing value storage preserves later uses of both values;
// mutable source slots always retain their separate storage.
pub(super) fn share_fulfilled_storage(function: &crate::ir::Function, offsets: &mut [usize]) {
    crate::ir::visit_operations(&function.body, &mut |op| {
        if let crate::ir::Op::FulfillPromise { result, input } = op {
            offsets[result.0] = offsets[input.0];
        }
    });
}

// Settled snapshots share General's tag/payload lanes. Reserve a status bit
// outside the canonical value-kind range so Error tag 4 remains a value kind.
pub(super) const SETTLED_REJECTED_BIT: u32 = 1 << 31;
pub(super) const SETTLED_TAG_MASK: u32 = !SETTLED_REJECTED_BIT;

// Selection carrier and opt-in Promise rejection reasons. Ordinary General
// fulfillment values retain kinds 0..=4 and adopt Promises in the guest.
pub(super) const ASYNC_PROMISE_TAG: u32 = 5;

#[derive(Clone, Copy)]
pub(super) enum Lane {
    I32,
    F64,
    ManagedRef,
    ObjectRef,
    ArrayRef,
    EnvironmentRef,
}

impl Lane {
    pub(super) fn wat(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::F64 => "f64",
            Self::ManagedRef => "(ref null $gc_value)",
            Self::ObjectRef => "(ref null $gc_object)",
            Self::ArrayRef => "(ref null $gc_array)",
            Self::EnvironmentRef => "(ref null $gc_frame)",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Part {
    pub lane: Lane,
    pub byte_offset: usize,
}

#[derive(Clone, Copy)]
pub(super) struct Layout {
    pub parts: &'static [Part],
    pub wat: &'static str,
    pub local: &'static str,
}

pub(super) fn value(ty: Type) -> Layout {
    use Lane::{F64, I32};
    match ty {
        Type::ManagedArray => Layout {
            parts: &[Part {
                lane: Lane::ArrayRef,
                byte_offset: 0,
            }],
            wat: "(ref null $gc_array)",
            local: "(local (ref null $gc_array))\n",
        },
        Type::ManagedObject => Layout {
            parts: &[Part {
                lane: Lane::ObjectRef,
                byte_offset: 0,
            }],
            wat: "(ref null $gc_object)",
            local: "(local (ref null $gc_object))\n",
        },
        Type::ManagedValue => Layout {
            parts: &[Part {
                lane: Lane::ManagedRef,
                byte_offset: 0,
            }],
            wat: "(ref null $gc_value)",
            local: "(local (ref null $gc_value))\n",
        },
        Type::Number => Layout {
            parts: &[Part {
                lane: F64,
                byte_offset: 0,
            }],
            wat: "f64",
            local: "(local f64)\n",
        },
        Type::Predicate | Type::Promise => Layout {
            parts: &[Part {
                lane: I32,
                byte_offset: 0,
            }],
            wat: "i32",
            local: "(local i32)\n",
        },
        Type::General | Type::FulfilledPromise | Type::SettledPromise | Type::AsyncValue => {
            Layout {
                parts: &[
                    Part {
                        lane: I32,
                        byte_offset: 0,
                    },
                    Part {
                        lane: F64,
                        byte_offset: 8,
                    },
                ],
                wat: "i32 f64",
                local: "(local i32 f64)\n",
            }
        }
        Type::Closure(_) => Layout {
            parts: &[
                Part {
                    lane: I32,
                    byte_offset: 0,
                },
                Part {
                    lane: I32,
                    byte_offset: 4,
                },
            ],
            wat: "i32 i32",
            local: "(local i32 i32)\n",
        },
    }
}

// Only modules with traced captures change their private closure layout.
pub(super) fn closure_value(ty: Type, gc: bool) -> Layout {
    if gc && matches!(ty, Type::Closure(_)) {
        Layout {
            parts: &[
                Part {
                    lane: Lane::I32,
                    byte_offset: 0,
                },
                Part {
                    lane: Lane::EnvironmentRef,
                    byte_offset: 0,
                },
            ],
            wat: "i32 (ref null $gc_frame)",
            local: "(local i32 (ref null $gc_frame))\n",
        }
    } else {
        value(ty)
    }
}

// Argument entries are storage contracts, distinct from captured-cell allocation.
pub(super) fn argument_bytes(ty: Type) -> usize {
    match ty {
        Type::Number | Type::Closure(_) | Type::Promise => 8,
        Type::General | Type::FulfilledPromise | Type::SettledPromise | Type::AsyncValue => 16,
        Type::Predicate => unreachable!("Predicate closure parameters are not admitted"),
        Type::ManagedValue | Type::ManagedObject | Type::ManagedArray => {
            unreachable!("managed closure parameters need GC frames")
        }
    }
}
