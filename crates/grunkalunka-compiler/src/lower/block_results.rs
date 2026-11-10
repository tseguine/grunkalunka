//! Known block arrows specialize into the caller while retaining typed early returns.
use super::*;
use oxc_ast::{AstKind, ast::ArrowFunctionExpression};
use oxc_syntax::node::NodeId;
use std::collections::BTreeMap;

// Shared by result-kind and callback-origin queries. The nearest function owns
// each return; nested bodies never provide evidence about their creator.
pub(super) fn return_statements<'a>(
    semantic: &Semantic<'a>,
    body: NodeId,
) -> Vec<&'a oxc_ast::ast::ReturnStatement<'a>> {
    semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::ReturnStatement(statement) = node.kind() else {
                return None;
            };
            let owner = semantic
                .nodes()
                .ancestor_kinds(node.id())
                .find_map(|kind| match kind {
                    AstKind::ArrowFunctionExpression(owner) => Some(owner.node_id.get()),
                    AstKind::Function(owner) => Some(owner.node_id.get()),
                    _ => None,
                });
            (owner == Some(body)).then_some(statement)
        })
        .collect()
}

// A side-effect-free true test may be moved to the back edge when the first
// iteration must exit. Restrict that proof to tail position so previously
// admitted fallback statements still undergo their ordinary lowering.
pub(super) fn terminal_entered_loop(semantic: &Semantic<'_>, id: NodeId) -> bool {
    let nodes = semantic.nodes();
    let test = match nodes.kind(id) {
        AstKind::WhileStatement(source) => Some(&source.test),
        AstKind::ForStatement(source) => source.test.as_ref(),
        _ => return false,
    };
    if let Some(test) = test {
        let truthy = match closures::unwrapped(test, 0) {
            Some(Expression::BooleanLiteral(value)) => value.value,
            Some(Expression::NumericLiteral(value)) => value.value != 0.0 && !value.value.is_nan(),
            _ => false,
        };
        if !truthy {
            return false;
        }
    }
    let mut child = id;
    loop {
        let parent = nodes.parent_id(child);
        let statements = match nodes.kind(parent) {
            AstKind::FunctionBody(body) => &body.statements,
            AstKind::BlockStatement(block) => &block.body,
            AstKind::IfStatement(_) => {
                child = parent;
                continue;
            }
            _ => return false,
        };
        if statements
            .iter()
            .rev()
            .find(|stmt| !inert_tail(stmt))
            .is_none_or(|last| last.node_id() != child)
        {
            return false;
        }
        if matches!(nodes.kind(parent), AstKind::FunctionBody(_)) {
            return true;
        }
        child = parent;
    }
}

// Representation planning uses the same completion algebra as lowering, but
// visits source statements with a heap worklist and never evaluates expressions.
// Unknown/unsupported statements still undergo the ordinary admission checks.
pub(super) fn normal_completion(
    semantic: &Semantic<'_>,
    function: &oxc_ast::ast::Function<'_>,
) -> bool {
    enum Task<'n, 'a> {
        Sequence(&'n [Statement<'a>]),
        Statement(&'n Statement<'a>),
        Constant(Completion),
        SequenceResult(usize),
        Union,
        Pretest,
        Do,
        Try { handler: bool, finalizer: bool },
        Switch { count: usize, default: bool },
    }
    let Some(body) = &function.body else {
        return false;
    };
    let mut work = vec![Task::Sequence(body.statements.as_slice())];
    let mut values: Vec<Completion> = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Task::Sequence(statements) => {
                work.push(Task::SequenceResult(statements.len()));
                work.extend(statements.iter().rev().map(Task::Statement));
            }
            Task::SequenceResult(count) => {
                let completion = values
                    .drain(values.len() - count..)
                    .fold(Completion::NORMAL, Completion::then);
                values.push(completion);
            }
            Task::Constant(completion) => values.push(completion),
            Task::Union => {
                let right = values.pop().unwrap();
                let left = values.pop().unwrap();
                values.push(left.union(right));
            }
            Task::Pretest => {
                let body = values.pop().unwrap();
                values.push(body.pretest_loop());
            }
            Task::Do => {
                let body = values.pop().unwrap();
                values.push(body.do_loop());
            }
            Task::Try { handler, finalizer } => {
                let finalizer = finalizer.then(|| values.pop().unwrap());
                let handler = handler.then(|| values.pop().unwrap());
                let mut body = values.pop().unwrap();
                if let Some(handler) = handler {
                    body = body.caught_by(handler);
                }
                if let Some(finalizer) = finalizer {
                    body = body.finalized_by(finalizer);
                }
                values.push(body);
            }
            Task::Switch { count, default } => {
                let mut suffix = Completion::NORMAL;
                let mut entries: Option<Completion> = None;
                for body in values.drain(values.len() - count..).rev() {
                    suffix = body.then(suffix);
                    entries = Some(entries.map_or(suffix, |all| all.union(suffix)));
                }
                let mut completion = entries.unwrap_or(Completion::NORMAL);
                if !default {
                    completion = completion.union(Completion::NORMAL);
                }
                completion.normal |= completion.breaks;
                completion.breaks = false;
                values.push(completion);
            }
            Task::Statement(statement) => match statement {
                Statement::ReturnStatement(_) => values.push(Completion::RETURN),
                Statement::ThrowStatement(_) => values.push(Completion::THROW),
                Statement::BreakStatement(_) => values.push(Completion::BREAK),
                Statement::LabeledStatement(label) => work.push(Task::Statement(&label.body)),
                Statement::ContinueStatement(_) => values.push(Completion::CONTINUE),
                Statement::BlockStatement(block) => {
                    work.push(Task::Sequence(block.body.as_slice()))
                }
                Statement::IfStatement(branch) => {
                    work.push(Task::Union);
                    if let Some(alternate) = &branch.alternate {
                        work.push(Task::Statement(alternate));
                    } else {
                        work.push(Task::Constant(Completion::NORMAL));
                    }
                    work.push(Task::Statement(&branch.consequent));
                }
                Statement::WhileStatement(loop_) => {
                    work.push(if terminal_entered_loop(semantic, loop_.node_id()) {
                        Task::Do
                    } else {
                        Task::Pretest
                    });
                    work.push(Task::Statement(&loop_.body));
                }
                Statement::ForStatement(loop_) => {
                    work.push(if terminal_entered_loop(semantic, loop_.node_id()) {
                        Task::Do
                    } else {
                        Task::Pretest
                    });
                    work.push(Task::Statement(&loop_.body));
                }
                Statement::ForOfStatement(loop_) => {
                    work.push(Task::Pretest);
                    work.push(Task::Statement(&loop_.body));
                }
                Statement::ForInStatement(loop_) => {
                    work.push(Task::Pretest);
                    work.push(Task::Statement(&loop_.body));
                }
                Statement::DoWhileStatement(loop_) => {
                    work.push(Task::Do);
                    work.push(Task::Statement(&loop_.body));
                }
                Statement::TryStatement(try_) => {
                    work.push(Task::Try {
                        handler: try_.handler.is_some(),
                        finalizer: try_.finalizer.is_some(),
                    });
                    if let Some(finalizer) = &try_.finalizer {
                        work.push(Task::Sequence(finalizer.body.as_slice()));
                    }
                    if let Some(handler) = &try_.handler {
                        work.push(Task::Sequence(handler.body.body.as_slice()));
                    }
                    work.push(Task::Sequence(try_.block.body.as_slice()));
                }
                Statement::SwitchStatement(switch) => {
                    work.push(Task::Switch {
                        count: switch.cases.len(),
                        default: switch.cases.iter().any(|case| case.test.is_none()),
                    });
                    work.extend(
                        switch
                            .cases
                            .iter()
                            .rev()
                            .map(|case| Task::Sequence(case.consequent.as_slice())),
                    );
                }
                _ => values.push(Completion::NORMAL),
            },
        }
    }
    assert_eq!(values.len(), 1);
    values[0].normal
}

