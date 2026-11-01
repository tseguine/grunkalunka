//! Shared mechanics for heap-owned source statement cursors.
//!
//! Control-specific frames remain local to their handlers. This is the common
//! enter/child request shape, admission order, and region handoff; it is not a
//! second expression evaluator or a JavaScript runtime scheduler.
use super::*;

/// A direct-nesting cursor can enter another node of its own kind or delegate
/// an ordinary child statement. Future handlers can share this request protocol
/// without sharing their distinct completion or lexical-environment semantics.
/// Source nodes reached through a control cursor were either charged by the
/// enclosing dispatcher or must be charged at this newly scheduled entry.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceCharge {
    Charged,
    Uncharged,
}

pub(super) enum StatementTask<'n, 'a, N> {
    Enter {
        source: N,
        output: Region,
        depth: usize,
        charge: SourceCharge,
    },
    Statement {
        source: &'n Statement<'a>,
        output: Region,
        depth: usize,
    },
}

impl Lowering<'_, '_> {
    /// Preserve the source-visible check order before charging a statement.
    /// Source depth and expansion limits are not native-stack or IR depth limits.
    pub(super) fn admit_source_statement(
        &mut self,
        source: &impl GetSpan,
        depth: usize,
    ) -> Result<(), String> {
        if self.active_block_calls != 0 && self.inline_depth + depth > 128 {
            return Err(unsupported(
                source,
                "block-arrow expansion exceeds 128 nested regions",
            ));
        }
        if depth > 64 {
            return Err(unsupported(source, "statement nesting exceeds 64"));
        }
        self.charge_statement(source)
    }

    /// Give a heap cursor the current output without duplicating or resetting
    /// previously emitted operations; the caller restores the completed region.
    pub(super) fn take_cursor_region(region: &mut Region) -> Region {
        Region {
            id: region.id,
            span: region.span,
            operations: std::mem::take(&mut region.operations),
            exit: std::mem::replace(&mut region.exit, Exit::Fallthrough),
        }
    }
}
