//! Physical value encodings belong to the Wasm backend. Source types and the
//! verifier describe logical values; emitters share this policy for stack lanes,
//! locals, suspended storage, captured cells and closure argument frames.
use crate::ir::Type;

#[derive(Clone, Copy)]
pub(super) enum Lane {
    I32,
    F64,
}

impl Lane {
    pub(super) fn wat(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::F64 => "f64",
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
        Type::Number => Layout {
            parts: &[Part {
                lane: F64,
                byte_offset: 0,
            }],
            wat: "f64",
            local: "(local f64)\n",
        },
        Type::Predicate => Layout {
            parts: &[Part {
                lane: I32,
                byte_offset: 0,
            }],
            wat: "i32",
            local: "(local i32)\n",
        },
        Type::General | Type::FulfilledPromise => Layout {
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
        },
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

// Argument entries are storage contracts, distinct from captured-cell allocation.
pub(super) fn argument_bytes(ty: Type) -> usize {
    match ty {
        Type::Number | Type::Closure(_) => 8,
        Type::General => 16,
        Type::Predicate | Type::FulfilledPromise => {
            unreachable!("Predicate and Promise closure parameters are not admitted")
        }
    }
}
