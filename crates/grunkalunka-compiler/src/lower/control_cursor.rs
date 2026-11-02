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

mod steps;

#[cfg(test)]
std::thread_local! {
    static BRIDGED_LOCAL_BODIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

type Next<'n, 'a> = source_control::StatementTask<'n, 'a, ControlNode<'n, 'a>>;

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

enum Frame<'n, 'a> {
    /// A charged expression statement awaits a Discard-mode completion.
    DiscardExpression {
        depth: usize,
    },
    /// Directly evaluated return expressions resume with their typed result.
    ReturnValue {
        depth: usize,
    },
    /// Async source throw produces the original General/AsyncValue reason.
    ThrowValue {
        depth: usize,
    },
    /// One source-charged numeric let binding awaits its initializer ValueId.
    /// The declaration slot and visible binding are published only after its
    /// initializer side effects finish, as on the original statement path.
    NumericLet {
        local: &'n VariableDeclarator<'a>,
        depth: usize,
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
    /// Declaration initializers retain their existing declaration helper.
    ForInitializer {
        source: &'n ForStatement<'a>,
        output: Region,
        depth: usize,
        outer: usize,
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
            Self::DiscardExpression { depth }
            | Self::ReturnValue { depth }
            | Self::ThrowValue { depth }
            | Self::NumericLet { depth, .. }
            | Self::IfPredicate { depth, .. }
            | Self::If { depth, .. }
            | Self::WhilePredicate { depth, .. }
            | Self::While { depth, .. }
            | Self::DoWhile { depth, .. }
            | Self::DoWhilePredicate { depth, .. }
            | Self::Block { depth, .. }
            | Self::Sequence { depth, .. }
            | Self::EvaluateBody { depth, .. }
            | Self::TryCatchBody { depth, .. }
            | Self::TryCatchHandler { depth, .. }
            | Self::TryFinallyBody { depth, .. }
            | Self::TryFinallyCleanup { depth, .. } => *depth,
            Self::ForInitializer { depth, .. } => *depth,
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
}

/// One active source-lowering machine. Language-specific transitions own
/// their variant-specific Frame data but never allocate another scheduler.
struct Cursor<'n, 'a> {
    pending: Vec<Frame<'n, 'a>>,
    next: Option<Next<'n, 'a>>,
    completed: Option<Outcome>,
}

impl<'n, 'a> Cursor<'n, 'a> {
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
            Type::General if !self.allow_await => evaluate::Mode::General,
            Type::AsyncValue => evaluate::Mode::AsyncValue,
            Type::Promise => evaluate::Mode::Pending,
            Type::SettledPromise => evaluate::Mode::Settled,
            Type::FulfilledPromise => evaluate::Mode::Promise,
            Type::ManagedValue => evaluate::Mode::Managed,
            Type::ManagedObject => evaluate::Mode::Object,
            Type::ManagedArray => evaluate::Mode::Array,
            Type::General | Type::Closure(_) => return None,
        })
    }

    /// Conservative read-only selection of the exact existing Number path
    /// for a single initialized `let`. All function-valued, mixed Promise,
    /// managed and General declarations retain their legacy lowering.
    fn numeric_let_initializer<'n>(
        &self,
        declaration: &'n VariableDeclaration<'ast>,
    ) -> Option<&'n VariableDeclarator<'ast>> {
        if declaration.kind != VariableDeclarationKind::Let
            || declaration.declare
            || declaration.declarations.len() != 1
        {
            return None;
        }
        let local = declaration.declarations.first()?;
        if local.definite {
            return None;
        }
        let BindingPattern::BindingIdentifier(identifier) = &local.id else {
            return None;
        };
        let symbol = identifier.symbol_id.get()?;
        let initializer = local.init.as_ref()?;
        if self.promise_parameters.is_mixed(&symbol)
            || self.function_arity(initializer, 0).is_some()
            || self.pending_promise_shape(initializer, 0)
            || self.fulfilled_promise_shape(initializer, 0)
            || self.managed_source_type(initializer, 0).is_some()
            || self.general_bindings.contains(&symbol)
            || !self.numeric_result_shape(initializer, 0)
        {
            return None;
        }
        Some(local)
    }

    /// Drive one owned expression activation until it yields a source-body
    /// request or completes. The source scheduler owns any yielded activation
    /// and continues without recursively entering another source cursor.
    #[inline(never)]
    fn advance_owned_expression<'n>(
        &mut self,
        mut expression: evaluate::EvaluationCursor<'n, 'ast>,
        mode: evaluate::Mode,
        cursor: &mut Cursor<'n, 'ast>,
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
                evaluate::EvaluationRun::LowerFunctionLeaf {
                    node,
                    signature,
                    depth,
                    region_depth,
                } => {
                    self.resume_lowered_function_leaf(
                        &mut expression,
                        node,
                        signature,
                        depth,
                        region_depth,
                    )?;
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
                                && let Some(local) = self.numeric_let_initializer(declaration)
                            {
                                // Match statement_inner -> declaration's admission
                                // and first-local charge before evaluating any source.
                                self.statement_depth = depth;
                                self.admit_source_statement(source, depth)?;
                                if self.source_charging && self.local_count >= 128 {
                                    return Err(unsupported(
                                        local,
                                        "at most 128 source locals are admitted",
                                    ));
                                }
                                if self.source_charging {
                                    self.local_count += 1;
                                }
                                cursor.pending.push(Frame::NumericLet { local, depth });
                                cursor.next = Some(Next::Expression {
                                    source: local.init.as_ref().expect("eligible initializer"),
                                    output,
                                    depth: 0,
                                    mode: evaluate::Mode::Number,
                                });
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
                if let Frame::EvaluateBody { activation, .. } = frame {
                    self.abort_arrow_block_activation(activation);
                }
            }
        }
        self.restore_source_context(saved_context);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
