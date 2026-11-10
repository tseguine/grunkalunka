//! String names proven to miss the pristine Object prototype.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct RecordNames {
    reads: BTreeSet<NodeId>,
}

impl RecordNames {
    pub(super) fn contains(&self, expression: &Expression<'_>, depth: usize) -> bool {
        name_from(expression, depth, |id| self.reads.contains(&id.node_id()))
    }

    pub(super) fn collect(semantic: &Semantic<'_>) -> Self {
        let scope = semantic.scoping();
        let nodes = semantic.nodes();
        let mut candidates = BTreeSet::new();
        let mut users: BTreeMap<SymbolId, Vec<SymbolId>> = BTreeMap::new();
        for node in nodes.iter() {
            let AstKind::VariableDeclarator(decl) = node.kind() else {
                continue;
            };
            let AstKind::VariableDeclaration(parent) = nodes.parent_kind(node.id()) else {
                continue;
            };
            let BindingPattern::BindingIdentifier(binding) = &decl.id else {
                continue;
            };
            let (Some(init), Some(symbol)) = (decl.init.as_ref(), binding.symbol_id.get()) else {
                continue;
            };
            if !matches!(
                parent.kind,
                VariableDeclarationKind::Let | VariableDeclarationKind::Const
            ) {
                continue;
            }

            let mut dependencies = BTreeSet::new();
            let mut read = |id: &IdentifierReference<'_>| {
                let Some(dep) = id
                    .reference_id
                    .get()
                    .and_then(|r| scope.get_reference(r).symbol_id())
                else {
                    return false;
                };
                if semantic.symbol_declaration(dep).kind().span().end > id.span.start {
                    return false;
                }
                dependencies.insert(dep);
                true
            };
            if !name_from(init, 0, &mut read) {
                continue;
            }
            // Every write, including one inside a closure or iteration head,
            // must be a simple assignment whose RHS also has a safe name.
            if !scope.get_resolved_references(symbol).filter(|r|r.is_write()).all(|r|{
                let AstKind::AssignmentExpression(assign)=nodes.parent_kind(r.node_id()) else {return false};
                assign.operator==AssignmentOperator::Assign
                    && matches!(assign.left.as_simple_assignment_target(),Some(SimpleAssignmentTarget::AssignmentTargetIdentifier(id)) if id.node_id()==r.node_id())
                    && name_from(&assign.right,0,&mut read)
            }) {continue}
            candidates.insert(symbol);
            for dep in dependencies {
                users.entry(dep).or_default().push(symbol);
            }
        }
        // Invalidate dependent bindings via reverse edges. Cycles of safe
        // copies retain their invariant because each initializer was checked.
        let mut invalid = users
            .keys()
            .filter(|id| !candidates.contains(id))
            .copied()
            .collect::<Vec<_>>();
        while let Some(dep) = invalid.pop() {
            for &user in users.get(&dep).into_iter().flatten() {
                if candidates.remove(&user) {
                    invalid.push(user);
                }
            }
        }
        let mut reads = BTreeSet::new();
        for symbol in candidates {
            let end = semantic.symbol_declaration(symbol).kind().span().end;
            reads.extend(
                scope
                    .get_resolved_references(symbol)
                    .filter(|r| r.is_read() && nodes.kind(r.node_id()).span().start >= end)
                    .map(|r| r.node_id()),
            );
        }
        Self { reads }
    }
}

// Keep the prototype names consistent with each fixed fragment's position.
// Unknown operands may contribute any String. A literal below an operator
// other than concatenation provides no evidence about the resulting name.
fn constructed_own_name(expression: &Expression<'_>, depth: usize) -> bool {
    let mut candidates = crate::prototype_keys::OBJECT.to_vec();
    let mut pending = vec![(expression, depth, true, true)];
    while let Some((node, depth, first, last)) = pending.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        let mut fragment = |text: Option<&str>, prefix: bool, suffix: bool| {
            candidates.retain(|name| {
                text.is_some_and(|text| match (prefix, suffix) {
                    (true, true) => *name == text,
                    (true, false) => name.starts_with(text),
                    (false, true) => name.ends_with(text),
                    (false, false) => name.contains(text),
                })
            });
        };
        match node {
            Expression::StringLiteral(s) => fragment(s.value.as_str(), first, last),
            Expression::TemplateLiteral(t) => {
                for (i, quasi) in t.quasis.iter().enumerate() {
                    let Some(cooked) = &quasi.value.cooked else {
                        return false;
                    };
                    fragment(
                        cooked.as_str(),
                        first && i == 0,
                        last && i + 1 == t.quasis.len(),
                    );
                }
            }
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                pending.push((&b.right, depth + 1, false, last));
                pending.push((&b.left, depth + 1, first, false));
            }
            _ => {}
        }
        if candidates.is_empty() {
            return true;
        }
    }
    false
}

// A plain record can use its own-property table when every possible String
// key misses the pristine Object prototype. Validate operands separately;
// this walk only describes the result of a successful key evaluation.
fn name_from<'a>(
    expression: &Expression<'a>,
    depth: usize,
    mut valid_read: impl FnMut(&IdentifierReference<'a>) -> bool,
) -> bool {
    let mut pending = vec![(expression, depth)];
    while let Some((node, depth)) = pending.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        match node {
            Expression::StringLiteral(_) => {
                let Some(name) = static_keys::literal(node) else {
                    return false;
                };
                if crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &name) {
                    return false;
                }
            }
            Expression::TemplateLiteral(_) | Expression::BinaryExpression(_)
                if constructed_own_name(node, depth) => {}
            Expression::ConditionalExpression(c) => {
                pending.push((&c.consequent, depth + 1));
                pending.push((&c.alternate, depth + 1));
            }
            Expression::SequenceExpression(seq) => {
                let Some(last) = seq.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::Identifier(id) if valid_read(id) => {}
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_initializer_and_write_must_preserve_record_names() {
        for (bindings, admitted) in [
            ("let k='x';if(a)k='y';", true),
            ("let k='x',j=k;k=j;j=k;", true),
            ("let k='x';const change=()=>k='y';change();", true),
            ("let k='x';k='constructor';", false),
            ("let k='x',j=k;k=j;j='constructor';", false),
            ("let k='x';const change=()=>k='__proto__';", false),
            ("let k='x';[k]=['toString'];", false),
            ("let k='x';for(k of ['toString']){}", false),
            ("let k='x';for(k in {constructor:1}){}", false),
            ("let k='to';k+='String';", false),
            ("let k=a as 'x';", false),
        ] {
            let source = format!(
                "export function run(a,b){{{bindings}const o={{x:b}};return o[k]===b?1:0;}}"
            );
            let result = crate::compile(&source, true);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }
}