impl Lowering<'_, '_> {
    // Only returns owned by this arrow participate; nested arrow bodies have their
    // own return contract. Shape recognition emits nothing and never runs effects.
    // Preserve full signature IDs; Number-leaf IDs alone coincide with arities.
    pub(super) fn block_function_arity(
        &self,
        arrow: &ArrowFunctionExpression<'_>,
        depth: usize,
    ) -> Option<usize> {
        self.block_function_arity_in(arrow, depth, &BTreeMap::new(), 0)
    }

    pub(super) fn block_function_arity_in(
        &self,
        arrow: &ArrowFunctionExpression<'_>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Option<usize> {
        if depth > 128 || arrow.get_expression().is_some() {
            return None;
        }
        let mut arity = None;
        for statement in return_statements(self.semantic, arrow.node_id.get()) {
            let next = self.block_expression_arity(
                statement.argument.as_ref()?,
                depth + 1,
                bindings,
                calls,
            )?;
            if arity.is_some_and(|expected| expected != next) {
                return None;
            }
            arity = Some(next);
        }
        arity
    }

    fn block_expression_arity(
        &self,
        expression: &Expression<'_>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Option<usize> {
        // Each leaf must prove the same complete closure signature. Preserve
        // the old per-node depth budget, but keep wrappers, branches and
        // initializer aliases on a heap worklist rather than the Rust stack.
        let mut pending = vec![(expression, depth)];
        let mut arity = None;
        while let Some((expression, depth)) = pending.pop() {
            if depth > 128 {
                return None;
            }
            if let Some(next) = self.function_arity_in(expression, depth, bindings, calls) {
                if arity.is_some_and(|expected| expected != next) {
                    return None;
                }
                arity = Some(next);
                continue;
            }
            if let Some(inner) = erased_inner(expression) {
                pending.push((inner, depth + 1));
                continue;
            }
            match expression {
                Expression::ParenthesizedExpression(wrapper) => {
                    pending.push((&wrapper.expression, depth + 1));
                }
                Expression::ConditionalExpression(conditional) => {
                    pending.push((&conditional.alternate, depth + 1));
                    pending.push((&conditional.consequent, depth + 1));
                }
                Expression::SequenceExpression(sequence) => {
                    pending.push((sequence.expressions.last()?, depth + 1));
                }
                Expression::AssignmentExpression(assignment)
                    if assignment.operator == AssignmentOperator::Assign =>
                {
                    pending.push((&assignment.right, depth + 1));
                }
                Expression::Identifier(identifier) => {
                    let symbol = self
                        .semantic
                        .scoping()
                        .get_reference(identifier.reference_id.get()?)
                        .symbol_id()?;
                    if let Some(kind) = bindings.get(&symbol) {
                        let next = (*kind)?;
                        if arity.is_some_and(|expected| expected != next) {
                            return None;
                        }
                        arity = Some(next);
                        continue;
                    }
                    let initializer = self.semantic.nodes().iter().find_map(|node| {
                        let AstKind::VariableDeclarator(declaration) = node.kind() else {
                            return None;
                        };
                        let BindingPattern::BindingIdentifier(binding) = &declaration.id else {
                            return None;
                        };
                        (binding.symbol_id.get() == Some(symbol))
                            .then_some(declaration.init.as_ref())
                            .flatten()
                    })?;
                    pending.push((initializer, depth + 1));
                }
                _ => return None,
            }
        }
        arity
    }
}
