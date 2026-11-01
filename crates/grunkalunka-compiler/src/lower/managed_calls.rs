//! Positive, finite source-flow facts for ordinary synchronous helper signatures.
//! This is representation planning, never admission or annotation-based typing.
use super::*;
use oxc_ast::{AstKind, ast::Function};
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};
mod input_flow;

// Number inputs retain their fast path. Non-Number primitive arguments need the
// managed carrier when a helper also transports managed values: ordinary source
// parameter signatures do not admit the General pair. Other profiles retain the
// existing primitive-result planner. Functions use the checked constructor path.
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
    closure_calls: Option<&BTreeMap<NodeId, Kinds>>,
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
                    Expression::CallExpression(call) if object_keys::is_call(call, semantic) => {
                        values.push([Kind::Array].into())
                    }
                    Expression::CallExpression(call) => values.push(
                        function(call, semantic, functions)
                            .map(|f| returns[f.id.0].clone())
                            .or_else(|| {
                                closure_calls
                                    .and_then(|calls| calls.get(&call.node_id.get()).cloned())
                            })
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

fn extend<K: Ord>(bindings: &mut BTreeMap<K, Kinds>, symbol: K, kinds: Kinds) -> bool {
    let entry = bindings.entry(symbol).or_default();
    let before = entry.len();
    entry.extend(kinds);
    entry.len() != before
}

fn has_managed_origin(semantic: &Semantic<'_>) -> bool {
    semantic.nodes().iter().any(|node| {
        let origin = match node.kind() {
            AstKind::CallExpression(call) if object_keys::is_call(call, semantic) => true,
            AstKind::ObjectExpression(_)
            | AstKind::ArrayExpression(_)
            | AstKind::StringLiteral(_)
            | AstKind::BooleanLiteral(_)
            | AstKind::NullLiteral(_) => true,
            AstKind::UnaryExpression(u) => matches!(
                u.operator,
                UnaryOperator::Typeof | UnaryOperator::Void | UnaryOperator::LogicalNot
            ),
            AstKind::BinaryExpression(b) => matches!(
                b.operator,
                BinaryOperator::Equality
                    | BinaryOperator::Inequality
                    | BinaryOperator::StrictEquality
                    | BinaryOperator::StrictInequality
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessEqualThan
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterEqualThan
            ),
            AstKind::IdentifierReference(id) => {
                id.name == "undefined" && symbol(id, semantic).is_none()
            }
            AstKind::VariableDeclarator(v) => v.init.is_none(),
            _ => false,
        };
        origin
            && !semantic.nodes().ancestor_kinds(node.id()).any(|parent| {
                parent.is_type()
                    || matches!(
                        parent,
                        AstKind::TSTypeAliasDeclaration(_)
                            | AstKind::TSInterfaceDeclaration(_)
                            | AstKind::TSTypeAnnotation(_)
                            | AstKind::TSTypeParameterDeclaration(_)
                            | AstKind::TSTypeParameterInstantiation(_)
                            | AstKind::TSImportType(_)
                            | AstKind::TSTypeQuery(_)
                            | AstKind::ImportDeclaration(_)
                            | AstKind::ExportNamedDeclaration(_)
                            | AstKind::ExportAllDeclaration(_)
                    )
            })
    })
}

pub(crate) fn infer(
    semantic: &Semantic<'_>,
    declarations: &[&Function<'_>],
    functions: &mut [NumericFunction],
    stored: &stored_functions::Fields,
) {
    // The only public inputs are Numbers. Without a construction there is no
    // managed reference origin, so ordinary modules keep the old planning path.
    if !functions
        .iter()
        .skip(1)
        .any(|f| f.async_signature.is_none())
        || !has_managed_origin(semantic)
    {
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
    let mut all_calls = Vec::new();
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
                all_calls.push(call);
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
    let forwarded = input_flow::collect(semantic, &definitions, &all_calls, stored);
    for arrow in forwarded.arrows.values() {
        for parameter in &arrow.params.items {
            if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                && let Some(symbol) = identifier.symbol_id.get()
            {
                bindings.entry(symbol).or_default();
            }
        }
    }
    for (symbol, actual) in &forwarded.actuals {
        definitions.push((*symbol, Some(*actual)));
    }
    let origins = callbacks::ReturnOrigins {
        semantic,
        stored,
        targets: &forwarded.targets,
        arrows: &forwarded.arrows,
    };
    let aliases = callbacks::named_targets(
        &origins,
        functions
            .iter()
            .filter(|f| f.async_signature.is_none())
            .map(|f| f.symbol),
        &definitions
            .iter()
            .filter_map(|(symbol, value)| value.map(|value| (*symbol, value)))
            .collect::<Vec<_>>(),
        &all_calls,
    );
    for call in all_calls {
        // Direct calls are already present. Every alias alternative contributes
        // argument evidence; this does not choose a runtime target.
        if function(call, semantic, functions).is_some() {
            continue;
        }
        let sources = origins
            .expression(invocation::callee(call), &BTreeMap::new(), 0, 0)
            .0;
        let mut targets = BTreeSet::<SymbolId>::new();
        for source in sources {
            if let Some(known) = aliases.get(&source) {
                targets.extend(known.iter().copied());
            }
        }
        for target in targets {
            if let Some(f) = functions.iter().find(|f| f.symbol == target && f.id.0 != 0) {
                calls.push((f.id.0, call));
            }
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
                .map(|v| kinds(v, semantic, functions, &bindings, &returns, None))
                .unwrap_or_else(|| [Kind::GeneralPrimitive].into());
            changed |= extend(&mut bindings, *symbol, next);
        }
        for (index, call) in &calls {
            for (symbol, actual) in parameters[*index].iter().zip(invocation::arguments(call)) {
                if let (Some(symbol), Some(actual)) = (symbol, actual.as_expression()) {
                    let next = kinds(actual, semantic, functions, &bindings, &returns, None);
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
                    .map(|v| kinds(v, semantic, functions, &bindings, &returns, None))
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

// Finite closure facts share the helper lattice; only actual lowering admits code.
#[derive(Default)]
pub(crate) struct ClosureKinds {
    pub(super) inputs: BTreeMap<SymbolId, Type>,
    pub(super) results: BTreeMap<NodeId, Type>,
    pub(super) calls: BTreeMap<NodeId, Type>,
    pub(super) identity_symbols: BTreeSet<SymbolId>,
    pub(super) identity_bodies: BTreeSet<NodeId>,
    pub(super) identity_calls: BTreeSet<NodeId>,
}
pub(super) struct ClosureFlow<'s, 'n, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub stored: &'s stored_functions::Fields,
    pub functions: &'s [NumericFunction],
    pub definitions: &'s BTreeMap<SymbolId, Option<&'n Expression<'a>>>,
    pub writes: &'s BTreeMap<SymbolId, Vec<&'n Expression<'a>>>,
    pub parameters: &'s BTreeSet<SymbolId>,
    pub groups: &'s BTreeSet<BTreeSet<NodeId>>,
    pub arrows: &'s BTreeMap<NodeId, &'n oxc_ast::ast::ArrowFunctionExpression<'a>>,
    pub calls: &'s BTreeMap<NodeId, BTreeSet<NodeId>>,
    pub targets: &'s BTreeMap<SymbolId, BTreeSet<NodeId>>,
    pub callback_parameters: &'s callbacks::Parameters,
    pub closure_signatures: &'s ir::ClosureSignatures,
}
pub(super) fn closure_kinds(flow: ClosureFlow<'_, '_, '_>) -> ClosureKinds {
    let ClosureFlow {
        semantic,
        stored,
        functions,
        definitions,
        writes,
        parameters,
        groups,
        arrows,
        calls,
        targets,
        callback_parameters,
        closure_signatures,
    } = flow;
    if !has_managed_origin(semantic) {
        return ClosureKinds::default();
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
    // Known unresolved formals and calls start at bottom, never an invented Number.
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
    let bodies = arrows
        .iter()
        .filter(|(_, arrow)| !arrow.r#async)
        .map(|(body, arrow)| {
            let complete = super::storage::arrow_returns(arrow, semantic);
            let missing = complete.is_none();
            let expressions = complete.unwrap_or_else(|| {
                super::block_results::return_statements(semantic, *body)
                    .into_iter()
                    .filter_map(|s| s.argument.as_ref())
                    .collect()
            });
            (*body, (expressions, missing))
        })
        .collect::<BTreeMap<_, _>>();
    let mut results: BTreeMap<_, Kinds> = bodies.keys().map(|body| (*body, Kinds::new())).collect();
    let mut call_kinds: BTreeMap<_, Kinds> =
        calls.keys().map(|call| (*call, Kinds::new())).collect();
    let origins = callbacks::ReturnOrigins {
        semantic,
        stored,
        targets,
        arrows,
    };
    let named = if functions
        .iter()
        .any(|f| f.async_signature.is_none() && f.return_type.is_managed())
    {
        let definitions = definitions
            .iter()
            .filter_map(|(s, e)| e.map(|e| (*s, e)))
            .chain(
                writes
                    .iter()
                    .flat_map(|(s, values)| values.iter().map(move |e| (*s, *e))),
            )
            .collect::<Vec<_>>();
        let calls = semantic
            .nodes()
            .iter()
            .filter_map(|n| match n.kind() {
                AstKind::CallExpression(c) => Some(c),
                _ => None,
            })
            .collect::<Vec<_>>();
        callbacks::named_targets(
            &origins,
            functions
                .iter()
                .filter(|f| f.async_signature.is_none())
                .map(|f| f.symbol),
            &definitions,
            &calls,
        )
    } else {
        BTreeMap::new()
    };
    // A checked callback signature is also positive evidence for its result
    // lane, including a synchronous module function passed as the actual.
    for node in semantic.nodes().iter() {
        if !named.is_empty()
            && let AstKind::CallExpression(call) = node.kind()
            && let Some(result) =
                input_flow::module_result(invocation::callee(call), &origins, &named, functions)
        {
            call_kinds
                .entry(call.node_id.get())
                .or_default()
                .extend(from_type(result));
        }
        if let AstKind::CallExpression(call) = node.kind()
            && let Some(Expression::Identifier(identifier)) =
                closures::unwrapped(invocation::callee(call), 0)
            && let Some(symbol) = symbol(identifier, semantic)
            && let Some(signature) = callback_parameters.values().find_map(|p| p.get(&symbol))
            && let Some(result) = closure_signatures.result(*signature)
            && result.is_managed()
        {
            call_kinds.insert(call.node_id.get(), from_type(result));
        }
    }
    loop {
        let mut changed = false;
        for (symbol, expression) in definitions {
            let next = expression
                .map(|e| {
                    kinds(
                        e,
                        semantic,
                        functions,
                        &bindings,
                        &returns,
                        Some(&call_kinds),
                    )
                })
                .unwrap_or_else(|| [Kind::GeneralPrimitive].into());
            changed |= extend(&mut bindings, *symbol, next);
        }
        for (symbol, expressions) in writes {
            for e in expressions {
                let next = kinds(
                    e,
                    semantic,
                    functions,
                    &bindings,
                    &returns,
                    Some(&call_kinds),
                );
                changed |= extend(&mut bindings, *symbol, next);
            }
        }
        for (body, (expressions, missing)) in &bodies {
            let mut next = Kinds::new();
            if *missing {
                next.insert(Kind::GeneralPrimitive);
            }
            for e in expressions {
                next.extend(kinds(
                    e,
                    semantic,
                    functions,
                    &bindings,
                    &returns,
                    Some(&call_kinds),
                ));
            }
            changed |= extend(&mut results, *body, next);
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
            let joined = group
                .iter()
                .filter_map(|body| results.get(body))
                .flatten()
                .copied()
                .collect::<Kinds>();
            for body in group {
                if !arrows[body].r#async {
                    changed |= extend(&mut results, *body, joined.clone());
                }
            }
        }
        for (call, targets) in calls {
            let mut next = Kinds::new();
            for target in targets {
                if arrows[target].r#async {
                    next.insert(Kind::Primitive);
                } else if let Some(kinds) = results.get(target) {
                    next.extend(kinds);
                }
            }
            changed |= extend(&mut call_kinds, *call, next);
        }
        if !changed {
            break;
        }
    }
    ClosureKinds {
        inputs: parameters
            .iter()
            .filter_map(|s| bindings.get(s).and_then(managed_type).map(|ty| (*s, ty)))
            .collect(),
        results: results
            .iter()
            .filter_map(|(body, kinds)| managed_type(kinds).map(|ty| (*body, ty)))
            .collect(),
        calls: call_kinds
            .iter()
            .filter_map(|(call, kinds)| managed_type(kinds).map(|ty| (*call, ty)))
            .collect(),
        ..ClosureKinds::default()
    }
}

// Identity demand is separate from scalar result kinds: a numeric closure may
// still escape as data. Follow the same finite constructor graph, without
// changing the calling convention of typed callback parameters.
pub(super) fn closure_identities(
    kinds: &mut ClosureKinds,
    origins: &callbacks::ReturnOrigins<'_, '_, '_>,
    calls: &BTreeMap<NodeId, BTreeSet<NodeId>>,
) {
    let observe = |kinds: &mut ClosureKinds, expression| {
        let (sources, mut bodies) = origins.expression(expression, &BTreeMap::new(), 0, 0);
        for source in &sources {
            if let Some(known) = origins.targets.get(source) {
                bodies.extend(known);
            }
        }
        if !sources.is_empty() || !bodies.is_empty() {
            // Record value-producing calls beneath wrappers and selectors too.
            // Their exact function signature remains checked during lowering.
            let mut pending = vec![(expression, 0)];
            while let Some((expression, depth)) = pending.pop() {
                if depth > 128 {
                    continue;
                }
                if let Some(inner) = erased_inner(expression) {
                    pending.push((inner, depth + 1));
                    continue;
                }
                match expression {
                    Expression::CallExpression(call) => {
                        kinds.identity_calls.insert(call.node_id.get());
                    }
                    Expression::ParenthesizedExpression(wrapper) => {
                        pending.push((&wrapper.expression, depth + 1))
                    }
                    Expression::ConditionalExpression(conditional) => {
                        pending.push((&conditional.consequent, depth + 1));
                        pending.push((&conditional.alternate, depth + 1));
                    }
                    Expression::LogicalExpression(logical) => {
                        pending.push((&logical.left, depth + 1));
                        pending.push((&logical.right, depth + 1));
                    }
                    Expression::SequenceExpression(sequence) => {
                        if let Some(last) = sequence.expressions.last() {
                            pending.push((last, depth + 1));
                        }
                    }
                    Expression::AssignmentExpression(assignment)
                        if assignment.operator == AssignmentOperator::Assign =>
                    {
                        pending.push((&assignment.right, depth + 1))
                    }
                    _ => (),
                }
            }
        }
        kinds.identity_symbols.extend(sources);
        kinds.identity_bodies.extend(bodies);
    };
    for node in origins.semantic.nodes().iter() {
        match node.kind() {
            AstKind::ObjectProperty(property) => observe(kinds, &property.value),
            AstKind::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(expression) = element.as_expression() {
                        observe(kinds, expression);
                    }
                }
            }
            AstKind::AssignmentExpression(assignment)
                if matches!(
                    assignment.left.as_simple_assignment_target(),
                    Some(
                        SimpleAssignmentTarget::StaticMemberExpression(_)
                            | SimpleAssignmentTarget::ComputedMemberExpression(_)
                    )
                ) =>
            {
                observe(kinds, &assignment.right);
            }
            AstKind::BinaryExpression(binary)
                if matches!(
                    binary.operator,
                    BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
                ) =>
            {
                observe(kinds, &binary.left);
                observe(kinds, &binary.right);
            }
            _ => (),
        }
    }
    for (body, arrow) in origins.arrows {
        if kinds.results.contains_key(body) {
            for statement in super::block_results::return_statements(origins.semantic, *body) {
                if let Some(expression) = &statement.argument {
                    observe(kinds, expression);
                }
            }
            if let Some(expression) = super::storage::arrow_returns(arrow, origins.semantic) {
                for expression in expression {
                    observe(kinds, expression);
                }
            }
        }
    }
    loop {
        let before = (
            kinds.identity_symbols.len(),
            kinds.identity_bodies.len(),
            kinds.identity_calls.len(),
        );
        for node in origins.semantic.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            for body in calls.get(&call.node_id.get()).into_iter().flatten() {
                for (parameter, actual) in origins.arrows[body]
                    .params
                    .items
                    .iter()
                    .zip(invocation::arguments(call))
                {
                    if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        && let Some(symbol) = identifier.symbol_id.get()
                        && (kinds.identity_symbols.contains(&symbol)
                            || kinds.inputs.contains_key(&symbol))
                        && let Some(expression) = actual.as_expression()
                    {
                        observe(kinds, expression);
                    }
                }
            }
        }
        if before
            == (
                kinds.identity_symbols.len(),
                kinds.identity_bodies.len(),
                kinds.identity_calls.len(),
            )
        {
            break;
        }
    }
}
