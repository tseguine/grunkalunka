//! Runtime kind exclusions established by guards on unchanged bindings.
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
    matched: bool,
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
        // Matching one literal proves the kind. Missing it leaves other
        // values of the same kind possible.
        Expression::StringLiteral(_) if strict && matched => Test::Typeof("string"),
        Expression::BooleanLiteral(_) if strict && matched => Test::Typeof("boolean"),
        _ if strict && matched && static_keys::number(value).is_some() => Test::Typeof("number"),
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

fn atomic_guard(
    expr: &Expression<'_>,
    depth: usize,
    mut positive: bool,
    semantic: &Semantic<'_>,
) -> Option<(SymbolId, Guard)> {
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
            comparison(left, right, strict, positive, depth + 1, semantic)
                .or_else(|| comparison(right, left, strict, positive, depth + 1, semantic))?
        }
        _ => return None,
    };
    Some((binding, Guard { test, positive }))
}

fn const_initializer<'a>(
    binding: SymbolId,
    owner: NodeId,
    semantic: &Semantic<'a>,
) -> Option<&'a Expression<'a>> {
    let declaration = semantic.symbol_declaration(binding);
    let AstKind::VariableDeclarator(local) = declaration.kind() else {
        return None;
    };
    let AstKind::VariableDeclaration(parent) = semantic.nodes().parent_kind(declaration.id())
    else {
        return None;
    };
    if parent.kind != VariableDeclarationKind::Const
        || !matches!(local.id, BindingPattern::BindingIdentifier(_))
        || var_bindings::owner(declaration.id(), semantic) != Some(owner)
    {
        return None;
    }
    local.init.as_ref()
}

fn visit_guards<'a>(
    expr: &Expression<'a>,
    outcome: bool,
    owner: NodeId,
    semantic: &Semantic<'a>,
    mut emit: impl FnMut(SymbolId, Guard),
) {
    let mut pending = Vec::new();
    let mut expanded = BTreeSet::new();
    let mut next = Some((expr, outcome, 0));
    while let Some((expr, outcome, depth)) = next.take().or_else(|| pending.pop()) {
        let Some((expr, depth)) = closures::unwrapped_at(expr, depth) else {
            continue;
        };
        match expr {
            Expression::UnaryExpression(u) if u.operator == UnaryOperator::LogicalNot => {
                next = Some((&u.argument, !outcome, depth + 1));
            }
            // A truthy conjunction or a falsy disjunction evaluated both operands.
            Expression::LogicalExpression(l)
                if (outcome && l.operator == LogicalOperator::And)
                    || (!outcome && l.operator == LogicalOperator::Or) =>
            {
                pending.push((&l.right, outcome, depth + 1));
                next = Some((&l.left, outcome, depth + 1));
            }
            _ => {
                let Some((binding, guard)) = atomic_guard(expr, depth, outcome, semantic) else {
                    continue;
                };
                emit(binding, guard);
                // A const stores its initializer's truthiness. Follow each
                // alias once per polarity, even when compound guards share it.
                if guard.test == Test::Truthy
                    && expanded.insert((binding, outcome))
                    && let Some(init) = const_initializer(binding, owner, semantic)
                {
                    next = Some((init, outcome, depth + 1));
                }
            }
        }
    }
}

fn skips_following_statements(mut statement: &Statement<'_>) -> bool {
    while let Statement::BlockStatement(block) = statement {
        let Some(last) = block.body.last() else {
            return false;
        };
        statement = last;
    }
    matches!(
        statement,
        Statement::ReturnStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::BreakStatement(_)
            | Statement::ContinueStatement(_)
    )
}

