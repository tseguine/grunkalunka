//! Positive, finite source-flow facts for ordinary synchronous helper signatures.
//! This is representation planning, never admission or annotation-based typing.
use super::*;
use oxc_ast::{AstKind, ast::Function};
use std::collections::{BTreeMap, BTreeSet};

// Number inputs retain their fast path. Non-Number primitive arguments need the
// managed carrier when a helper also transports managed values: ordinary source
// parameter signatures do not admit the General pair. Other profiles retain the
// existing primitive-result planner. Function values remain subject to rejection.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Primitive,
    GeneralPrimitive,
    Object,
    Array,
    String,
}
type Kinds = BTreeSet<Kind>;

fn managed_type(kinds: &Kinds) -> Option<Type> {
    (kinds.contains(&Kind::Object) || kinds.contains(&Kind::Array) || kinds.contains(&Kind::String))
        .then_some(if kinds.len() == 1 && kinds.contains(&Kind::Object) {
            Type::ManagedObject
        } else if kinds.len() == 1 && kinds.contains(&Kind::Array) {
            Type::ManagedArray
        } else {
            Type::ManagedValue
        })
}

fn symbol(identifier: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> Option<SymbolId> {
    identifier
        .reference_id
        .get()
        .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
}

pub(super) fn function<'s>(
    call: &CallExpression<'_>,
    semantic: &Semantic<'_>,
    functions: &'s [NumericFunction],
) -> Option<&'s NumericFunction> {
    let Expression::Identifier(identifier) = invocation::callee(call) else {
        return None;
    };
    let symbol = symbol(identifier, semantic)?;
    functions.iter().find(|f| f.symbol == symbol)
}

