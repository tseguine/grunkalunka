//! The shared, heap-owned source-control lowering scheduler.
//!
//! Source families enter the same loop through typed tasks and resumable frames.
//! Semantic transitions live in steps.rs rather than making this scheduler a
//! monolithic source-language interpreter. Expression/function lowering and
//! effect-free fact queries keep their existing, separate boundaries.
use super::*;
use oxc_ast::ast::{
    BlockStatement, DoWhileStatement, ForStatement, IfStatement, TryStatement, WhileStatement,
};

#[cfg(test)]
mod callback_tests;
#[cfg(test)]
mod callee_tests;
mod children;
#[cfg(test)]
mod closure_check_tests;
#[cfg(test)]
mod concise_factory_tests;
#[cfg(test)]
mod declaration_tests;
mod factories;
#[cfg(test)]
mod factory_call_tests;
#[cfg(test)]
mod factory_effect_tests;
#[cfg(test)]
mod factory_tests;
#[cfg(test)]
mod function_result_tests;
mod initializers;
#[cfg(test)]
mod runtime_factory_tests;
mod steps;

#[cfg(test)]
std::thread_local! {
    static BRIDGED_LOCAL_BODIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static SCHEDULED_FUNCTION_RETURNS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static SCHEDULED_DECLARATION_VALUES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static BRIDGED_BLOCK_FACTORIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static BRIDGED_CONCISE_FACTORIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static BRIDGED_FUNCTION_RESULT_VALUES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static BRIDGED_RUNTIME_FACTORIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static BRIDGED_CLOSURE_CHECKS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

type Next<'n, 'a> = source_control::StatementTask<'n, 'a, ControlNode<'n, 'a>>;

/// A declaration publishes one completed binding before selecting its next
/// initializer. The child expression owns the output Region while suspended.
struct DeclarationSequence<'n, 'a> {
    source: &'n VariableDeclaration<'a>,
    index: usize,
    depth: usize,
}

#[derive(Clone, Copy)]
pub(super) enum ControlNode<'n, 'a> {
    If(&'n IfStatement<'a>),
    While(&'n WhileStatement<'a>),
    DoWhile(&'n DoWhileStatement<'a>),
    For(&'n ForStatement<'a>),
    Block(&'n BlockStatement<'a>),
    /// A lexical statement sequence is a traversal boundary, not a source node.
    Sequence(&'n [Statement<'a>]),
    TryCatch(&'n TryStatement<'a>),
    TryFinally(&'n TryStatement<'a>),
}

impl<'n, 'a> ControlNode<'n, 'a> {
    fn of_statement(source: &'n Statement<'a>) -> Option<Self> {
        match source {
            Statement::IfStatement(statement) => Some(Self::If(statement)),
            Statement::WhileStatement(statement) => Some(Self::While(statement)),
            Statement::DoWhileStatement(statement) => Some(Self::DoWhile(statement)),
            Statement::ForStatement(statement) => Some(Self::For(statement)),
            Statement::BlockStatement(statement) => Some(Self::Block(statement)),
            Statement::TryStatement(statement) => Some(if statement.finalizer.is_some() {
                Self::TryFinally(statement)
            } else {
                Self::TryCatch(statement)
            }),
            _ => None,
        }
    }
}

/// The for header is lowered; an explicit source expression request will
/// produce the test predicate before the loop body Region is allocated.
struct ForPredicateFrame<'n, 'a> {
    source: &'n ForStatement<'a>,
    output: Region,
    header: Region,
    depth: usize,
    outer: usize,
    header_bindings: usize,
    let_slots: Vec<SlotId>,
}

/// For-loop-specific continuation data is independent of the scheduler.
/// The corresponding lowering transitions live in steps/for_loop.rs.
struct ForFrame<'n, 'a> {
    source: &'n ForStatement<'a>,
    output: Region,
    header: Region,
    condition: Region,
    depth: usize,
    outer: usize,
    header_bindings: usize,
    let_slots: Vec<SlotId>,
    old_loop: bool,
}

/// Both function-valued expressions and immediately invoked returned functions
/// suspend the same evaluator while a factory runs. Only the final consumer
/// differs; keep its function-value charge and resume action explicit.
enum FactoryResume<'n, 'a> {
    Value,
    Callee(evaluate::SyncCalleeContinuation<'n, 'a>),
}

/// Function-mode children share region, source-depth and admission ownership.
/// Only the consuming evaluator action and callback-specific entry guard differ.
enum FunctionValueResume {
    Effect(evaluate::FunctionEffect),
    Callback,
    Callee,
}

struct FactoryExpression<'n, 'a> {
    expression: Box<evaluate::EvaluationCursor<'n, 'a>>,
    mode: evaluate::Mode,
    previous_result_depth: usize,
    node: &'n Expression<'a>,
    signature: usize,
    resume: FactoryResume<'n, 'a>,
}