// Case tests use strict equality and run before the selected body. A default
// selected directly has missed every label, including labels after default.
fn visit_switch_guards(
    switch: &oxc_ast::ast::SwitchStatement<'_>,
    semantic: &Semantic<'_>,
    mut emit: impl FnMut(SymbolId, Guard, Span),
) {
    let Some(subject) = closures::unwrapped(&switch.discriminant, 0) else {
        return;
    };
    let mut fallthrough = false;
    for case in &switch.cases {
        if !fallthrough && let Some(first) = case.consequent.first() {
            let body = Span::new(first.span().start, case.span.end);
            let positive = case.test.is_some();
            let mut check_label = |label: &Expression<'_>| {
                if let Some(value) = closures::unwrapped(label, 0)
                    && let Some((binding, test)) =
                        comparison(subject, value, true, positive, 0, semantic)
                {
                    emit(binding, Guard { test, positive }, body);
                }
            };
            if let Some(label) = &case.test {
                check_label(label);
            } else {
                for label in switch.cases.iter().filter_map(|c| c.test.as_ref()) {
                    check_label(label);
                }
            }
        }
        fallthrough = !case
            .consequent
            .last()
            .is_some_and(skips_following_statements);
    }
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
    let mut record = |owner, binding, guard, span| {
        guarded
            .entry(binding)
            .or_default()
            .entry(owner)
            .or_default()
            .entry(guard)
            .or_default()
            .push(span);
    };
    for node in nodes.iter() {
        let (test, truthy_arm, falsy_arm) = match node.kind() {
            AstKind::SwitchStatement(switch) => {
                if let Some(owner) = var_bindings::owner(node.id(), semantic) {
                    visit_switch_guards(switch, semantic, |binding, guard, span| {
                        record(owner, binding, guard, span);
                    });
                }
                continue;
            }
            AstKind::IfStatement(c) => (
                &c.test,
                Some(c.consequent.span()),
                c.alternate.as_ref().map(GetSpan::span),
            ),
            AstKind::ConditionalExpression(c) => {
                (&c.test, Some(c.consequent.span()), Some(c.alternate.span()))
            }
            AstKind::WhileStatement(w) => (&w.test, Some(w.body.span()), None),
            AstKind::ForStatement(f) => {
                let Some(test) = &f.test else { continue };
                // The update and body are adjacent in source and both require a
                // truthy test. The initializer and the test precede this range.
                let mut span = f.body.span();
                if let Some(update) = &f.update {
                    span.start = update.span().start;
                }
                (test, Some(span), None)
            }
            AstKind::LogicalExpression(l) if l.operator == LogicalOperator::And => {
                (&l.left, Some(l.right.span()), None)
            }
            AstKind::LogicalExpression(l) if l.operator == LogicalOperator::Or => {
                (&l.left, None, Some(l.right.span()))
            }
            _ => continue,
        };
        let Some(owner) = var_bindings::owner(node.id(), semantic) else {
            continue;
        };
        let mut truthy_suffix = None;
        let mut falsy_suffix = None;
        if let AstKind::IfStatement(choice) = node.kind() {
            let end = match nodes.parent_kind(node.id()) {
                AstKind::BlockStatement(b) => Some(b.span.end),
                AstKind::FunctionBody(b) => Some(b.span.end),
                // A later case can be entered directly, bypassing this test.
                AstKind::SwitchCase(c) => Some(c.span.end),
                _ => None,
            };
            if let Some(end) = end {
                let suffix = Span::new(choice.span.end, end);
                truthy_suffix = choice
                    .alternate
                    .as_ref()
                    .is_some_and(skips_following_statements)
                    .then_some(suffix);
                falsy_suffix = skips_following_statements(&choice.consequent).then_some(suffix);
            }
        }
        for (outcome, arm, suffix) in [
            (true, truthy_arm, truthy_suffix),
            (false, falsy_arm, falsy_suffix),
        ] {
            if arm.is_some() || suffix.is_some() {
                visit_guards(test, outcome, owner, semantic, |binding, guard| {
                    for span in arm.into_iter().chain(suffix) {
                        record(owner, binding, guard, span);
                    }
                });
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
