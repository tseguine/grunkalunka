//! Nullish exclusions established by checks on unwritten lexical bindings.
use super::*;
use oxc_span::Span;

// Positive means the true arm excludes both Null and Undefined. Truthiness
// can exclude additional values, but is not a proof of any other kind.
fn condition(expr: &Expression<'_>, semantic: &Semantic<'_>) -> Option<(SymbolId, bool)> {
    let (mut expr, mut depth) = closures::unwrapped_at(expr, 0)?;
    let mut positive = true;
    while let Expression::UnaryExpression(unary) = expr {
        if unary.operator != UnaryOperator::LogicalNot {
            return None;
        }
        positive = !positive;
        (expr, depth) = closures::unwrapped_at(&unary.argument, depth + 1)?;
    }
    let identifier = match expr {
        Expression::Identifier(id) => id,
        Expression::BinaryExpression(b)
            if matches!(
                b.operator,
                BinaryOperator::Equality | BinaryOperator::Inequality
            ) =>
        {
            let left = closures::unwrapped(&b.left, depth + 1)?;
            let right = closures::unwrapped(&b.right, depth + 1)?;
            let nullish = |v: &Expression<'_>| {
                matches!(v, Expression::NullLiteral(_))
                    || matches!(v, Expression::Identifier(id) if id.name == "undefined" && symbol(id,semantic).is_none())
            };
            positive = positive == (b.operator == BinaryOperator::Inequality);
            match (left, right) {
                (Expression::Identifier(id), value) if nullish(value) => id,
                (value, Expression::Identifier(id)) if nullish(value) => id,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((symbol(identifier, semantic)?, positive))
}

fn exits(mut statement: &Statement<'_>) -> bool {
    while let Statement::BlockStatement(block) = statement {
        let Some(last) = block.body.last() else {
            return false;
        };
        statement = last;
    }
    matches!(
        statement,
        Statement::ReturnStatement(_) | Statement::ThrowStatement(_)
    )
}

pub(super) fn non_nullish(semantic: &Semantic<'_>) -> BTreeSet<NodeId> {
    let nodes = semantic.nodes();
    let scope = semantic.scoping();
    let mut guarded = BTreeMap::<SymbolId, BTreeMap<NodeId, Vec<Span>>>::new();
    for node in nodes.iter() {
        let AstKind::IfStatement(choice) = node.kind() else {
            continue;
        };
        let Some((binding, positive)) = condition(&choice.test, semantic) else {
            continue;
        };
        let Some(owner) = var_bindings::owner(node.id(), semantic) else {
            continue;
        };
        let (live, rejected) = if positive {
            (Some(&choice.consequent), choice.alternate.as_ref())
        } else {
            (choice.alternate.as_ref(), Some(&choice.consequent))
        };
        let ranges = guarded
            .entry(binding)
            .or_default()
            .entry(owner)
            .or_default();
        if let Some(live) = live {
            ranges.push(live.span());
        }
        if rejected.is_some_and(exits) {
            let end = match nodes.parent_kind(node.id()) {
                AstKind::BlockStatement(b) => b.span.end,
                AstKind::FunctionBody(b) => b.span.end,
                _ => continue,
            };
            ranges.push(Span::new(choice.span.end, end));
        }
    }
    let mut result = BTreeSet::new();
    for (binding, mut owners) in guarded {
        let declaration = semantic.symbol_declaration(binding);
        if !matches!(declaration.kind(), AstKind::VariableDeclarator(d) if matches!(d.id,BindingPattern::BindingIdentifier(_)))
            || !matches!(nodes.parent_kind(declaration.id()),AstKind::VariableDeclaration(d) if matches!(d.kind,VariableDeclarationKind::Let | VariableDeclarationKind::Const))
            || scope.get_resolved_references(binding).any(|r| r.is_write())
        {
            continue;
        }
        for spans in owners.values_mut() {
            spans.sort_unstable_by_key(|s| s.start);
            // Prefix maxima let each read test containment with a binary search,
            // even when an earlier outer guard contains several inner guards.
            let mut end = 0;
            for span in spans {
                end = end.max(span.end);
                span.end = end;
            }
        }
        for reference in scope
            .get_resolved_references(binding)
            .filter(|r| r.is_read())
        {
            let id = reference.node_id();
            let Some(spans) =
                var_bindings::owner(id, semantic).and_then(|owner| owners.get(&owner))
            else {
                continue;
            };
            let read = nodes.kind(id).span();
            let before = spans.partition_point(|s| s.start <= read.start);
            if spans[..before].last().is_some_and(|s| s.end >= read.end) {
                result.insert(id);
            }
        }
    }
    result
}