struct FactoryArguments<'n, 'a> {
    state: FactoryExpression<'n, 'a>,
    factory: function_values::PreparedFactory<'n, 'a>,
    call: &'n CallExpression<'a>,
    expression_depth: usize,
    index: usize,
    values: Vec<ValueId>,
}

/// A boundary owns exactly one suspended Lowering in Cursor's LIFO state stack.
struct LoweringChildId(usize);

/// Checking completion is distinct from a future lifted-body publication.
enum ChildCompletion {
    DefinitionCheck { closure: closures::LocalClosure },
}

enum Frame<'n, 'a> {
    /// Restore this parent before applying its semantic child completion or
    /// unwinding any older source/evaluator frame that belongs to the parent.
    ChildBoundary {
        child: LoweringChildId,
        completion: ChildCompletion,
        depth: usize,
    },
    CheckedLocalCall {
        expression: Box<evaluate::EvaluationCursor<'n, 'a>>,
        mode: evaluate::Mode,
        continuation: evaluate::LocalCallPreparation<'n, 'a>,
        depth: usize,
    },
    CheckExpression {
        function_value: bool,
        depth: usize,
    },
    CheckBlock {
        activation: closures::BlockArrowActivation,
        factory: bool,
        depth: usize,
    },
    CheckReturnedClosure {
        output: Region,
        depth: usize,
    },
    /// A charged expression statement awaits a Discard-mode completion.
    DiscardExpression {
        depth: usize,
    },
    /// Directly evaluated return expressions resume with their typed result.
    ReturnValue {
        depth: usize,
    },
    /// An explicitly admitted closure-valued return keeps its function-value
    /// resource charge until the scheduled expression finishes or unwinds.
    FunctionReturn {
        depth: usize,
    },
    /// Async source throw produces the original General/AsyncValue reason.
    ThrowValue {
        depth: usize,
    },
    /// One admitted initializer is pending; its token owns any function-value
    /// charge and the sequence retains per-declarator publication order.
    DeclarationValue {
        sequence: Box<DeclarationSequence<'n, 'a>>,
        active: declarations::ActiveDeclarationValue<'n, 'a>,
    },
    /// The If source is suspended until its condition yields a predicate.
    IfPredicate {
        source: &'n IfStatement<'a>,
        depth: usize,
    },
    If {
        source: &'n IfStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        condition: ValueId,
        consequent: Option<Region>,
        consequent_returns: Option<Completion>,
    },
    /// The loop condition owns a child Region until its predicate returns.
    WhilePredicate {
        source: &'n WhileStatement<'a>,
        output: Region,
        depth: usize,
    },
    While {
        source: &'n WhileStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        old_loop: bool,
        condition: Region,
    },
    DoWhile {
        source: &'n DoWhileStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        old_loop: bool,
    },
    /// The body has completed; the do-while test is now awaited.
    DoWhilePredicate {
        source: &'n DoWhileStatement<'a>,
        output: Region,
        depth: usize,
        body: Region,
        body_completion: Completion,
    },
    /// A for initializer expression runs in its lexical header Region.
    ForInitializer {
        source: &'n ForStatement<'a>,
        output: Region,
        depth: usize,
        outer: usize,
    },
    /// A declaration completes its ordered initializers before header bindings
    /// are collected for captured let-cell renewal.
    ForDeclarationInitializer {
        source: &'n ForStatement<'a>,
        output: Region,
        depth: usize,
        outer: usize,
        start: usize,
        mutable: bool,
    },
    ForPredicate(ForPredicateFrame<'n, 'a>),
    For(ForFrame<'n, 'a>),
    /// The loop body has finished; an update expression must run before
    /// captured let-header renewal and final loop IR assembly.
    ForUpdate {
        frame: ForFrame<'n, 'a>,
        body: Region,
        completion: Completion,
    },
    Block {
        source: &'n BlockStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
    },
    Sequence {
        statements: &'n [Statement<'a>],
        index: usize,
        depth: usize,
        completion: Completion,
    },
    /// An evaluator has yielded a block-bodied local-call source request.
    /// Keep its Work/typed values and the function activation off the Rust
    /// stack while the same source cursor lowers the nested statement body.
    EvaluateBody {
        expression: Box<evaluate::EvaluationCursor<'n, 'a>>,
        mode: evaluate::Mode,
        activation: closures::BlockArrowActivation,
        node: &'n Expression<'a>,
        result_type: Type,
        depth: usize,
    },
    /// A function construction or callback argument suspends its evaluator
    /// while the Function-mode child runs on this same source scheduler.
    EvaluateFunctionValue {
        expression: Box<evaluate::EvaluationCursor<'n, 'a>>,
        mode: evaluate::Mode,
        node: &'n Expression<'a>,
        resume: FunctionValueResume,
        previous_result_depth: usize,
        depth: usize,
    },
    /// Runtime result callees retain their ordinary function-value admission
    /// and result token before the shared caller-argument phase can start.
    EvaluateRuntimeFactoryCallee {
        state: FactoryExpression<'n, 'a>,
        factory: function_values::PreparedRuntimeFactory<'n, 'a>,
        depth: usize,
    },
    /// One source argument is running in the caller's lexical environment.
    /// A callback holds its own function-value admission across suspension.
    EvaluateFactoryArgument {
        arguments: Box<FactoryArguments<'n, 'a>>,
        kind: closures::FactoryArgumentKind,
        depth: usize,
    },
    /// A source-owned function-valued call is waiting for its block-bodied
    /// factory to complete. This owns all three nested checkpoints.
    EvaluateFactoryBody {
        state: FactoryExpression<'n, 'a>,
        activation: closures::BlockArrowActivation,
        factory: function_values::LocalFunctionResult,
        depth: usize,
    },
    /// A general concise factory keeps its lexical bindings, result admission
    /// and body function-value admission until the expression returns a value.
    EvaluateFactoryValue {
        state: FactoryExpression<'n, 'a>,
        factory: function_values::LocalFunctionResult,
        depth: usize,
    },
    TryCatchBody {
        source: &'n TryStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
    },
    TryCatchHandler {
        source: &'n TryStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        body: Region,
        body_completion: Completion,
        reason: ValueId,
    },
    TryFinallyBody {
        source: &'n TryStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        outer_type: Type,
    },
    TryFinallyCleanup {
        source: &'n TryStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        outer_type: Type,
        body: Region,
        body_completion: Completion,
    },
}

