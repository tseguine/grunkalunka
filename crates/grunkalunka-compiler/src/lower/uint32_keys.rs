//! Original-source proofs for the shared unsigned computed-key representation.
use super::*;
use oxc_ast::{AstKind, ast::ForStatement};
use oxc_syntax::node::NodeId;
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct Uint32Keys {
    reads: BTreeSet<NodeId>,
    array_or_string_reads: BTreeSet<NodeId>,
}

// An AND clears the sign bit if either operand is an integer literal in
// the nonnegative i32 range. Both original operands still execute.
fn nonnegative_i32_mask(expression: &Expression<'_>, depth: usize) -> bool {
    closures::unwrapped(expression, depth)
        .and_then(static_keys::number)
        .is_some_and(|n| (0.0..=f64::from(i32::MAX)).contains(&n) && n.fract() == 0.0)
}

impl Uint32Keys {
    pub(super) fn collect(semantic: &Semantic<'_>) -> Self {
        let mut keys = Self::default();
        // Earlier declarations publish facts to their resolved reads. A chain
        // of aliases therefore needs one forward pass and no initializer replay.
        for node in semantic.nodes().iter() {
            if let AstKind::ForStatement(source) = node.kind()
                && let Some(symbol) = keys.loop_counter(source, semantic)
            {
                let body = source.body.span();
                keys.reads.extend(
                    semantic
                        .scoping()
                        .get_resolved_references(symbol)
                        .filter(|r| {
                            r.is_read()
                                && body
                                    .contains_inclusive(semantic.nodes().kind(r.node_id()).span())
                        })
                        .map(|r| r.node_id()),
                );
            }
            let AstKind::VariableDeclarator(declaration) = node.kind() else {
                continue;
            };
            let BindingPattern::BindingIdentifier(name) = &declaration.id else {
                continue;
            };
            let Some(symbol) = name.symbol_id.get() else {
                continue;
            };
            if !semantic.scoping().symbol_flags(symbol).is_const_variable() {
                continue;
            }
            let Some(init) = declaration.init.as_ref() else {
                continue;
            };
            let reads = if keys.unsigned_value(init) {
                &mut keys.reads
            } else if keys.array_or_string(init, 0) {
                &mut keys.array_or_string_reads
            } else {
                continue;
            };
            reads.extend(
                semantic
                    .scoping()
                    .get_resolved_references(symbol)
                    .filter(|reference| {
                        reference.is_read()
                            && semantic.nodes().kind(reference.node_id()).span().start
                                >= declaration.span.end
                    })
                    .map(|reference| reference.node_id()),
            );
        }
        keys
    }

    fn unsigned_value(&self, expression: &Expression<'_>) -> bool {
        closures::unwrapped(expression, 0)
            .and_then(static_keys::number)
            .is_some_and(|n| (0.0..=f64::from(u32::MAX)).contains(&n) && n.fract() == 0.0)
            || self.contains(expression, 0)
    }

    // Facts apply only inside the body, after the test succeeds. In a reverse
    // traversal the terminating test can observe -1. The counter is integral
    // throughout and its sole write is the increment or decrement in the header.
    fn loop_counter(&self, source: &ForStatement<'_>, semantic: &Semantic<'_>) -> Option<SymbolId> {
        let ForStatementInit::VariableDeclaration(declaration) = source.init.as_ref()? else {
            return None;
        };
        let [counter] = declaration.declarations.as_slice() else {
            return None;
        };
        let BindingPattern::BindingIdentifier(name) = &counter.id else {
            return None;
        };
        if declaration.kind != VariableDeclarationKind::Let {
            return None;
        }
        let init = counter.init.as_ref()?;
        let symbol = name.symbol_id.get()?;
        let Expression::BinaryExpression(test) = closures::unwrapped(source.test.as_ref()?, 0)?
        else {
            return None;
        };
        let left = closures::unwrapped(&test.left, 1)?;
        let scope = semantic.scoping();
        let limit = closures::unwrapped(&test.right, 1)?;
        let bound = static_keys::number(limit);
        let update = if let Some(step) = &source.update {
            let Expression::Identifier(left) = left else {
                return None;
            };
            if scope.get_reference(left.reference_id.get()?).symbol_id() != Some(symbol) {
                return None;
            }
            let Expression::UpdateExpression(update) = closures::unwrapped(step, 0)? else {
                return None;
            };
            match (update.operator, test.operator) {
                (UpdateOperator::Increment, BinaryOperator::LessThan)
                    if self.unsigned_value(init) && self.unsigned_value(limit) => {}
                (UpdateOperator::Decrement, BinaryOperator::GreaterEqualThan)
                    if bound == Some(0.0) && self.reverse_start(init) => {}
                (UpdateOperator::Decrement, BinaryOperator::GreaterThan)
                    if (bound == Some(-1.0) || bound == Some(0.0)) && self.reverse_start(init) => {}
                _ => return None,
            }
            update
        } else {
            // for (let i = xs.length; i-- > 0;) visits length - 1 through 0.
            let Expression::UpdateExpression(update) = left else {
                return None;
            };
            if update.prefix
                || update.operator != UpdateOperator::Decrement
                || test.operator != BinaryOperator::GreaterThan
                || bound != Some(0.0)
                || !self.unsigned_value(init)
            {
                return None;
            }
            update
        };
        let step = target_identifier(&update.argument, 0).ok()??;
        if scope.get_reference(step.reference_id.get()?).symbol_id() != Some(symbol)
            || scope
                .get_resolved_references(symbol)
                .any(|r| r.is_write() && r.node_id() != step.node_id())
        {
            return None;
        }
        Some(symbol)
    }