// Union over finite source alternatives. Bottom denotes unresolved facts, not
// proof of a runtime kind. Actual lowering still checks every typed position.
fn kinds(
    node: &Expression<'_>,
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    bindings: &BTreeMap<SymbolId, Kinds>,
    returns: &[Kinds],
) -> Kinds {
    enum Task<'e, 'a> {
        Expr(&'e Expression<'a>, usize),
        Join,
        Member(bool),
    }
    let mut work = vec![Task::Expr(node, 0)];
    let mut values: Vec<Kinds> = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Task::Join => {
                let right = values.pop().expect("right source kinds");
                values.last_mut().expect("left source kinds").extend(right);
            }
            Task::Member(length) => {
                let receiver = values.pop().expect("receiver source kinds");
                values.push(
                    if receiver.contains(&Kind::Object)
                        || (receiver.contains(&Kind::Array) && !length)
                    {
                        [Kind::Primitive, Kind::Object, Kind::Array, Kind::String].into()
                    } else {
                        [Kind::Primitive].into()
                    },
                );
            }
            Task::Expr(node, depth) => {
                if depth > 128 {
                    values.push(Kinds::new());
                    continue;
                }
                if let Some(inner) = erased_inner(node) {
                    work.push(Task::Expr(inner, depth + 1));
                    continue;
                }
                match node {
                    Expression::ParenthesizedExpression(w) => {
                        work.push(Task::Expr(&w.expression, depth + 1));
                    }
                    Expression::ObjectExpression(_) => values.push([Kind::Object].into()),
                    Expression::ArrayExpression(_) => values.push([Kind::Array].into()),
                    Expression::StringLiteral(_) => values.push([Kind::String].into()),
                    Expression::UnaryExpression(u) if u.operator == UnaryOperator::Typeof => {
                        values.push([Kind::String].into());
                    }
                    Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => {
                        values.push([Kind::GeneralPrimitive].into());
                    }
                    Expression::UnaryExpression(u)
                        if matches!(
                            u.operator,
                            UnaryOperator::Void | UnaryOperator::LogicalNot
                        ) =>
                    {
                        values.push([Kind::GeneralPrimitive].into());
                    }
                    Expression::BinaryExpression(b)
                        if matches!(
                            b.operator,
                            BinaryOperator::Equality
                                | BinaryOperator::Inequality
                                | BinaryOperator::StrictEquality
                                | BinaryOperator::StrictInequality
                                | BinaryOperator::LessThan
                                | BinaryOperator::LessEqualThan
                                | BinaryOperator::GreaterThan
                                | BinaryOperator::GreaterEqualThan
                        ) =>
                    {
                        values.push([Kind::GeneralPrimitive].into());
                    }
                    Expression::Identifier(i)
                        if i.name == "undefined" && symbol(i, semantic).is_none() =>
                    {
                        values.push([Kind::GeneralPrimitive].into());
                    }
                    Expression::Identifier(identifier) => values.push(
                        symbol(identifier, semantic)
                            .and_then(|symbol| bindings.get(&symbol).cloned())
                            .unwrap_or_else(|| [Kind::Primitive].into()),
                    ),
                    Expression::CallExpression(call) => values.push(
                        function(call, semantic, functions)
                            .map(|f| returns[f.id.0].clone())
                            .unwrap_or_else(|| [Kind::Primitive].into()),
                    ),
                    Expression::ConditionalExpression(c) => {
                        work.push(Task::Join);
                        work.push(Task::Expr(&c.alternate, depth + 1));
                        work.push(Task::Expr(&c.consequent, depth + 1));
                    }
                    Expression::LogicalExpression(l) => {
                        work.push(Task::Join);
                        work.push(Task::Expr(&l.right, depth + 1));
                        work.push(Task::Expr(&l.left, depth + 1));
                    }
                    Expression::SequenceExpression(s) => {
                        if let Some(last) = s.expressions.last() {
                            work.push(Task::Expr(last, depth + 1));
                        } else {
                            values.push(Kinds::new());
                        }
                    }
                    Expression::StaticMemberExpression(m) => {
                        work.push(Task::Member(m.property.name == "length"));
                        work.push(Task::Expr(&m.object, depth + 1));
                    }
                    Expression::ComputedMemberExpression(m) => {
                        work.push(Task::Member(closures::unwrapped_at(&m.expression,depth+1).is_some_and(|(key,_)|matches!(key,Expression::StringLiteral(s) if s.value=="length"))));
                        work.push(Task::Expr(&m.object, depth + 1));
                    }
                    Expression::AssignmentExpression(a) => {
                        // Property assignment expressions use managed snapshots,
                        // even when their current RHS is a known primitive.
                        match a.left.as_simple_assignment_target() {
                            Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => {
                                work.push(Task::Member(m.property.name == "length"));
                                work.push(Task::Expr(&m.object, depth + 1));
                            }
                            Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => {
                                work.push(Task::Member(closures::unwrapped_at(&m.expression,depth+1).is_some_and(|(key,_)|matches!(key,Expression::StringLiteral(s) if s.value=="length"))));
                                work.push(Task::Expr(&m.object, depth + 1));
                            }
                            _ => work.push(Task::Expr(&a.right, depth + 1)),
                        }
                    }
                    _ => values.push([Kind::Primitive].into()),
                }
            }
        }
    }
    values.pop().expect("one source kind query")
}

fn extend(bindings: &mut BTreeMap<SymbolId, Kinds>, symbol: SymbolId, kinds: Kinds) -> bool {
    let entry = bindings.entry(symbol).or_default();
    let before = entry.len();
    entry.extend(kinds);
    entry.len() != before
}

