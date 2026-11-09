//! Original-source proofs for the shared unsigned computed-key representation.
use super::*;
use oxc_ast::AstKind;
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
            let literal = closures::unwrapped(init, 0).and_then(static_keys::number);
            let reads = if literal
                .is_some_and(|n| (0.0..=f64::from(u32::MAX)).contains(&n) && n.fract() == 0.0)
                || keys.contains(init, 0)
            {
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