impl Frame<'_, '_> {
    fn depth(&self) -> usize {
        match self {
            Self::ChildBoundary { depth, .. }
            | Self::CheckedLocalCall { depth, .. }
            | Self::CheckExpression { depth, .. }
            | Self::CheckBlock { depth, .. }
            | Self::CheckReturnedClosure { depth, .. } => *depth,
            Self::DiscardExpression { depth }
            | Self::ReturnValue { depth }
            | Self::FunctionReturn { depth }
            | Self::ThrowValue { depth }
            | Self::IfPredicate { depth, .. }
            | Self::If { depth, .. }
            | Self::WhilePredicate { depth, .. }
            | Self::While { depth, .. }
            | Self::DoWhile { depth, .. }
            | Self::DoWhilePredicate { depth, .. }
            | Self::Block { depth, .. }
            | Self::Sequence { depth, .. }
            | Self::EvaluateBody { depth, .. }
            | Self::EvaluateFunctionValue { depth, .. }
            | Self::EvaluateRuntimeFactoryCallee { depth, .. }
            | Self::EvaluateFactoryArgument { depth, .. }
            | Self::EvaluateFactoryBody { depth, .. }
            | Self::EvaluateFactoryValue { depth, .. }
            | Self::TryCatchBody { depth, .. }
            | Self::TryCatchHandler { depth, .. }
            | Self::TryFinallyBody { depth, .. }
            | Self::TryFinallyCleanup { depth, .. } => *depth,
            Self::ForInitializer { depth, .. } => *depth,
            Self::ForDeclarationInitializer { depth, .. } => *depth,
            Self::DeclarationValue { sequence, .. } => sequence.depth,
            Self::ForPredicate(frame) => frame.depth,
            Self::For(frame) => frame.depth,
            Self::ForUpdate { frame, .. } => frame.depth,
        }
    }
}

/// A source continuation can complete an ordinary statement or deliver a
/// requested value. The optional expression value also represents Discard mode.
enum Outcome {
    Statement(Region, Completion),
    Expression(Region, Option<ValueId>),
    CheckBody { region: Region, factory: bool },
    CheckedClosure(closures::LocalClosure),
}

/// One active source-lowering machine. Language-specific transitions own
/// their variant-specific Frame data but never allocate another scheduler.
struct Cursor<'s, 'n, 'a> {
    pending: Vec<Frame<'n, 'a>>,
    next: Option<Next<'n, 'a>>,
    completed: Option<Outcome>,
    // Reuse the large child's preparation box for its suspended parent; stack
    // growth moves only pointers, and restoration swaps into the active state.
    #[allow(clippy::vec_box)]
    parents: Vec<Box<Lowering<'s, 'a>>>,
}

