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
    For(ForFrame<'n, 'a>),
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
            Self::IfPredicate { depth, .. }
            | Self::If { depth, .. }
            | Self::While { depth, .. }
            | Self::DoWhile { depth, .. }
            | Self::Block { depth, .. }
            | Self::Sequence { depth, .. }
            | Self::TryCatchBody { depth, .. }
            | Self::TryCatchHandler { depth, .. }
            | Self::TryFinallyBody { depth, .. }
            | Self::TryFinallyCleanup { depth, .. } => *depth,
            Self::For(frame) => frame.depth,
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

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn control_statement_iterative<'n, 'a>(
        &mut self,
        root: ControlNode<'n, 'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        #[cfg(test)]
        let _entry = crate::source_cursor_tests::entry();
        // The root has already been charged by statement_inner. Every direct
        // nested control node is charged exactly once when its Enter runs.
        let saved_context = self.source_context();
        let original_depth = saved_context.statement_depth;
        let root_output = Self::take_cursor_region(region);
        let result = (|| {
            let mut cursor = Cursor::new(root, root_output, depth);

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
                            } else {
                                // The child may itself be a block, for, try or
                                // function body. Preserve its enclosing loop mode
                                // and source depth until the child has completed.
                                self.statement_depth =
                                    cursor.pending.last().map_or(original_depth, Frame::depth);
                                let completion = self.statement(source, &mut output, depth)?;
                                cursor.completed = Some(Outcome::Statement(output, completion));
                            }
                        }
                        Next::Expression {
                            source,
                            mut output,
                            depth,
                            mode,
                        } => {
                            // A single source-owned handoff point. Eventually
                            // the evaluator may itself yield a child-body task;
                            // for now it uses the original typed evaluator.
                            let value = self.evaluate(source, &mut output, depth, mode)?;
                            cursor.completed = Some(Outcome::Expression(output, value));
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
        self.restore_source_context(saved_context);
        result
    }
}

#[cfg(test)]
mod tests {
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