pub(crate) fn infer(
    semantic: &Semantic<'_>,
    declarations: &[&Function<'_>],
    functions: &mut [NumericFunction],
) {
    // The only public inputs are Numbers. Without a construction there is no
    // managed reference origin, so ordinary modules keep the old planning path.
    if !semantic.nodes().iter().any(|n| {
        matches!(
            n.kind(),
            AstKind::ObjectExpression(_) | AstKind::ArrayExpression(_) | AstKind::StringLiteral(_)
        ) || matches!(n.kind(), AstKind::UnaryExpression(u) if u.operator == UnaryOperator::Typeof)
    }) {
        return;
    }
    let parameters: Vec<Vec<_>> = declarations
        .iter()
        .map(|f| {
            f.params
                .items
                .iter()
                .map(|p| {
                    if let BindingPattern::BindingIdentifier(i) = &p.pattern {
                        i.symbol_id.get()
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    let mut bindings = BTreeMap::new();
    for (index, parameters) in parameters.iter().enumerate() {
        for symbol in parameters.iter().flatten() {
            bindings.insert(
                *symbol,
                if index == 0 {
                    [Kind::Primitive].into()
                } else {
                    Kinds::new()
                },
            );
        }
    }
    let mut definitions = Vec::new();
    let mut calls = Vec::new();
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::VariableDeclarator(v) => {
                if let BindingPattern::BindingIdentifier(i) = &v.id
                    && let Some(symbol) = i.symbol_id.get()
                {
                    bindings.entry(symbol).or_default();
                    definitions.push((symbol, v.init.as_ref()));
                }
            }
            AstKind::AssignmentExpression(a) => {
                if let Some(identifier) = a
                    .left
                    .as_simple_assignment_target()
                    .and_then(|target| target_identifier(target, 0).ok().flatten())
                    && let Some(symbol) = symbol(identifier, semantic)
                {
                    definitions.push((symbol, Some(&a.right)));
                }
            }
            AstKind::CallExpression(call) => {
                if let Some(f) = function(call, semantic, functions)
                    && f.id.0 != 0
                    && f.async_signature.is_none()
                {
                    calls.push((f.id.0, call));
                }
            }
            _ => (),
        }
    }
    let results: Vec<_> = declarations
        .iter()
        .map(|f| block_results::return_statements(semantic, f.node_id.get()))
        .collect();
    let mut returns = vec![Kinds::new(); functions.len()];
    loop {
        let mut changed = false;
        for (symbol, value) in &definitions {
            let next = value
                .map(|v| kinds(v, semantic, functions, &bindings, &returns))
                .unwrap_or_else(|| [Kind::Primitive].into());
            changed |= extend(&mut bindings, *symbol, next);
        }
        for (index, call) in &calls {
            for (symbol, actual) in parameters[*index].iter().zip(invocation::arguments(call)) {
                if let (Some(symbol), Some(actual)) = (symbol, actual.as_expression()) {
                    let next = kinds(actual, semantic, functions, &bindings, &returns);
                    changed |= extend(&mut bindings, *symbol, next);
                }
            }
        }
        for (index, statements) in results.iter().enumerate().skip(1) {
            if functions[index].async_signature.is_some() {
                continue;
            }
            for statement in statements {
                let next = statement
                    .argument
                    .as_ref()
                    .map(|v| kinds(v, semantic, functions, &bindings, &returns))
                    .unwrap_or_else(|| [Kind::Primitive].into());
                let before = returns[index].len();
                returns[index].extend(next);
                changed |= before != returns[index].len();
            }
        }
        if !changed {
            break;
        }
    }
    let numeric_entry = functions[0].return_type == Type::Number;
    for (index, f) in functions.iter_mut().enumerate().skip(1) {
        if f.async_signature.is_some() {
            continue;
        }
        // Numeric-entry profiles forbid General helper results. The managed
        // result lane retains Boolean/nullish kinds there; other profiles keep
        // their original primitive-result planner.
        let managed_return = managed_type(&returns[index]).or_else(|| {
            (numeric_entry && returns[index].contains(&Kind::GeneralPrimitive))
                .then_some(Type::ManagedValue)
        });
        let managed_signature = managed_return.is_some()
            || parameters[index]
                .iter()
                .flatten()
                .any(|s| bindings.get(s).and_then(managed_type).is_some());
        let types: Vec<_> = parameters[index]
            .iter()
            .map(|symbol| {
                symbol
                    .and_then(|s| bindings.get(&s))
                    .and_then(|kinds| {
                        managed_type(kinds).or_else(|| {
                            (managed_signature && kinds.contains(&Kind::GeneralPrimitive))
                                .then_some(Type::ManagedValue)
                        })
                    })
                    .unwrap_or(Type::Number)
            })
            .collect();
        if managed_signature {
            f.parameter_types = types;
        }
        if let Some(ty) = managed_return {
            f.return_type = ty;
        }
    }
}

// Reuse the finite value-kind lattice for arrow formal cells. Call targets and
// actual-to-formal writes are supplied by storage's existing constructor graph.
pub(super) fn closure_inputs<'a>(
    semantic: &Semantic<'a>,
    functions: &[NumericFunction],
    definitions: &BTreeMap<SymbolId, Option<&Expression<'a>>>,
    writes: &BTreeMap<SymbolId, Vec<&Expression<'a>>>,
    parameters: &BTreeSet<SymbolId>,
    groups: &BTreeSet<BTreeSet<oxc_syntax::node::NodeId>>,
    arrows: &BTreeMap<oxc_syntax::node::NodeId, &oxc_ast::ast::ArrowFunctionExpression<'a>>,
) -> BTreeMap<SymbolId, Type> {
    if !semantic.nodes().iter().any(|n| {
        matches!(
            n.kind(),
            AstKind::ObjectExpression(_) | AstKind::ArrayExpression(_) | AstKind::StringLiteral(_)
        ) || matches!(n.kind(),AstKind::UnaryExpression(u) if u.operator==UnaryOperator::Typeof)
    }) {
        return BTreeMap::new();
    }
    let from_type = |ty: Type| -> Kinds {
        match ty {
            Type::ManagedObject => [Kind::Object].into(),
            Type::ManagedArray => [Kind::Array].into(),
            Type::ManagedValue => [
                Kind::Primitive,
                Kind::GeneralPrimitive,
                Kind::Object,
                Kind::Array,
                Kind::String,
            ]
            .into(),
            Type::General => [Kind::Primitive, Kind::GeneralPrimitive].into(),
            _ => [Kind::Primitive].into(),
        }
    };
    // Unresolved formal facts start at bottom, so forwarding does not invent an
    // initial Number call before the actual object/array origin has propagated.
    let mut bindings: BTreeMap<_, Kinds> = parameters.iter().map(|s| (*s, Kinds::new())).collect();
    for symbol in definitions.keys().chain(writes.keys()) {
        bindings.entry(*symbol).or_default();
    }
    for node in semantic.nodes().iter() {
        let AstKind::Function(f) = node.kind() else {
            continue;
        };
        let Some(function) =
            f.id.as_ref()
                .and_then(|i| i.symbol_id.get())
                .and_then(|s| functions.iter().find(|f| f.symbol == s))
        else {
            continue;
        };
        for (index, p) in f.params.items.iter().enumerate() {
            if let BindingPattern::BindingIdentifier(i) = &p.pattern
                && let Some(s) = i.symbol_id.get()
            {
                bindings.insert(s, from_type(function.parameter_type(index)));
            }
        }
    }
    let returns = functions
        .iter()
        .map(|f| from_type(f.return_type))
        .collect::<Vec<_>>();
    loop {
        let mut changed = false;
        for (symbol, expression) in definitions {
            let next = expression
                .map(|e| kinds(e, semantic, functions, &bindings, &returns))
                .unwrap_or_else(|| [Kind::GeneralPrimitive].into());
            changed |= extend(&mut bindings, *symbol, next);
        }
        for (symbol, expressions) in writes {
            for expression in expressions {
                let next = kinds(expression, semantic, functions, &bindings, &returns);
                changed |= extend(&mut bindings, *symbol, next);
            }
        }
        for group in groups {
            let count = group
                .iter()
                .map(|body| arrows[body].params.items.len())
                .max()
                .unwrap_or(0);
            for index in 0..count {
                let symbols = group
                    .iter()
                    .filter_map(|body| arrows[body].params.items.get(index))
                    .filter_map(|p| match &p.pattern {
                        BindingPattern::BindingIdentifier(i) => i.symbol_id.get(),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let joined = symbols
                    .iter()
                    .filter_map(|s| bindings.get(s))
                    .flatten()
                    .copied()
                    .collect::<Kinds>();
                for symbol in symbols {
                    changed |= extend(&mut bindings, symbol, joined.clone());
                }
            }
        }
        if !changed {
            break;
        }
    }
    parameters
        .iter()
        .filter_map(|s| bindings.get(s).and_then(managed_type).map(|ty| (*s, ty)))
        .collect()
}
