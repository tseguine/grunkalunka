//! Nullish exclusions established by checks on unwritten lexical bindings.
use super::*;
use oxc_span::Span;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Test {
    Truthy,
    Nullish,
    Null,
    Undefined,
    Array,
    Typeof(&'static str),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Guard {
    test: Test,
    positive: bool,
}

impl Guard {
    fn retains(self, origin: &Origin) -> bool {
        if *origin == Origin::Other {
            return true;
        }
        let matches = match self.test {
            Test::Truthy => match origin {
                Origin::Null | Origin::Undefined => false,
                Origin::String | Origin::Number | Origin::Boolean => return true,
                _ => true,
            },
            Test::Nullish => matches!(origin, Origin::Null | Origin::Undefined),
            Test::Null => *origin == Origin::Null,
            Test::Undefined => *origin == Origin::Undefined,
            Test::Array => matches!(origin, Origin::Array(_)),
            Test::Typeof(name) => {
                name == match origin {
                    Origin::Arrow(_) | Origin::Named(_) => "function",
                    Origin::Object(_) | Origin::NullObject(_) | Origin::Array(_) | Origin::Null => {
                        "object"
                    }
                    Origin::Undefined => "undefined",
                    Origin::String => "string",
                    Origin::Number => "number",
                    Origin::Boolean => "boolean",
                    Origin::Other => unreachable!(),
                }
            }
        };
        matches == self.positive
    }
}

fn comparison(
    subject: &Expression<'_>,
    value: &Expression<'_>,
    strict: bool,
    depth: usize,
    semantic: &Semantic<'_>,
) -> Option<(SymbolId, Test)> {
    if let Expression::UnaryExpression(u) = subject
        && u.operator == UnaryOperator::Typeof
    {
        let Expression::Identifier(id) = closures::unwrapped(&u.argument, depth + 1)? else {
            return None;
        };
        let Expression::StringLiteral(s) = value else {
            return None;
        };
        let name = [
            "object",
            "function",
            "undefined",
            "number",
            "boolean",
            "string",
        ]
        .into_iter()
        .find(|name| s.value == *name)?;
        return Some((symbol(id, semantic)?, Test::Typeof(name)));
    }
    let Expression::Identifier(id) = subject else {
        return None;
    };
    let test = match value {
        Expression::NullLiteral(_) => {
            if strict {
                Test::Null
            } else {
                Test::Nullish
            }
        }
        Expression::Identifier(id) if id.name == "undefined" && symbol(id, semantic).is_none() => {
            if strict {
                Test::Undefined
            } else {
                Test::Nullish
            }
        }
        _ => return None,
    };
    Some((symbol(id, semantic)?, test))
}

fn condition(expr: &Expression<'_>, semantic: &Semantic<'_>) -> Option<(SymbolId, Guard)> {
    let (mut expr, mut depth) = closures::unwrapped_at(expr, 0)?;
    let mut positive = true;
    while let Expression::UnaryExpression(unary) = expr {
        if unary.operator != UnaryOperator::LogicalNot {
            break;
        }
        positive = !positive;
        (expr, depth) = closures::unwrapped_at(&unary.argument, depth + 1)?;
    }
    let (binding, test) = match expr {
        Expression::Identifier(id) => (symbol(id, semantic)?, Test::Truthy),
        Expression::CallExpression(call)
            if object_intrinsics::is_array_test(call, semantic) && call.arguments.len() == 1 =>
        {
            let Expression::Identifier(id) =
                closures::unwrapped(call.arguments[0].as_expression()?, depth + 1)?
            else {
                return None;
            };
            (symbol(id, semantic)?, Test::Array)
        }
        Expression::BinaryExpression(b) => {
            let (equal, strict) = match b.operator {
                BinaryOperator::Equality => (true, false),
                BinaryOperator::StrictEquality => (true, true),
                BinaryOperator::Inequality => (false, false),
                BinaryOperator::StrictInequality => (false, true),
                _ => return None,
            };
            let left = closures::unwrapped(&b.left, depth + 1)?;
            let right = closures::unwrapped(&b.right, depth + 1)?;
            positive = positive == equal;
            comparison(left, right, strict, depth + 1, semantic)
                .or_else(|| comparison(right, left, strict, depth + 1, semantic))?
        }
        _ => return None,
    };
    Some((binding, Guard { test, positive }))
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

pub(super) struct Reads(BTreeMap<NodeId, Vec<Guard>>);

impl Reads {
    pub(super) fn narrow(&self, id: NodeId, alternatives: &mut Alternatives) {
        if let Some(guards) = self.0.get(&id) {
            alternatives.retain(|origin| guards.iter().all(|guard| guard.retains(origin)));
        }
    }
}

pub(super) fn collect(
    semantic: &Semantic<'_>,
    var_symbols: &BTreeSet<SymbolId>,
    var_aliases: &BTreeMap<SymbolId, SymbolId>,
) -> Reads {
    let nodes = semantic.nodes();
    let scope = semantic.scoping();
    let mut guarded = BTreeMap::<SymbolId, BTreeMap<NodeId, BTreeMap<Guard, Vec<Span>>>>::new();
    for node in nodes.iter() {
        let AstKind::IfStatement(choice) = node.kind() else {
            continue;
        };
        let Some((binding, guard)) = condition(&choice.test, semantic) else {
            continue;
        };
        let Some(owner) = var_bindings::owner(node.id(), semantic) else {
            continue;
        };
        let guards = guarded
            .entry(binding)
            .or_default()
            .entry(owner)
            .or_default();
        for (positive, arm, opposite) in [
            (
                guard.positive,
                Some(&choice.consequent),
                choice.alternate.as_ref(),
            ),
            (
                !guard.positive,
                choice.alternate.as_ref(),
                Some(&choice.consequent),
            ),
        ] {
            let ranges = guards.entry(Guard { positive, ..guard }).or_default();
            if let Some(arm) = arm {
                ranges.push(arm.span());
            }
            if opposite.is_some_and(exits) {
                let end = match nodes.parent_kind(node.id()) {
                    AstKind::BlockStatement(b) => b.span.end,
                    AstKind::FunctionBody(b) => b.span.end,
                    _ => continue,
                };
                ranges.push(Span::new(choice.span.end, end));
            }
        }
    }
    let mut reads = BTreeMap::<NodeId, Vec<Guard>>::new();
    // A var initializer or iteration head can write a parameter cell without
    // being an assignment reference to the parameter's own symbol.
    let aliased_parameters: BTreeSet<_> = var_aliases.values().copied().collect();
    for (binding, mut owners) in guarded {
        let declaration = semantic.symbol_declaration(binding);
        let eligible = match declaration.kind() {
            AstKind::VariableDeclarator(d) => {
                matches!(d.id, BindingPattern::BindingIdentifier(_))
                    && matches!(nodes.parent_kind(declaration.id()), AstKind::VariableDeclaration(parent)
                    if matches!(parent.kind,VariableDeclarationKind::Let | VariableDeclarationKind::Const))
            }
            AstKind::FormalParameter(p) => {
                matches!(p.pattern, BindingPattern::BindingIdentifier(_))
                    && !var_symbols.contains(&binding)
                    && !aliased_parameters.contains(&binding)
                    && scope.symbol_redeclarations(binding).is_empty()
            }
            _ => false,
        };
        if !eligible || scope.get_resolved_references(binding).any(|r| r.is_write()) {
            continue;
        }
        for guards in owners.values_mut() {
            for spans in guards.values_mut() {
                spans.sort_unstable_by_key(|s| s.start);
                // Prefix maxima also cover nested and overlapping guard ranges.
                let mut end = 0;
                for span in spans {
                    end = end.max(span.end);
                    span.end = end;
                }
            }
        }
        for reference in scope
            .get_resolved_references(binding)
            .filter(|r| r.is_read())
        {
            let id = reference.node_id();
            let Some(guards) =
                var_bindings::owner(id, semantic).and_then(|owner| owners.get(&owner))
            else {
                continue;
            };
            let read = nodes.kind(id).span();
            for (guard, spans) in guards {
                let before = spans.partition_point(|s| s.start <= read.start);
                if spans[..before].last().is_some_and(|s| s.end >= read.end) {
                    reads.entry(id).or_default().push(*guard);
                }
            }
        }
    }
    Reads(reads)
}