impl<'s, 'n, 'a> Cursor<'s, 'n, 'a> {
    fn new(root: ControlNode<'n, 'a>, output: Region, depth: usize) -> Self {
        Self {
            pending: Vec::new(),
            next: Some(Next::Enter {
                source: root,
                output,
                depth,
                charge: source_control::SourceCharge::Charged,
            }),
            completed: None,
            parents: Vec::new(),
        }
    }
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    /// Select only return paths that previously called the existing evaluator
    /// directly. Closure-valued returns and async General resolution have
    /// extra context/adoption semantics and retain their separate handlers.
    fn direct_return_mode(&self) -> Option<evaluate::Mode> {
        Some(match self.return_type {
            Type::Number | Type::Predicate => evaluate::Mode::Number,
            Type::General => {
                if self.allow_await {
                    evaluate::Mode::AsyncReturn
                } else {
                    evaluate::Mode::General
                }
            }
            Type::AsyncValue => evaluate::Mode::AsyncValue,
            Type::Promise => evaluate::Mode::Pending,
            Type::SettledPromise => evaluate::Mode::Settled,
            Type::FulfilledPromise => evaluate::Mode::Promise,
            Type::ManagedValue => evaluate::Mode::Managed,
            Type::ManagedObject => evaluate::Mode::Object,
            Type::ManagedArray => evaluate::Mode::Array,
            Type::Closure(_) => return None,
        })
    }