    fn reverse_start(&self, init: &Expression<'_>) -> bool {
        self.unsigned_value(init)
            || matches!(
                closures::unwrapped(init, 0),
                Some(Expression::BinaryExpression(sub)) if sub.operator == BinaryOperator::Subtraction
                    && closures::unwrapped(&sub.right, 1).and_then(static_keys::number) == Some(1.0)
                    && self.unsigned_value(&sub.left)
            )
    }

    // This proof runs before general field-origin analysis, which itself uses
    // unsigned keys. Only literal receivers and their immutable aliases qualify.
    fn array_or_string(&self, expression: &Expression<'_>, depth: usize) -> bool {
        closures::unwrapped(expression, depth).is_some_and(|expr| match expr {
            Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_) => true,
            Expression::Identifier(id) => self.array_or_string_reads.contains(&id.node_id()),
            _ => false,
        })
    }

    // Source operators, never annotations, prove the key's numeric range. The
    // ordinary evaluator still validates every operand and effect of these forms.
    pub(super) fn contains(&self, expression: &Expression<'_>, depth: usize) -> bool {
        let mut pending = vec![(expression, depth)];
        while let Some((expression, depth)) = pending.pop() {
            let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
                return false;
            };
            match expression {
                Expression::Identifier(id) if self.reads.contains(&id.node_id()) => {}
                Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)
                    if static_keys::length_receiver(expression, depth)
                        .is_some_and(|receiver| self.array_or_string(receiver, depth + 1)) => {}
                Expression::BinaryExpression(binary)
                    if binary.operator == BinaryOperator::ShiftRightZeroFill => {}
                Expression::BinaryExpression(binary)
                    if binary.operator == BinaryOperator::BitwiseAnd
                        && (nonnegative_i32_mask(&binary.left, depth + 1)
                            || nonnegative_i32_mask(&binary.right, depth + 1)) => {}
                Expression::SequenceExpression(sequence) => {
                    let Some(last) = sequence.expressions.last() else {
                        return false;
                    };
                    pending.push((last, depth + 1));
                }
                Expression::AssignmentExpression(assignment)
                    if assignment.operator == AssignmentOperator::Assign =>
                {
                    pending.push((&assignment.right, depth + 1))
                }
                Expression::ConditionalExpression(conditional) => {
                    pending.push((&conditional.consequent, depth + 1));
                    pending.push((&conditional.alternate, depth + 1));
                }
                _ => return false,
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn loop_counter_requires_a_bounded_test_and_a_single_increment() {
        for (header, body, admitted) in [
            ("let i=0;i<xs.length;i++", "", true),
            ("let i=1;i<3;++i", "", true),
            ("let i=xs.length-1;i>=0;--i", "", true),
            ("let i=xs.length;i-- > 0;", "", true),
            ("let i=xs.length;i-- >= 0;", "", false),
            ("let i=xs.length-1;i>=-1;i--", "", false),
            ("let i=a-1;i>=0;i--", "", false),
            ("let i=xs.length-0.5;i>=0;--i", "", false),
            ("let i=xs.length;i-- > 0;", "i=-2;", false),
            ("let i=0;i<=xs.length;i++", "", false),
            ("let i=0;i<a;i++", "", false),
            ("let i=-1;i<xs.length;i++", "", false),
            ("let i=0;i<xs.length;i--", "", false),
            ("let i=0;i<xs.length;a++", "", false),
            ("let i=0;a<xs.length;i++", "", false),
            ("let i=0;i<xs.length;i++", "i=b;", false),
            ("let i=0;i<xs.length;i++", "const f=()=>i=b;f();", false),
            ("let i=0;i<xs.length;i++", "for(i of [-1]){}", false),
        ] {
            let source = format!(
                "export function run(a,b){{const xs=[a,b];for({header}){{{body}xs[i]=1;}}return xs.length;}}"
            );
            let result = crate::compile(&source, false);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }

    #[test]
    fn length_keys_require_an_array_or_string_receiver() {
        for (body, admitted) in [
            ("const xs=[a];return Object.hasOwn(xs,xs.length)?1:0;", true),
            (
                "const s='ab',t=s;return Object.hasOwn([b],t['length'])?1:0;",
                true,
            ),
            (
                "const o={length:-1};return Object.hasOwn([b],o.length)?1:0;",
                false,
            ),
            (
                "let xs=[a];xs={length:-1};return Object.hasOwn([b],xs.length)?1:0;",
                false,
            ),
            (
                "const x=a as string;return Object.hasOwn([b],x.length)?1:0;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let result = crate::compile(&source, true);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }

    #[test]
    fn const_key_proofs_respect_bindings_and_unsigned_range() {
        for (body, admitted) in [
            ("const i=a>>>0,j=i;return Object.hasOwn([b],j)?1:0;", true),
            ("const i=4294967295;return Object.hasOwn([b],i)?1:0;", true),
            ("const i=-1;return Object.hasOwn([b],i)?1:0;", false),
            ("const i=0.5;return Object.hasOwn([b],i)?1:0;", false),
            ("const i=4294967296;return Object.hasOwn([b],i)?1:0;", false),
            ("const i=a as 0;return Object.hasOwn([b],i)?1:0;", false),
            (
                "const i=1;{const i=a;return Object.hasOwn([b],i)?1:0;}",
                false,
            ),
            ("let i=a>>>0;i=-1;return Object.hasOwn([b],i)?1:0;", false),
            ("const [i]=[-1];return Object.hasOwn([b],i)?1:0;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let result = crate::compile(&source, true);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }
}