    /// Drive one owned expression activation until it yields a source-body
    /// request or completes. The source scheduler owns any yielded activation
    /// and continues without recursively entering another source cursor.
    #[inline(never)]
    fn advance_owned_expression<'n>(
        &mut self,
        mut expression: evaluate::EvaluationCursor<'n, 'ast>,
        mode: evaluate::Mode,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        loop {
            match self.run_evaluation(&mut expression, mode)? {
                evaluate::EvaluationRun::Complete(value) => {
                    cursor.completed = Some(Outcome::Expression(expression.into_region(), value));
                    return Ok(());
                }
                evaluate::EvaluationRun::CheckLocalClosure {
                    closure,
                    continuation,
                } => {
                    cursor.pending.push(Frame::CheckedLocalCall {
                        expression: Box::new(expression),
                        mode,
                        continuation,
                        depth: self.statement_depth,
                    });
                    return self.start_closure_check(closure, 0, cursor);
                }
                evaluate::EvaluationRun::LowerFunctionLeaf {
                    node,
                    signature,
                    depth,
                    region_depth,
                } => {
                    let previous_result_depth = self.enter_function_result_depth(region_depth);
                    let prepared = self.source_function_value_leaf(
                        node,
                        expression.current_region(),
                        signature,
                        depth,
                    );
                    match prepared {
                        Ok(function_values::FunctionLeafResult::Factory(factory)) => {
                            return self.start_factory_arguments(
                                FactoryExpression {
                                    expression: Box::new(expression),
                                    mode,
                                    previous_result_depth,
                                    node,
                                    signature,
                                    resume: FactoryResume::Value,
                                },
                                factory,
                                depth,
                                cursor,
                            );
                        }
                        Ok(function_values::FunctionLeafResult::RuntimeFactory(factory)) => {
                            return self.start_runtime_factory_callee(
                                FactoryExpression {
                                    expression: Box::new(expression),
                                    mode,
                                    previous_result_depth,
                                    node,
                                    signature,
                                    resume: FactoryResume::Value,
                                },
                                factory,
                                cursor,
                            );
                        }
                        Ok(function_values::FunctionLeafResult::Value(value)) => {
                            self.restore_function_result_depth(previous_result_depth);
                            expression.resume_function_value(value);
                        }
                        Err(error) => {
                            self.restore_function_result_depth(previous_result_depth);
                            return Err(error);
                        }
                    }
                }
                request @ (evaluate::EvaluationRun::LowerFunctionEffect { .. }
                | evaluate::EvaluationRun::LowerCallbackArgument { .. }
                | evaluate::EvaluationRun::LowerClosureCallee { .. }) => {
                    let (node, signature, depth, region_depth, resume) = match request {
                        evaluate::EvaluationRun::LowerFunctionEffect {
                            node,
                            signature,
                            depth,
                            region_depth,
                            effect,
                        } => (
                            node,
                            signature,
                            depth,
                            region_depth,
                            FunctionValueResume::Effect(effect),
                        ),
                        evaluate::EvaluationRun::LowerCallbackArgument {
                            node,
                            signature,
                            depth,
                            region_depth,
                        } => (
                            node,
                            signature,
                            depth,
                            region_depth,
                            FunctionValueResume::Callback,
                        ),
                        evaluate::EvaluationRun::LowerClosureCallee {
                            node,
                            signature,
                            depth,
                            region_depth,
                        } => (
                            node,
                            signature,
                            depth,
                            region_depth,
                            FunctionValueResume::Callee,
                        ),
                        _ => unreachable!("matched a function-value request"),
                    };
                    let previous_result_depth =
                        self.enter_function_result_depth(Some(region_depth));
                    let admitted = match resume {
                        FunctionValueResume::Effect(_) | FunctionValueResume::Callee => {
                            self.enter_function_value(node)
                        }
                        FunctionValueResume::Callback => self.enter_callback_argument(node),
                    };
                    if let Err(error) = admitted {
                        self.restore_function_result_depth(previous_result_depth);
                        return Err(error);
                    }
                    let output = Self::take_cursor_region(expression.current_region());
                    cursor.pending.push(Frame::EvaluateFunctionValue {
                        expression: Box::new(expression),
                        mode,
                        node,
                        resume,
                        previous_result_depth,
                        depth: self.statement_depth,
                    });
                    cursor.next = Some(Next::Expression {
                        source: node,
                        output,
                        depth,
                        mode: evaluate::Mode::Function(signature),
                    });
                    return Ok(());
                }
                evaluate::EvaluationRun::LowerSyncClosureCallee {
                    node,
                    call,
                    signature,
                    depth,
                    region_depth,
                    mode: outer_mode,
                    result_type,
                    number_proven,
                } => {
                    let callee_node = invocation::callee(call);
                    let callee_depth = invocation::depth(call, depth);
                    let previous_result_depth =
                        self.enter_function_result_depth(Some(region_depth));
                    if let Err(error) = self.enter_function_value(callee_node) {
                        self.restore_function_result_depth(previous_result_depth);
                        return Err(error);
                    }
                    let prepared = self.prepare_local_function_result(
                        callee_node,
                        expression.current_region(),
                        callee_depth,
                    );
                    match prepared {
                        Ok(Some(factory)) => {
                            return self.start_factory_arguments(
                                FactoryExpression {
                                    expression: Box::new(expression),
                                    mode,
                                    previous_result_depth,
                                    node: callee_node,
                                    signature,
                                    resume: FactoryResume::Callee(
                                        evaluate::SyncCalleeContinuation {
                                            node,
                                            call,
                                            depth,
                                            mode: outer_mode,
                                            result_type,
                                            number_proven,
                                        },
                                    ),
                                },
                                function_values::PreparedFactory::LocalResult(factory),
                                callee_depth,
                                cursor,
                            );
                        }
                        Ok(None) => {
                            // Preserve the legacy path if the read-only
                            // factory shape no longer selects this source.
                            self.leave_function_value();
                            self.restore_function_result_depth(previous_result_depth);
                            let callee = self.prepare_closure_call(
                                node,
                                call,
                                signature,
                                expression.current_region(),
                                depth,
                                region_depth,
                            )?;
                            self.resume_sync_closure_callee(
                                &mut expression,
                                evaluate::SyncCalleeContinuation {
                                    node,
                                    call,
                                    depth,
                                    mode: outer_mode,
                                    result_type,
                                    number_proven,
                                },
                                callee,
                            )?;
                        }
                        Err(error) => {
                            self.leave_function_value();
                            self.restore_function_result_depth(previous_result_depth);
                            return Err(error);
                        }
                    }
                }
                evaluate::EvaluationRun::LowerBody {
                    node,
                    arrow,
                    result_type,
                    parent_depth,
                } => {
                    let arrow = self.arrow(arrow);
                    let (activation, request) =
                        self.begin_arrow_block_activation(arrow, parent_depth, result_type)?;
                    #[cfg(test)]
                    BRIDGED_LOCAL_BODIES.with(|count| count.set(count.get() + 1));
                    cursor.pending.push(Frame::EvaluateBody {
                        expression: Box::new(expression),
                        mode,
                        activation,
                        node,
                        result_type,
                        depth: 0,
                    });
                    cursor.next = Some(Next::Enter {
                        source: ControlNode::Sequence(request.statements),
                        output: request.region,
                        depth: 0,
                        charge: source_control::SourceCharge::Charged,
                    });
                    return Ok(());
                }
            }
        }
    }

    #[inline(never)]
    pub(super) fn control_statement_iterative<'n>(
        &mut self,
        root: ControlNode<'n, 'ast>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String>
    where
        'ast: 'n,
    {
        #[cfg(test)]
        let _entry = crate::source_cursor_tests::entry();
        // The root has already been charged by statement_inner. Every direct
        // nested control node is charged exactly once when its Enter runs.
        let saved_context = self.source_context();
        let original_depth = saved_context.statement_depth;
        let root_output = Self::take_cursor_region(region);
        let mut cursor = Cursor::new(root, root_output, depth);
        let result = (|| {
            loop {
                #[cfg(test)]
                crate::source_cursor_tests::pending(cursor.pending.len());
                if let Some(task) = cursor.next.take() {
                    match task {
                        Next::Enter {
                            source,
                            output,
                            depth,
                            charge,
                        } => {
                            self.statement_depth = depth;
                            if charge == source_control::SourceCharge::Uncharged {
                                match source {
                                    ControlNode::If(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::While(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::DoWhile(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::For(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::Block(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::Sequence(_) => {}
                                    ControlNode::TryCatch(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::TryFinally(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                }
                            }

                            self.begin_control(source, output, depth, &mut cursor)?;
                        }
                        Next::Statement {
                            source,
                            mut output,
                            depth,
                        } => {
                            if let Some(node) = ControlNode::of_statement(source) {
                                cursor.next = Some(Next::Enter {
                                    source: node,
                                    output,
                                    depth,
                                    charge: source_control::SourceCharge::Uncharged,
                                });
                            } else if let Statement::ThrowStatement(statement) = source {
                                self.statement_depth = depth;
                                self.admit_source_statement(source, depth)?;
                                if !self.allow_await {
                                    return Err(unsupported(
                                        statement.as_ref(),
                                        "statement is outside the numeric experiment",
                                    ));
                                }
                                let mode = if self.promise_parameters.identities
                                    && self.has_promise_choice(&statement.argument, 0)
                                {
                                    evaluate::Mode::AsyncValue
                                } else {
                                    evaluate::Mode::General
                                };
                                cursor.pending.push(Frame::ThrowValue { depth });
                                cursor.next = Some(Next::Expression {
                                    source: &statement.argument,
                                    output,
                                    depth: 0,
                                    mode,
                                });
                            } else if let Statement::ReturnStatement(statement) = source
                                && statement.argument.is_some()
                                && let Type::Closure(arity) = self.return_type
                            {
                                self.statement_depth = depth;
                                self.admit_source_statement(source, depth)?;
                                let expression =
                                    statement.argument.as_ref().expect("checked return");
                                self.enter_function_value(expression)?;
                                #[cfg(test)]
                                SCHEDULED_FUNCTION_RETURNS.with(|count| count.set(count.get() + 1));
                                cursor.pending.push(Frame::FunctionReturn { depth });
                                cursor.next = Some(Next::Expression {
                                    source: expression,
                                    output,
                                    depth: 0,
                                    mode: evaluate::Mode::Function(arity),
                                });
                            } else if let Statement::ReturnStatement(statement) = source
                                && statement.argument.is_some()
                                && let Some(mode) = self.direct_return_mode()
                            {
                                self.statement_depth = depth;
                                self.admit_source_statement(source, depth)?;
                                cursor.pending.push(Frame::ReturnValue { depth });
                                cursor.next = Some(Next::Expression {
                                    source: statement.argument.as_ref().expect("checked return"),
                                    output,
                                    depth: 0,
                                    mode,
                                });
                            } else if let Statement::VariableDeclaration(declaration) = source
                                && !is_type_declaration(source)
                            {
                                self.statement_depth = depth;
                                self.admit_source_statement(source, depth)?;
                                self.start_declaration(declaration, output, depth, &mut cursor)?;
                            } else if let Statement::ExpressionStatement(statement) = source {
                                // A discard expression is another typed leaf
                                // request. Preserve the original statement
                                // depth and source-charge order before it runs.
                                self.statement_depth = depth;
                                self.admit_source_statement(source, depth)?;
                                cursor.pending.push(Frame::DiscardExpression { depth });
                                cursor.next = Some(Next::Expression {
                                    source: &statement.expression,
                                    output,
                                    depth: 0,
                                    mode: evaluate::Mode::Discard,
                                });
                            } else {
                                // The child may still run a source function body
                                // through the existing expression evaluator.
                                self.statement_depth =
                                    cursor.pending.last().map_or(original_depth, Frame::depth);
                                let completion = self.statement(source, &mut output, depth)?;
                                cursor.completed = Some(Outcome::Statement(output, completion));
                            }
                        }
                        Next::Expression {
                            source,
                            output,
                            depth,
                            mode,
                        } => {
                            let expression =
                                evaluate::EvaluationCursor::new(output, source, depth, mode);
                            self.advance_owned_expression(expression, mode, &mut cursor)?;
                        }
                    }
                    continue;
                }

                let completed = cursor.completed.take().expect("completed source child");
                match completed {
                    Outcome::CheckBody { region, factory } => {
                        let frame = cursor.pending.pop().expect("checking child boundary");
                        // This boundary must restore its parent before any
                        // parent statement-depth write or continuation runs.
                        self.finish_checking_child(frame, region, factory, &mut cursor);
                    }
                    Outcome::CheckedClosure(closure) => {
                        let frame = cursor.pending.pop().expect("checked closure consumer");
                        self.statement_depth = frame.depth();
                        self.resume_checked_closure(frame, closure, &mut cursor)?;
                    }
                    Outcome::Statement(finished_region, completion) => {
                        let Some(frame) = cursor.pending.pop() else {
                            *region = finished_region;
                            return Ok(completion);
                        };
                        self.statement_depth = frame.depth();
                        self.resume_control(frame, finished_region, completion, &mut cursor)?;
                    }
                    Outcome::Expression(finished_region, value) => {
                        let frame = cursor
                            .pending
                            .pop()
                            .expect("expression needs a waiting source continuation");
                        self.statement_depth = frame.depth();
                        self.resume_expression(frame, finished_region, value, &mut cursor)?;
                    }
                }
            }
        })();
        // A diagnostic may unwind through nested function bodies before their
        // evaluator resumes. Restore those block-arrow activation checkpoints
        // in reverse nesting order rather than leaking function/lexical modes.
        if result.is_err() {
            while let Some(frame) = cursor.pending.pop() {
                match frame {
                    Frame::ChildBoundary { child, .. } => {
                        cursor.leave_lowering_child(self, child);
                    }
                    Frame::CheckExpression {
                        function_value: true,
                        ..
                    } => {
                        self.leave_function_value();
                    }
                    Frame::CheckBlock { activation, .. } => {
                        self.abort_arrow_block_activation(activation);
                    }
                    Frame::EvaluateBody { activation, .. } => {
                        self.abort_arrow_block_activation(activation);
                    }
                    Frame::EvaluateFunctionValue {
                        previous_result_depth,
                        ..
                    } => {
                        self.leave_function_value();
                        self.restore_function_result_depth(previous_result_depth);
                    }
                    Frame::EvaluateFactoryArgument {
                        arguments, kind, ..
                    } => {
                        self.abort_factory_argument(*arguments, kind);
                    }
                    Frame::EvaluateRuntimeFactoryCallee { state, factory, .. } => {
                        self.leave_function_value();
                        self.abort_runtime_factory(factory);
                        self.restore_factory_expression(&state);
                    }
                    Frame::EvaluateFactoryBody {
                        state,
                        activation,
                        factory,
                        ..
                    } => {
                        self.abort_arrow_block_activation(activation);
                        self.finish_local_function_result(factory);
                        self.restore_factory_expression(&state);
                    }
                    Frame::EvaluateFactoryValue { state, factory, .. } => {
                        self.leave_function_value();
                        self.finish_local_function_result(factory);
                        self.restore_factory_expression(&state);
                    }
                    Frame::FunctionReturn { .. } => {
                        self.leave_function_value();
                    }
                    Frame::DeclarationValue { active, .. } => {
                        self.abort_declaration_value(active);
                    }
                    _ => {}
                }
            }
        }
        debug_assert!(
            cursor.parents.is_empty(),
            "balanced child Lowering ownership"
        );
        self.restore_source_context(saved_context);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_argument_returned_block_factories_use_the_same_source_cursor() {
        for (body, admitted) in [
            ("const get=()=>{return ()=>a;};return get()();", true),
            (
                "const get=()=>{if(a)return ()=>a;return ()=>b;};return get()();",
                true,
            ),
            ("const get=()=>{return ()=>a+b;};return get()()+b;", true),
            ("const get=()=>{if(a)return ()=>a;};return get()();", false),
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let (result, bridged) = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || {
                        BRIDGED_BLOCK_FACTORIES.with(|count| count.set(0));
                        let result = crate::compile(&source, typescript);
                        let bridged = BRIDGED_BLOCK_FACTORIES.with(std::cell::Cell::get);
                        (result, bridged)
                    })
                    .expect("2 MiB returned block factory thread")
                    .join()
                    .expect("returned block factory must not overflow native stack");
                assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
                if admitted {
                    assert!(
                        bridged > 0,
                        "{body}: factory still reentered a source cursor"
                    );
                }
            }
        }
    }

    #[test]
    fn async_general_returns_resume_through_the_owned_source_evaluator() {
        for body in [
            "return a;",
            "return await a;",
            "const p=Promise.resolve(a);return p;",
            "const p=Promise.resolve(a);return await p;",
            "const f=x=>{return x+b;};return f(a);",
        ] {
            for typescript in [false, true] {
                let source = format!("export async function run(a,b){{{body}}}");
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, typescript))
                    .expect("2 MiB async return compiler thread")
                    .join()
                    .expect("async return resolution must not overflow native stack");
                assert!(result.is_ok(), "{body}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn general_let_values_use_the_same_owned_source_evaluator() {
        for body in [
            "let value=null;return value;",
            "let value=a?false:null;return value;",
            "let value=undefined;return value;",
            "const f=x=>{return x?null:false;};let value=f(a);return value;",
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_value(&source, typescript))
                    .expect("2 MiB General let compiler thread")
                    .join()
                    .expect("General let initialization must not overflow Rust stack");
                assert!(result.is_ok(), "{body}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn function_valued_returns_share_the_owned_expression_cursor() {
        for body in [
            "const make=x=>{return ()=>x+a;};return make(b)();",
            "const make=x=>{if(x)return ()=>x+a;return ()=>x-b;};return make(a)();",
            "const make=x=>{return y=>{return ()=>x+y;};};return make(a)(b)();",
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let (result, scheduled) = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || {
                        SCHEDULED_FUNCTION_RETURNS.with(|count| count.set(0));
                        let result = crate::compile(&source, typescript);
                        let scheduled = SCHEDULED_FUNCTION_RETURNS.with(std::cell::Cell::get);
                        (result, scheduled)
                    })
                    .expect("2 MiB function return compiler thread")
                    .join()
                    .expect("function-valued return must not overflow native stack");
                assert!(result.is_ok(), "{body}: {:?}", result.err());
                assert!(
                    scheduled > 0,
                    "{body}: function result used the legacy return path"
                );
            }
        }
        for typescript in [false, true] {
            let source =
                "export function run(a,b){const make=x=>{if(x)return ()=>x+a;};return make(b)();}";
            assert!(crate::compile(source, typescript).is_err());
        }
    }

    #[test]
    fn numeric_let_initializers_join_the_owned_source_evaluator_cursor() {
        // A local block-arrow call in a variable initializer used to enter a
        // second source cursor through statement_inner -> declaration -> evaluate.
        for body in [
            "const f=x=>{return x+a;};let n=f(b);return n;",
            "const f=x=>{if(x)return x+a;return b;};let n=f(a);return n;",
            "const f=x=>{return x+a;};let n=f(b);n=f(n);return n;",
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let (compiled, bridged) = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || {
                        BRIDGED_LOCAL_BODIES.with(|count| count.set(0));
                        let compiled = crate::compile(&source, typescript);
                        let bridged = BRIDGED_LOCAL_BODIES.with(std::cell::Cell::get);
                        (compiled, bridged)
                    })
                    .expect("2 MiB numeric let compiler thread")
                    .join()
                    .expect("numeric let initializer must not exhaust Rust stack");
                assert!(compiled.is_ok(), "{body}: {:?}", compiled.err());
                assert!(
                    bridged > 0,
                    "{body}: initializer bypassed the owned scheduler"
                );
            }
        }
        for body in [
            "let n=a**b;return n;",
            "const f=x=>{if(x)return x;};let n=f(b);return n;",
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                assert!(crate::compile(&source, typescript).is_err(), "{body}");
            }
        }
    }

    #[test]
    fn block_bodied_local_calls_share_the_source_cursor_on_two_mib_stack() {
        for (body, admitted) in [
            ("const f=x=>{return x+a;};return f(b);", true),
            ("const f=x=>{if(x)return x+a;return b;};return f(a);", true),
            ("const f=x=>{return x+a;};return f(f(b));", true),
            ("const f=x=>{if(x)return x;};return f(a);", false),
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let (result, bridged) = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || {
                        BRIDGED_LOCAL_BODIES.with(|count| count.set(0));
                        let result = crate::compile(&source, typescript);
                        let bridged = BRIDGED_LOCAL_BODIES.with(std::cell::Cell::get);
                        (result, bridged)
                    })
                    .expect("2 MiB source/evaluator bridge thread")
                    .join()
                    .expect("block-bodied source/evaluator lowering must not overflow");
                assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
                if admitted {
                    assert!(bridged > 0, "{body}: no same-cursor block-body handoff");
                }
            }
        }
    }

    #[test]
    fn alternating_lexical_blocks_and_control_nodes_preserve_source_guards() {
        // Every nested block has a source declaration and a following control
        // node: a singleton-block peel alone cannot avoid this reentry pattern.
        for depth in [4, 12, 24, 40] {
            let mut body = String::from("a+=b;");
            for index in 0..depth {
                body = match index % 3 {
                    0 => format!("if(a){{let v_{index}=b;{body}}}else{{b+=a;}}"),
                    1 => format!("while(a){{let v_{index}=b;{body}}}"),
                    _ => format!("do{{let v_{index}=b;{body}}}while(false);"),
                };
            }
            let source = format!("export function run(a,b){{{body}return a;}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, ts))
                    .expect("small stack control/block thread")
                    .join()
                    .expect("mixed lexical controls must not overflow Rust stack");
                if depth <= 12 {
                    assert!(result.is_ok(), "{depth} levels: {:?}", result.err());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64") || error.contains("128") || error.contains("256"),
                        "{depth} mixed lexical controls: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn alternating_if_and_loop_families_keep_native_stack_bounded() {
        // Homogeneous nesting was already iterative. Alternating kinds used to
        // reenter different native-stack cursors for each source layer.
        for depth in [4, 12, 24, 40, 66] {
            let mut body = String::from("a+=b;");
            for index in 0..depth {
                body = match index % 3 {
                    0 => format!("if(a){{{body}}}else{{b+=a;}}"),
                    1 => format!("while(a){{{body}}}"),
                    _ => format!("do{{{body}}}while(false);"),
                };
            }
            let source = format!("export function run(a,b){{{body}return a;}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, ts))
                    .expect("2 MiB mixed-control compiler thread")
                    .join()
                    .expect("mixed statement cursor must not exhaust native stack");
                if depth <= 12 {
                    assert!(result.is_ok(), "{depth} mixed controls: {:?}", result.err());
                }
                if depth == 66 {
                    let error = result.err().expect("over-depth mixed source must reject");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} mixed controls: {error}"
                    );
                }
            }
        }
    }
}
