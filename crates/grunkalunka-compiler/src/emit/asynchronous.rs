//! Split verified regions into a switched-resume coroutine. The frame currently
//! conservatively keeps source locals, captured-cell pointers and snapshots in
//! instance globals; captured cell contents live in the invocation's guest arena.
use super::{Emitter, NUMBER_REMAINDER, cells, closures, emit_function, layout};
use crate::ir::{self, Exit, Op, Region, Type, ValueId};
use std::fmt::Write;

#[cfg(test)]
mod block_split_tests;
#[cfg(test)]
mod branch_split_tests;
#[cfg(test)]
mod call_split_tests;
mod dispatch;
mod errors;
#[cfg(test)]
mod finally_split_tests;
mod frames;
mod graph;
#[cfg(test)]
mod loop_split_tests;
mod managed_globals;
#[cfg(test)]
mod mixed_split_tests;
mod promises;
mod starters;
#[cfg(test)]
mod trycatch_split_tests;

const MAX_STATES: usize = 4096;

#[derive(Clone, Copy)]
struct LoopTargets {
    exit: usize,
    next: usize,
}

#[derive(Clone, Copy)]
enum YieldTarget {
    Copy { result: ValueId, next: usize },
    Test { yes: usize, no: usize },
}

enum End<'a> {
    AdoptPromise {
        subscription: usize,
    },
    AdoptSubscription {
        input: ValueId,
        reaction: usize,
    },
    AwaitStarted {
        reaction: usize,
    },
    StartedClosure {
        result: ValueId,
        next: usize,
    },
    AwaitPromise {
        input: ValueId,
        reaction: usize,
    },
    Reaction {
        fulfilled: usize,
        rejected: usize,
    },
    InvokeClosure {
        callee: ValueId,
        arguments: &'a [ValueId],
        returned: usize,
    },
    EnterClosure {
        next: usize,
    },
    ReturnedClosure {
        result: ValueId,
        fulfilled: usize,
        rejected: usize,
    },
    Jump(usize),
    Copy {
        input: ValueId,
        result: ValueId,
        next: usize,
    },
    Test {
        condition: ValueId,
        yes: usize,
        no: usize,
    },
    SettledTest {
        input: ValueId,
        yes: usize,
        no: usize,
    },
    Suspend {
        input: ValueId,
        rejected: bool,
        resume: usize,
    },
    Return(ValueId),
    Reject(ValueId),
    Trap,
}

struct State<'a> {
    function: usize,
    operations: &'a [ir::Operation],
    resumed: Option<ValueId>,
    end: End<'a>,
}

#[derive(Default)]
struct Splitter<'a> {
    queued: bool,
    function: usize,
    states: Vec<State<'a>>,
    awaits: usize,
    shared_awaits: std::collections::BTreeSet<(usize, usize)>,
    expanded_awaits: std::collections::BTreeSet<(usize, usize)>,
    return_to: Option<(ValueId, usize)>,
    catch_to: Option<(ValueId, usize)>,
}

// The explicit splitter stack suspends a parent while one child Region is
// built. Each If first builds the consequent, then the alternate, preserving
// the previous state IDs, continuation targets and branch evaluation order.
enum PendingSplit<'a> {
    Block {
        parent: &'a Region,
        remaining: usize,
    },
    IfConsequent {
        parent: &'a Region,
        remaining: usize,
        alternate: &'a Region,
        condition: ValueId,
        normal: usize,
        yielded: Option<YieldTarget>,
    },
    IfAlternate {
        parent: &'a Region,
        remaining: usize,
        condition: ValueId,
        yes: usize,
    },
    TryHandler {
        parent: &'a Region,
        remaining: usize,
        body: &'a Region,
        reason: ValueId,
        normal: usize,
    },
    TryBody {
        parent: &'a Region,
        remaining: usize,
        outer_catch: Option<(ValueId, usize)>,
    },
    InlineBody {
        parent: &'a Region,
        remaining: usize,
        outer_return: Option<(ValueId, usize)>,
        outer_loops: Option<LoopTargets>,
    },
    AwaitBody {
        parent: &'a Region,
        remaining: usize,
        outer_return: Option<(ValueId, usize)>,
        outer_catch: Option<(ValueId, usize)>,
        outer_loops: Option<LoopTargets>,
    },
    WhileStep {
        parent: &'a Region,
        remaining: usize,
        body: &'a Region,
        condition: &'a Region,
        test: usize,
        normal: usize,
        outer_loops: Option<LoopTargets>,
    },
    WhileBody {
        parent: &'a Region,
        remaining: usize,
        condition: &'a Region,
        test: usize,
        normal: usize,
        outer_loops: Option<LoopTargets>,
    },
    WhileCondition {
        parent: &'a Region,
        remaining: usize,
        test: usize,
        outer_loops: Option<LoopTargets>,
    },
    DoCondition {
        parent: &'a Region,
        remaining: usize,
        body: &'a Region,
        first: usize,
        normal: usize,
        outer_loops: Option<LoopTargets>,
    },
    DoBody {
        parent: &'a Region,
        remaining: usize,
        first: usize,
        outer_loops: Option<LoopTargets>,
    },
    FinallyContinuation(FinallyFrame<'a>),
    FinallyReturn(FinallyFrame<'a>),
    FinallyThrow {
        frame: FinallyFrame<'a>,
        on_return: usize,
    },
    FinallyNormal {
        frame: FinallyFrame<'a>,
        on_return: usize,
        on_throw: usize,
    },
    FinallyLoopExit {
        frame: FinallyFrame<'a>,
        on_return: usize,
        on_throw: usize,
        normal: usize,
    },
    FinallyLoopNext {
        frame: FinallyFrame<'a>,
        on_return: usize,
        on_throw: usize,
        normal: usize,
        exit: usize,
    },
    FinallyBody(FinallyFrame<'a>),
}

// All finalizer variants reuse one original Region with distinct continuations.
#[derive(Clone, Copy)]
struct FinallyFrame<'a> {
    parent: &'a Region,
    remaining: usize,
    body: &'a Region,
    finalizer: &'a Region,
    returned: ValueId,
    reason: ValueId,
    normal: usize,
    outer_return: Option<(ValueId, usize)>,
    outer_catch: Option<(ValueId, usize)>,
    outer_loops: Option<LoopTargets>,
}

impl<'a> Splitter<'a> {
    fn charge_await(&mut self, operations: &[ir::Operation], index: usize) -> Result<(), String> {
        // A finalizer's one IR await can be emitted under several pending exits.
        // Count that source operation once, while every executed reaction still runs.
        if !self
            .expanded_awaits
            .insert((self.function, &operations[index] as *const _ as usize))
        {
            return Ok(());
        }
        let shared = index
            .checked_sub(1)
            .and_then(|previous| match operations[previous].kind {
                Op::AwaitSite { site } => Some((self.function, site)),
                _ => None,
            });
        if shared.is_none_or(|site| self.shared_awaits.insert(site)) {
            self.awaits += 1;
        }
        if self.awaits > 32 {
            return Err("async profile admits at most 32 await sites".into());
        }
        Ok(())
    }

    fn state(
        &mut self,
        operations: &'a [ir::Operation],
        resumed: Option<ValueId>,
        end: End<'a>,
    ) -> Result<usize, String> {
        if self.states.len() == MAX_STATES {
            return Err("async continuation graph exceeds 4096 states".into());
        }
        let id = self.states.len();
        self.states.push(State {
            function: self.function,
            operations,
            resumed,
            end,
        });
        Ok(id)
    }

    fn terminal(&mut self, end: End<'a>) -> Result<usize, String> {
        self.state(&[], None, end)
    }

    fn rejection_resume(&mut self, result: ValueId, next: usize) -> Result<(bool, usize), String> {
        let caught = self.catch_to;
        let (resumed, target) = caught.unwrap_or((result, next));
        // A private rejection settles this child after its reaction. It must
        // return to its caller, whose await owns a separate reaction boundary.
        let end = if caught.is_none() && (self.function != 0 || self.queued) {
            End::Reject(resumed)
        } else {
            End::Jump(target)
        };
        let resume = self.state(&[], Some(resumed), end)?;
        Ok((caught.is_none() && self.function == 0, resume))
    }

    // Invocation arguments borrow the original verified IR ValueId slice. It
    // remains valid through splitting, including repeated finally continuations,
    // and avoids allocating a new vector for each emitted call.
    // Construct an entry for a Region's exit without recursing into children.
    // Block continuations share this rule with ordinary recursive calls.
    fn exit_entry(
        &mut self,
        region: &Region,
        normal: usize,
        yielded: Option<YieldTarget>,
        loops: Option<LoopTargets>,
    ) -> Result<usize, String> {
        let next = match region.exit {
            Exit::Fallthrough => normal,
            Exit::Return(input) => self.terminal(match self.return_to {
                Some((result, next)) => End::Copy {
                    input,
                    result,
                    next,
                },
                None => End::Return(input),
            })?,
            Exit::Throw(input) => self.terminal(match self.catch_to {
                Some((result, next)) => End::Copy {
                    input,
                    result,
                    next,
                },
                None => End::Reject(input),
            })?,
            Exit::Break => loops.ok_or("async break lacks a loop target")?.exit,
            Exit::Continue => loops.ok_or("async continue lacks a loop target")?.next,
            Exit::Yield(input) => {
                self.terminal(match yielded.ok_or("async yield lacks a target")? {
                    YieldTarget::Copy { result, next } => End::Copy {
                        input,
                        result,
                        next,
                    },
                    YieldTarget::Test { yes, no } => End::Test {
                        condition: input,
                        yes,
                        no,
                    },
                })?
            }
        };
        Ok(next)
    }

    // Build backwards from the suffix. Every internal edge stays in the current
    // host job; only Suspend returns to the scheduler. Loop targets are lexical.
    fn region(
        &mut self,
        region: &'a Region,
        normal: usize,
        yielded: Option<YieldTarget>,
        loops: Option<LoopTargets>,
    ) -> Result<usize, String> {
        // Nested try/catch changes the lexical exception target. An early
        // graph-budget failure must not leak that dynamic binding to the next
        // attempted split on this same builder.
        let outer_return = self.return_to;
        let outer_catch = self.catch_to;
        let result = self.region_inner(region, normal, yielded, loops);
        self.return_to = outer_return;
        self.catch_to = outer_catch;
        result
    }

    fn region_inner(
        &mut self,
        region: &'a Region,
        normal: usize,
        yielded: Option<YieldTarget>,
        loops: Option<LoopTargets>,
    ) -> Result<usize, String> {
        // A fallthrough Region containing only a Block is a transparent
        // continuation edge. Follow these wrappers iteratively before
        // descending into the ordinary splitter, which otherwise recurses
        // once for every nested Block even though no state is created.
        let mut region = region;
        let mut yielded = yielded;
        while matches!(region.exit, Exit::Fallthrough) && region.operations.len() == 1 {
            let Op::Block(child) = &region.operations[0].kind else {
                break;
            };
            region = child;
            // Op::Block passes no yield target to its nested statement.
            yielded = None;
        }

        let mut active_loops = loops;
        let mut next = self.exit_entry(region, normal, yielded, active_loops)?;
        // Block and If child continuations now keep their parent region,
        // remaining prefix and branch targets in an explicit heap-owned stack,
        // independent of arbitrary source-generated nesting depth.
        let mut pending: Vec<PendingSplit<'a>> = Vec::new();
        let mut end = region.operations.len();
        loop {
            if end == 0 {
                match pending.pop() {
                    Some(PendingSplit::Block { parent, remaining }) => {
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::IfConsequent {
                        parent,
                        remaining,
                        alternate,
                        condition,
                        normal,
                        yielded,
                    }) => {
                        pending.push(PendingSplit::IfAlternate {
                            parent,
                            remaining,
                            condition,
                            yes: next,
                        });
                        next = self.exit_entry(alternate, normal, yielded, active_loops)?;
                        region = alternate;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::IfAlternate {
                        parent,
                        remaining,
                        condition,
                        yes,
                    }) => {
                        next = self.terminal(End::Test {
                            condition,
                            yes,
                            no: next,
                        })?;
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::TryHandler {
                        parent,
                        remaining,
                        body,
                        reason,
                        normal,
                    }) => {
                        let outer_catch = self.catch_to.replace((reason, next));
                        pending.push(PendingSplit::TryBody {
                            parent,
                            remaining,
                            outer_catch,
                        });
                        next = self.exit_entry(body, normal, None, active_loops)?;
                        region = body;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::TryBody {
                        parent,
                        remaining,
                        outer_catch,
                    }) => {
                        self.catch_to = outer_catch;
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::InlineBody {
                        parent,
                        remaining,
                        outer_return,
                        outer_loops,
                    }) => {
                        self.return_to = outer_return;
                        active_loops = outer_loops;
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::AwaitBody {
                        parent,
                        remaining,
                        outer_return,
                        outer_catch,
                        outer_loops,
                    }) => {
                        self.return_to = outer_return;
                        self.catch_to = outer_catch;
                        active_loops = outer_loops;
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::WhileStep {
                        parent,
                        remaining,
                        body,
                        condition,
                        test,
                        normal,
                        outer_loops,
                    }) => {
                        // Step is the nearest-loop continue target. Build the
                        // body under that target, keeping the outer loop only
                        // for the separate condition and step regions.
                        let step = next;
                        pending.push(PendingSplit::WhileBody {
                            parent,
                            remaining,
                            condition,
                            test,
                            normal,
                            outer_loops,
                        });
                        active_loops = Some(LoopTargets {
                            exit: normal,
                            next: step,
                        });
                        next = self.exit_entry(body, step, None, active_loops)?;
                        region = body;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::WhileBody {
                        parent,
                        remaining,
                        condition,
                        test,
                        normal,
                        outer_loops,
                    }) => {
                        let body = next;
                        pending.push(PendingSplit::WhileCondition {
                            parent,
                            remaining,
                            test,
                            outer_loops,
                        });
                        active_loops = outer_loops;
                        next = self.exit_entry(
                            condition,
                            normal,
                            Some(YieldTarget::Test {
                                yes: body,
                                no: normal,
                            }),
                            active_loops,
                        )?;
                        region = condition;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::WhileCondition {
                        parent,
                        remaining,
                        test,
                        outer_loops,
                    }) => {
                        self.states[test].end = End::Jump(next);
                        next = test;
                        active_loops = outer_loops;
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::DoCondition {
                        parent,
                        remaining,
                        body,
                        first,
                        normal,
                        outer_loops,
                    }) => {
                        let condition = next;
                        pending.push(PendingSplit::DoBody {
                            parent,
                            remaining,
                            first,
                            outer_loops,
                        });
                        active_loops = Some(LoopTargets {
                            exit: normal,
                            next: condition,
                        });
                        next = self.exit_entry(body, condition, None, active_loops)?;
                        region = body;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::DoBody {
                        parent,
                        remaining,
                        first,
                        outer_loops,
                    }) => {
                        self.states[first].end = End::Jump(next);
                        next = first;
                        active_loops = outer_loops;
                        region = parent;
                        end = remaining;
                    }
                    Some(PendingSplit::FinallyContinuation(frame)) => {
                        let resume_return = next;
                        self.return_to = Some((frame.returned, resume_return));
                        pending.push(PendingSplit::FinallyReturn(frame));
                        active_loops = frame.outer_loops;
                        next =
                            self.exit_entry(frame.finalizer, resume_return, None, active_loops)?;
                        region = frame.finalizer;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::FinallyReturn(frame)) => {
                        let on_return = next;
                        let rethrow = self.terminal(match frame.outer_catch {
                            Some((result, target)) => End::Copy {
                                input: frame.reason,
                                result,
                                next: target,
                            },
                            None => End::Reject(frame.reason),
                        })?;
                        pending.push(PendingSplit::FinallyThrow { frame, on_return });
                        active_loops = frame.outer_loops;
                        next = self.exit_entry(frame.finalizer, rethrow, None, active_loops)?;
                        region = frame.finalizer;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::FinallyThrow { frame, on_return }) => {
                        let on_throw = next;
                        pending.push(PendingSplit::FinallyNormal {
                            frame,
                            on_return,
                            on_throw,
                        });
                        active_loops = frame.outer_loops;
                        next =
                            self.exit_entry(frame.finalizer, frame.normal, None, active_loops)?;
                        region = frame.finalizer;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::FinallyNormal {
                        frame,
                        on_return,
                        on_throw,
                    }) => {
                        let normal = next;
                        if let Some(targets) = frame.outer_loops {
                            pending.push(PendingSplit::FinallyLoopExit {
                                frame,
                                on_return,
                                on_throw,
                                normal,
                            });
                            active_loops = frame.outer_loops;
                            next =
                                self.exit_entry(frame.finalizer, targets.exit, None, active_loops)?;
                            region = frame.finalizer;
                            end = region.operations.len();
                        } else {
                            self.return_to = Some((frame.returned, on_return));
                            self.catch_to = Some((frame.reason, on_throw));
                            pending.push(PendingSplit::FinallyBody(frame));
                            active_loops = None;
                            next = self.exit_entry(frame.body, normal, None, active_loops)?;
                            region = frame.body;
                            end = region.operations.len();
                        }
                    }
                    Some(PendingSplit::FinallyLoopExit {
                        frame,
                        on_return,
                        on_throw,
                        normal,
                    }) => {
                        let exit = next;
                        let targets = frame.outer_loops.expect("outer loop exists");
                        pending.push(PendingSplit::FinallyLoopNext {
                            frame,
                            on_return,
                            on_throw,
                            normal,
                            exit,
                        });
                        active_loops = frame.outer_loops;
                        next =
                            self.exit_entry(frame.finalizer, targets.next, None, active_loops)?;
                        region = frame.finalizer;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::FinallyLoopNext {
                        frame,
                        on_return,
                        on_throw,
                        normal,
                        exit,
                    }) => {
                        let next_loop = next;
                        self.return_to = Some((frame.returned, on_return));
                        self.catch_to = Some((frame.reason, on_throw));
                        pending.push(PendingSplit::FinallyBody(frame));
                        active_loops = Some(LoopTargets {
                            exit,
                            next: next_loop,
                        });
                        next = self.exit_entry(frame.body, normal, None, active_loops)?;
                        region = frame.body;
                        end = region.operations.len();
                    }
                    Some(PendingSplit::FinallyBody(frame)) => {
                        self.return_to = frame.outer_return;
                        self.catch_to = frame.outer_catch;
                        active_loops = frame.outer_loops;
                        region = frame.parent;
                        end = frame.remaining;
                    }
                    None => return Ok(next),
                }
                continue;
            }
            let operation = &region.operations[end - 1];
            if let Op::Block(child) = &operation.kind {
                pending.push(PendingSplit::Block {
                    parent: region,
                    remaining: end - 1,
                });
                next = self.exit_entry(child, next, None, active_loops)?;
                region = child;
                end = region.operations.len();
                continue;
            }
            if let Op::If {
                result,
                condition,
                consequent,
                alternate,
            } = &operation.kind
            {
                let yielded = result.map(|result| YieldTarget::Copy { result, next });
                pending.push(PendingSplit::IfConsequent {
                    parent: region,
                    remaining: end - 1,
                    alternate,
                    condition: *condition,
                    normal: next,
                    yielded,
                });
                next = self.exit_entry(consequent, next, yielded, active_loops)?;
                region = consequent;
                end = region.operations.len();
                continue;
            }
            if let Op::TryCatch {
                body,
                reason,
                handler,
            } = &operation.kind
            {
                // Build the handler under the outer lexical target. Only after
                // that child completes does the protected body inherit the
                // newly established exception continuation.
                pending.push(PendingSplit::TryHandler {
                    parent: region,
                    remaining: end - 1,
                    body,
                    reason: *reason,
                    normal: next,
                });
                next = self.exit_entry(handler, next, None, active_loops)?;
                region = handler;
                end = region.operations.len();
                continue;
            }
            if let Op::InlineCall { result, body } = &operation.kind {
                let outer_return = self.return_to.replace((*result, next));
                pending.push(PendingSplit::InlineBody {
                    parent: region,
                    remaining: end - 1,
                    outer_return,
                    outer_loops: active_loops,
                });
                active_loops = None;
                next = self.exit_entry(body, next, None, active_loops)?;
                region = body;
                end = region.operations.len();
                continue;
            }
            if let Op::AwaitCall { result, body } = &operation.kind {
                // Capture the caller's two settlement continuations before
                // entering the callee, exactly as with the recursive splitter.
                self.charge_await(&region.operations, end - 1)?;
                let fulfilled_resume = self.state(&[], Some(*result), End::Jump(next))?;
                let fulfilled = self.terminal(End::Suspend {
                    input: *result,
                    rejected: false,
                    resume: fulfilled_resume,
                })?;
                let (uncaught, rejected_resume) = self.rejection_resume(*result, next)?;
                let rejected = self.terminal(End::Suspend {
                    input: *result,
                    rejected: uncaught,
                    resume: rejected_resume,
                })?;
                let outer_return = self.return_to.replace((*result, fulfilled));
                let outer_catch = self.catch_to.replace((*result, rejected));
                pending.push(PendingSplit::AwaitBody {
                    parent: region,
                    remaining: end - 1,
                    outer_return,
                    outer_catch,
                    outer_loops: active_loops,
                });
                active_loops = None;
                next = self.exit_entry(body, fulfilled, None, active_loops)?;
                region = body;
                end = region.operations.len();
                continue;
            }
            if let Op::While {
                condition,
                body,
                step,
            } = &operation.kind
            {
                let test = self.terminal(End::Trap)?;
                if let Some(step) = step {
                    pending.push(PendingSplit::WhileStep {
                        parent: region,
                        remaining: end - 1,
                        body,
                        condition,
                        test,
                        normal: next,
                        outer_loops: active_loops,
                    });
                    next = self.exit_entry(step, test, None, active_loops)?;
                    region = step;
                } else {
                    pending.push(PendingSplit::WhileBody {
                        parent: region,
                        remaining: end - 1,
                        condition,
                        test,
                        normal: next,
                        outer_loops: active_loops,
                    });
                    active_loops = Some(LoopTargets {
                        exit: next,
                        next: test,
                    });
                    next = self.exit_entry(body, test, None, active_loops)?;
                    region = body;
                }
                end = region.operations.len();
                continue;
            }
            if let Op::DoWhile { body, condition } = &operation.kind {
                let first = self.terminal(End::Trap)?;
                pending.push(PendingSplit::DoCondition {
                    parent: region,
                    remaining: end - 1,
                    body,
                    first,
                    normal: next,
                    outer_loops: active_loops,
                });
                next = self.exit_entry(
                    condition,
                    next,
                    Some(YieldTarget::Test {
                        yes: first,
                        no: next,
                    }),
                    active_loops,
                )?;
                region = condition;
                end = region.operations.len();
                continue;
            }
            if let Op::TryFinally {
                body,
                finalizer,
                returned,
                reason,
                continuation,
            } = &operation.kind
            {
                let frame = FinallyFrame {
                    parent: region,
                    remaining: end - 1,
                    body,
                    finalizer,
                    returned: *returned,
                    reason: *reason,
                    normal: next,
                    outer_return: self.return_to,
                    outer_catch: self.catch_to,
                    outer_loops: active_loops,
                };
                pending.push(PendingSplit::FinallyContinuation(frame));
                next = self.exit_entry(continuation, next, None, active_loops)?;
                region = continuation;
                end = region.operations.len();
                continue;
            }
            next = match &operation.kind {
                Op::AwaitSite { .. } => next,
                Op::StartAsyncClosure {
                    result,
                    callee,
                    arguments,
                } => {
                    let returned = self.terminal(End::StartedClosure {
                        result: *result,
                        next,
                    })?;
                    self.terminal(End::InvokeClosure {
                        callee: *callee,
                        arguments,
                        returned,
                    })?
                }
                Op::AwaitPromise { result, input } => {
                    self.charge_await(&region.operations, end - 1)?;
                    let fulfilled = self.state(&[], Some(*result), End::Jump(next))?;
                    let (_, rejected) = self.rejection_resume(*result, next)?;
                    let reaction = self.terminal(End::Reaction {
                        fulfilled,
                        rejected,
                    })?;
                    self.terminal(End::AwaitPromise {
                        input: *input,
                        reaction,
                    })?
                }
                Op::AdoptPromise { result, input } => {
                    let fulfilled = self.state(&[], Some(*result), End::Jump(next))?;
                    let rejected = self.state(&[], Some(*result), End::Reject(*result))?;
                    let reaction = self.terminal(End::Reaction {
                        fulfilled,
                        rejected,
                    })?;
                    let subscription = self.terminal(End::AdoptSubscription {
                        input: *input,
                        reaction,
                    })?;
                    self.terminal(End::AdoptPromise { subscription })?
                }
                Op::AwaitClosureCall {
                    result,
                    callee,
                    arguments,
                } => {
                    self.charge_await(&region.operations, end - 1)?;
                    let fulfilled_resume = self.state(&[], Some(*result), End::Jump(next))?;
                    let returned = if self.queued {
                        let (_, rejected) = self.rejection_resume(*result, next)?;
                        let reaction = self.terminal(End::Reaction {
                            fulfilled: fulfilled_resume,
                            rejected,
                        })?;
                        self.terminal(End::AwaitStarted { reaction })?
                    } else {
                        let fulfilled = self.terminal(End::Suspend {
                            input: *result,
                            rejected: false,
                            resume: fulfilled_resume,
                        })?;
                        let (uncaught, rejected_resume) = self.rejection_resume(*result, next)?;
                        let rejected = self.terminal(End::Suspend {
                            input: *result,
                            rejected: uncaught,
                            resume: rejected_resume,
                        })?;
                        self.terminal(End::ReturnedClosure {
                            result: *result,
                            fulfilled,
                            rejected,
                        })?
                    };
                    self.terminal(End::InvokeClosure {
                        callee: *callee,
                        arguments,
                        returned,
                    })?
                }
                Op::InlineCall { .. } | Op::AwaitCall { .. } => {
                    unreachable!("call bodies use the explicit continuation stack")
                }
                Op::TryCatch { .. } => unreachable!("catch continuations use the explicit stack"),
                Op::TryFinally { .. } => {
                    unreachable!("finally continuations use the explicit stack")
                }
                Op::AwaitSettled { result, input } => {
                    self.charge_await(&region.operations, end - 1)?;
                    let normal_resume = self.state(&[], Some(*result), End::Jump(next))?;
                    let normal = self.terminal(End::Suspend {
                        input: *input,
                        rejected: false,
                        resume: normal_resume,
                    })?;
                    let (uncaught, rejected_resume) = self.rejection_resume(*result, next)?;
                    let rejected = self.terminal(End::Suspend {
                        input: *input,
                        rejected: uncaught,
                        resume: rejected_resume,
                    })?;
                    self.terminal(End::SettledTest {
                        input: *input,
                        yes: rejected,
                        no: normal,
                    })?
                }
                Op::Await {
                    result,
                    input,
                    rejected,
                } => {
                    self.charge_await(&region.operations, end - 1)?;
                    // A caught rejection resumes into its lexical handler on the
                    // same reaction job. The host still supplies the original pair.
                    let (uncaught, resume) = if *rejected {
                        self.rejection_resume(*result, next)?
                    } else {
                        (false, self.state(&[], Some(*result), End::Jump(next))?)
                    };
                    self.terminal(End::Suspend {
                        input: *input,
                        rejected: uncaught,
                        resume,
                    })?
                }
                Op::Block(_) => unreachable!("block is scheduled on the explicit stack"),
                Op::If { .. } => unreachable!("If continuations use the explicit stack"),
                Op::While { .. } | Op::DoWhile { .. } => {
                    unreachable!("loop continuations use the explicit stack")
                }
                _ => {
                    let mut start = end - 1;
                    while start > 0 && atomic(&region.operations[start - 1].kind) {
                        start -= 1;
                    }
                    next = self.state(&region.operations[start..end], None, End::Jump(next))?;
                    end = start;
                    continue;
                }
            };
            end -= 1;
        }
    }
}

fn atomic(op: &Op) -> bool {
    !matches!(
        op,
        Op::AwaitSite { .. }
            | Op::Await { .. }
            | Op::AwaitPromise { .. }
            | Op::AdoptPromise { .. }
            | Op::StartAsyncClosure { .. }
            | Op::AwaitClosureCall { .. }
            | Op::AwaitSettled { .. }
            | Op::AwaitCall { .. }
            | Op::TryCatch { .. }
            | Op::TryFinally { .. }
            | Op::InlineCall { .. }
            | Op::Block(_)
            | Op::If { .. }
            | Op::While { .. }
            | Op::DoWhile { .. }
    )
}

fn jump(output: &mut String, target: usize) {
    writeln!(output, "i32.const {target}\nglobal.set $pc\nbr $dispatch").unwrap();
}

pub(crate) fn emit_async(module: &ir::Module) -> Result<String, String> {
    ir::verify_async_module(module)?;
    let managed = super::module_has_managed(module);
    if super::gc_closures::needed(module) {
        return Err("managed values require reference-aware async frames".into());
    }
    let starters = starters::needed(module);
    let queued = module.functions.iter().any(|function| {
        let mut pending = false;
        ir::visit_operations(&function.body, &mut |op| {
            pending |= matches!(
                op,
                Op::StartAsyncClosure { .. }
                    | Op::AwaitPromise { .. }
                    | Op::AdoptPromise { .. }
                    | Op::MaterializePromise { .. }
            )
        });
        pending
    });
    let errors = module.functions.iter().any(|function| {
        let mut found = false;
        ir::visit_operations(&function.body, &mut |op| {
            found |= matches!(op, Op::AdoptPromise { .. })
        });
        found
    });
    let Some((function, outlined)) = module.functions.split_first() else {
        return Err("async profile currently requires a single entry function".into());
    };
    for (index, function) in module.functions.iter().enumerate() {
        let mut source_call = false;
        ir::visit_operations(&function.body, &mut |op| {
            source_call |= matches!(op, Op::Call { function, .. } if function.0 == 0);
        });
        // The source entry's logical ID does not denote a callable numeric Wasm
        // function in this module; its physical implementation is $advance.
        if source_call {
            return Err("async profile forbids source function calls to the async entry".into());
        }
        if index != 0
            && !ir::is_async_closure(function)
            && super::region_has_control(&function.body, false)
        {
            return Err("async closure bodies must be synchronous".into());
        }
    }
    let has_closures = outlined
        .iter()
        .any(|function| ir::closure_environment(function).is_some());
    let mut split = Splitter {
        queued,
        ..Splitter::default()
    };
    let fresh = split.terminal(End::Trap)?;
    let unreachable = split.terminal(End::Trap)?;
    let entry = split.region(&function.body, unreachable, None, None)?;
    split.states[fresh].end = End::Jump(entry);
    let mut entries = vec![None; module.functions.len()];
    for (index, function) in module.functions.iter().enumerate().skip(1) {
        if !ir::is_async_closure(function) {
            continue;
        }
        split.function = index;
        let unreachable = split.terminal(End::Trap)?;
        let body = split.region(&function.body, unreachable, None, None)?;
        entries[index] = Some(split.terminal(End::EnterClosure { next: body })?);
    }
    graph::verify(&split, &entries)?;
    graph::thread_empty_jumps(&mut split, fresh);
    let has_async = entries.iter().any(Option::is_some);
    if managed && has_async {
        return Err("managed values require reference-aware async frames".into());
    }
    let dispatch_targets = dispatch::targets(module);
    let mut types = Vec::new();
    let mut frames = Vec::new();
    for (index, function) in module.functions.iter().enumerate() {
        if index != 0 && !ir::is_async_closure(function) {
            frames.push(None);
            continue;
        }
        let mut frame = frames::Frame::new(function, types.len(), queued, errors, has_async);
        if starters {
            let mut spill = String::new();
            let mut restore = String::new();
            frame.spill(&mut spill);
            frame.restore(&mut restore);
            frame.emitter.call_boundary = Some(Box::new(super::CallBoundary { spill, restore }));
        }
        types.extend(frame.lanes.iter().copied());
        frames.push(Some(frame));
    }
    let has_cells = queued
        || has_closures
        || module
            .functions
            .iter()
            .any(|function| !ir::captured_cells(&function.body).is_empty());
    let mut output = String::from("(module\n");
    let roots = managed_globals::Globals::new(&types);
    roots.declare(&mut output, &types);
    if has_cells {
        output.push_str(cells::MEMORY);
    }
    if has_closures {
        output.push_str(&closures::table(module, true, starters, false));
    }
    if has_async || queued {
        output.push_str(frames::GLOBALS);
    }
    if queued {
        output.push_str(promises::GLOBALS);
    }
    output.push_str("(global $pc (mut i32) (i32.const 0))\n(func $advance (param $awaited_tag i32) (param $awaited f64) (result i32 i32 f64)\n");
    if frames
        .iter()
        .flatten()
        .any(|frame| super::needs_literal_scratch(frame.emitter.function))
    {
        // Construction is atomic within one state. The completed String/key
        // moves to a traced value global before any suspension can occur.
        output.push_str("(local $gc_literal (ref null $gc_key))\n");
    }
    output.push_str("block $invalid\nloop $dispatch\n");
    // A br_table dispatch avoids scanning every state on each loop iteration.
    for id in 0..split.states.len() {
        writeln!(output, "block $state{id}").unwrap();
    }
    output.push_str("global.get $pc\nbr_table");
    for id in 0..split.states.len() {
        write!(output, " $state{id}").unwrap();
    }
    output.push_str(" $invalid\n");
    for (id, state) in split.states.iter().enumerate().rev() {
        output.push_str("end\n");
        let frame = frames[state.function]
            .as_mut()
            .expect("coroutine function frame");
        if let End::EnterClosure { next } = state.end {
            frame.enter(&mut output);
            jump(&mut output, next);
            continue;
        }
        if queued && (state.resumed.is_some() || matches!(state.end, End::AdoptSubscription { .. }))
        {
            frame.restore(&mut output);
        }
        let emitter = &mut frame.emitter;
        if let Some(value) = state.resumed {
            let tagged = matches!(
                emitter.function.values[value.0],
                Type::General | Type::AsyncValue
            );
            if tagged {
                writeln!(
                    output,
                    "local.get $awaited_tag\nglobal.set {}",
                    emitter.value_offsets[value.0]
                )
                .unwrap();
            }
            writeln!(
                output,
                "local.get $awaited\nglobal.set {}",
                emitter.value_offsets[value.0] + usize::from(tagged)
            )
            .unwrap();
        }
        if id == fresh {
            if has_async || queued {
                writeln!(
                    output,
                    "i32.const {}\ncall $closure_frame\nglobal.set $async_frame",
                    frame.bytes()
                )
                .unwrap();
            }
            let emitter = &mut frame.emitter;
            emitter.init_cell_parameters();
        }
        let emitter = &mut frame.emitter;
        emitter.operations(state.operations);
        // Atomic emission only reads/writes numeric-indexed locals. Private
        // helpers retain their own locals and are appended separately below.
        frame.take_output(&mut output);
        let frame = frames[state.function].as_ref().expect("coroutine frame");
        let emitter = &frame.emitter;
        match state.end {
            End::AdoptPromise { subscription } => {
                // Promise resolution rejects itself immediately, before a
                // thenable-registration job. It is a guest error, not a trap.
                if errors && state.function != 0 {
                    let End::AdoptSubscription { input, .. } = split.states[subscription].end
                    else {
                        unreachable!()
                    };
                    writeln!(output,"global.get {}\nglobal.get $async_frame\ni32.load offset=16\ni32.eq\nif\nglobal.get $async_frame\ni32.load offset=16\ni32.const 1\ni32.const 4\ncall $type_error\ncall $promise_settle\ndrop",emitter.value_offsets[input.0]).unwrap();
                    promises::park(&mut output);
                    output.push_str("end\n");
                }
                frame.spill(&mut output);
                writeln!(output,"global.get $async_frame\ni32.const {subscription}\ni32.const 0\ni32.const 0\nf64.const 0\ncall $reaction_schedule").unwrap();
                promises::park(&mut output);
            }
            End::AdoptSubscription { input, reaction } => {
                writeln!(output,"global.get {}\nglobal.get $async_frame\ni32.const {reaction}\ncall $promise_subscribe",emitter.value_offsets[input.0]).unwrap();
                promises::park(&mut output);
            }
            End::EnterClosure { .. } => unreachable!(),
            End::AwaitStarted { reaction } => {
                frame.restore(&mut output);
                writeln!(output, "global.get $async_promise\nglobal.get $async_frame\ni32.const {reaction}\ncall $promise_subscribe").unwrap();
                promises::park(&mut output);
            }
            End::StartedClosure { result, next } => {
                frame.restore(&mut output);
                writeln!(
                    output,
                    "global.get $async_promise\nglobal.set {}",
                    emitter.value_offsets[result.0]
                )
                .unwrap();
                jump(&mut output, next);
            }
            End::AwaitPromise { input, reaction } => {
                frame.spill(&mut output);
                writeln!(output, "global.get {}\nglobal.get $async_frame\ni32.const {reaction}\ncall $promise_subscribe", emitter.value_offsets[input.0]).unwrap();
                promises::park(&mut output);
            }
            End::Reaction {
                fulfilled,
                rejected,
            } => {
                writeln!(output, "i32.const {rejected}\ni32.const {fulfilled}\nglobal.get $async_rejected\nselect\nglobal.set $pc\nbr $dispatch").unwrap();
            }

            End::InvokeClosure {
                callee,
                arguments,
                returned,
            } => {
                frame.spill(&mut output);
                let Type::Closure(signature) = emitter.function.values[callee.0] else {
                    unreachable!()
                };
                if let Some(targets) = dispatch_targets.get(&signature) {
                    for &index in targets {
                        let target = frames[index].as_ref().expect("indexed async closure");
                        writeln!(
                            output,
                            "global.get {}\ni32.const {index}\ni32.eq\nif",
                            emitter.value_offsets[callee.0]
                        )
                        .unwrap();
                        frame.invoke(
                            &mut output,
                            target,
                            callee,
                            arguments,
                            returned,
                            entries[index].expect("async entry"),
                        );
                        output.push_str("end\n");
                    }
                }
                output.push_str("unreachable\n");
            }
            End::ReturnedClosure {
                result,
                fulfilled,
                rejected,
            } => {
                frame.restore(&mut output);
                writeln!(output,"global.get $async_tag\nglobal.set {}\nglobal.get $async_payload\nglobal.set {}\ni32.const {rejected}\ni32.const {fulfilled}\nglobal.get $async_rejected\nselect\nglobal.set $pc\nbr $dispatch",emitter.value_offsets[result.0],emitter.value_offsets[result.0]+1).unwrap();
            }
            End::Jump(target) => jump(&mut output, target),
            End::Copy {
                input,
                result,
                next,
            } => {
                for part in 0..layout::value(emitter.function.values[input.0]).parts.len() {
                    writeln!(
                        output,
                        "global.get {}\nglobal.set {}",
                        emitter.value_offsets[input.0] + part,
                        emitter.value_offsets[result.0] + part
                    )
                    .unwrap();
                }
                jump(&mut output, next);
            }
            End::Test { condition, yes, no } => {
                writeln!(output, "i32.const {yes}\ni32.const {no}\nglobal.get {}\nselect\nglobal.set $pc\nbr $dispatch", emitter.value_offsets[condition.0]).unwrap();
            }
            End::SettledTest { input, yes, no } => {
                writeln!(output, "i32.const {yes}\ni32.const {no}\nglobal.get {}\ni32.const {}\ni32.and\nselect\nglobal.set $pc\nbr $dispatch", emitter.value_offsets[input.0],super::layout::SETTLED_REJECTED_BIT).unwrap();
            }
            End::Suspend {
                input,
                rejected,
                resume,
            } => {
                if queued {
                    frame.spill(&mut output);
                    writeln!(
                        output,
                        "global.get $async_frame\ni32.const {resume}\ni32.const {}",
                        u8::from(rejected)
                    )
                    .unwrap();
                    primitive(&mut output, emitter, input);
                    output.push_str("call $reaction_schedule\n");
                    promises::park(&mut output);
                    continue;
                }
                writeln!(
                    output,
                    "i32.const {}\nglobal.set $pc\ni32.const {}",
                    if rejected { -1 } else { resume as i32 },
                    if rejected { 2 } else { 1 }
                )
                .unwrap();
                primitive(&mut output, emitter, input);
                output.push_str("return\n");
            }
            End::Return(value) => {
                if state.function == 0 {
                    roots.clear(&mut output);
                }
                if queued {
                    promises::complete(&mut output, emitter, value, state.function == 0, false);
                    continue;
                }
                if state.function != 0 {
                    primitive(&mut output, emitter, value);
                    frames::complete(&mut output, false);
                    continue;
                }
                output.push_str("i32.const -1\nglobal.set $pc\ni32.const 0\n");
                primitive(&mut output, emitter, value);
                output.push_str("return\n");
            }
            End::Reject(value) => {
                if state.function == 0 {
                    roots.clear(&mut output);
                }
                if queued {
                    promises::complete(&mut output, emitter, value, state.function == 0, true);
                    continue;
                }
                if state.function != 0 {
                    primitive(&mut output, emitter, value);
                    frames::complete(&mut output, true);
                    continue;
                }
                output.push_str("i32.const -1\nglobal.set $pc\ni32.const 3\n");
                primitive(&mut output, emitter, value);
                output.push_str("return\n");
            }
            End::Trap => output.push_str("unreachable\n"),
        }
    }
    output.push_str("end\nend\nunreachable\n)\n");
    // $advance occupies logical source entry ID0. All source helpers and
    // lifted bodies retain their verified IDs before exported protocol wrappers.
    let mut needs_remainder = frames
        .iter()
        .flatten()
        .any(|frame| frame.emitter.needs_remainder);
    for (index, function) in outlined.iter().enumerate() {
        if ir::is_async_closure(function) {
            if starters {
                starters::emit(
                    &mut output,
                    frames[index + 1].as_ref().expect("async frame"),
                    index + 1,
                    entries[index + 1].expect("async entry"),
                );
                continue;
            }
            writeln!(
                output,
                "(func $closure_body{} (param i32 i32) (result i32 f64) unreachable)",
                index + 1
            )
            .unwrap();
            continue;
        }
        let mut header = if ir::closure_environment(function).is_some() {
            format!("(func $closure_body{} (param i32 i32)", index + 1)
        } else {
            let mut header = String::from("(func");
            if function.parameter_count > 0 {
                header.push_str(" (param");
                for slot in function.slots.iter().take(function.parameter_count) {
                    header.push(' ');
                    header.push_str(layout::value(slot.ty).wat);
                }
                header.push(')');
            }
            header
        };
        writeln!(
            header,
            " (result {})",
            layout::value(function.return_type).wat
        )
        .unwrap();
        let (body, remainder) = emit_function(function, header, errors, false);
        output.push_str(&body);
        needs_remainder |= remainder;
    }
    if queued {
        output.push_str(promises::WRAPPERS);
    } else {
        output.push_str(
            r#"(func (export "start") (param f64 f64) (result i32 i32 f64)
      global.get $pc if unreachable end
      local.get 0 global.set 0
      local.get 1 global.set 1
      i32.const 3 f64.const 0 call $advance)
      (func (export "resume") (param i32 f64) (result i32 i32 f64)
      global.get $pc i32.const 0 i32.le_s if unreachable end
      local.get 0 local.get 1 call $advance)
    "#,
        );
    }
    if needs_remainder {
        output.push_str(NUMBER_REMAINDER);
    }
    if has_cells {
        output.push_str(cells::ALLOCATOR);
    }
    if has_closures || queued {
        output.push_str(closures::FRAME_ALLOCATOR);
    }
    if queued {
        if errors {
            output.push_str(&errors::runtime());
        } else {
            output.push_str(promises::RUNTIME);
        }
    }
    if managed {
        output.push_str(include_str!("managed.wat"));
    }
    roots.finish(&mut output);
    output.push_str(")\n");
    Ok(output)
}

fn primitive(output: &mut String, emitter: &Emitter<'_>, value: ValueId) {
    let offset = emitter.value_offsets[value.0];
    if emitter.function.values[value.0] == Type::SettledPromise {
        writeln!(
            output,
            "global.get {offset}\ni32.const {}\ni32.and\nglobal.get {}",
            super::layout::SETTLED_TAG_MASK,
            offset + 1
        )
        .unwrap();
    } else if matches!(
        emitter.function.values[value.0],
        Type::General | Type::AsyncValue
    ) {
        writeln!(output, "global.get {offset}\nglobal.get {}", offset + 1).unwrap();
    } else {
        writeln!(output, "i32.const 3\nglobal.get {offset}").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_block_wrappers_split_on_a_small_native_stack() {
        use crate::ir::{Operation, RegionId};
        use oxc_span::Span;

        const DEPTH: usize = 8192;
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let span = Span::new(0, 0);
                let mut root = Region {
                    id: RegionId(DEPTH),
                    span,
                    operations: Vec::new(),
                    exit: Exit::Return(ValueId(7)),
                };
                for level in 0..DEPTH {
                    root = Region {
                        id: RegionId(level),
                        span,
                        operations: vec![Operation {
                            span,
                            kind: Op::Block(root),
                        }],
                        exit: Exit::Fallthrough,
                    };
                }
                let mut split = Splitter::default();
                let unreachable = split.terminal(End::Trap).unwrap();
                let first = split.region(&root, unreachable, None, None).unwrap();
                assert!(matches!(split.states[first].end, End::Return(ValueId(7))));
                assert_eq!(split.states.len(), 2);

                // Keep this test independent of the separate iterative
                // Region destructor change: manually release synthetic IR.
                let mut current = root;
                for _ in 0..DEPTH {
                    let operation = current.operations.pop().expect("nested block");
                    let Op::Block(child) = operation.kind else {
                        panic!("expected block");
                    };
                    current = child;
                }
                assert!(current.operations.is_empty());
            })
            .expect("small-stack async splitter thread")
            .join()
            .expect("stack-independent block splitting completed");
    }

    #[test]
    fn transparent_blocks_retain_yield_and_loop_boundaries() {
        use crate::ir::{Operation, RegionId};
        use oxc_span::Span;

        let span = Span::new(0, 0);
        let block = |exit| Region {
            id: RegionId(0),
            span,
            operations: vec![Operation {
                span,
                kind: Op::Block(Region {
                    id: RegionId(1),
                    span,
                    operations: Vec::new(),
                    exit,
                }),
            }],
            exit: Exit::Fallthrough,
        };
        let yielded_region = block(Exit::Yield(ValueId(0)));
        let mut split = Splitter::default();
        let normal = split.terminal(End::Trap).unwrap();
        let error = split
            .region(
                &yielded_region,
                normal,
                Some(YieldTarget::Copy {
                    result: ValueId(1),
                    next: normal,
                }),
                None,
            )
            .expect_err("a statement block must not inherit an enclosing yield target");
        assert!(error.contains("async yield lacks a target"), "{error}");
        let target = 9;
        assert_eq!(
            split
                .region(
                    &block(Exit::Continue),
                    normal,
                    None,
                    Some(LoopTargets {
                        exit: 8,
                        next: target,
                    }),
                )
                .unwrap(),
            target
        );
    }

    #[test]
    fn async_split_states_borrow_their_original_argument_snapshots() {
        use oxc_span::Span;

        // Both immediate child startup and awaited closure invocation share the
        // source-owned argument slice; no per-state copy is needed.
        for awaited in [false, true] {
            let span = Span::new(0, 0);
            let arguments = vec![ValueId(7), ValueId(3), ValueId(11)];
            let kind = if awaited {
                Op::AwaitClosureCall {
                    result: ValueId(12),
                    callee: ValueId(5),
                    arguments,
                }
            } else {
                Op::StartAsyncClosure {
                    result: ValueId(12),
                    callee: ValueId(5),
                    arguments,
                }
            };
            let region = Region {
                id: ir::RegionId(0),
                span,
                operations: vec![ir::Operation { span, kind }],
                exit: Exit::Fallthrough,
            };
            let source_arguments = match &region.operations[0].kind {
                Op::StartAsyncClosure { arguments, .. }
                | Op::AwaitClosureCall { arguments, .. } => arguments.as_slice(),
                _ => unreachable!(),
            };
            let mut split = Splitter::default();
            let done = split.terminal(End::Trap).unwrap();
            let entry = split.region(&region, done, None, None).unwrap();
            match &split.states[entry].end {
                End::InvokeClosure {
                    arguments: saved, ..
                } => {
                    assert_eq!(*saved, source_arguments);
                    assert_eq!(saved.as_ptr(), source_arguments.as_ptr());
                }
                _ => panic!("async invocation must use the source argument list"),
            }
        }
    }

    #[test]
    fn async_targets_cannot_use_the_source_entry_as_an_ordinary_call() {
        for body in [
            "return +run(a,b);",
            "const f=a?()=>+run(a,b):()=>b;return f();",
            "const f=a?()=>{const g=a?()=>+run(a,b):()=>b;return g();}:()=>b;return f();",
        ] {
            let source = format!("export async function run(a,b){{{body}}}");
            assert!(
                crate::compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("source function calls")
            );
        }
        let source = "function helper(a){return a;}export async function run(a,b){return await b;}";
        crate::compile_async(source, false).unwrap();
        for source in [
            "function helper(x){return +run(x,x);}export async function run(a,b){return a;}",
            "function helper(x){const f=x?()=>+run(x,x):()=>x;return f();}export async function run(a,b){return a;}",
        ] {
            assert!(
                crate::compile_async(source, false)
                    .err()
                    .unwrap()
                    .contains("source function calls")
            );
        }
    }

    #[test]
    fn a_typed_lifted_body_still_cannot_suspend() {
        let mut module = crate::lower_source_profile(
            "export async function run(a,b){const f=a?()=>1:()=>2;return f();}",
            false,
            Type::General,
            true,
        )
        .unwrap();
        let function = &mut module.functions[1];
        let Exit::Return(input) = function.body.exit else {
            unreachable!()
        };
        let result = ValueId(function.values.len());
        function.values.push(Type::Number);
        function.body.operations.push(ir::Operation {
            span: function.body.span,
            kind: Op::Await {
                result,
                input,
                rejected: false,
            },
        });
        function.body.exit = Exit::Return(result);
        ir::verify_module_with_entry_type(&module, Type::General).unwrap();
        assert!(
            emit_async(&module)
                .unwrap_err()
                .contains("closure bodies must be synchronous")
        );
    }

    #[test]
    fn ordinary_helpers_cannot_suspend_or_throw_in_typed_ir() {
        for throwing in [false, true] {
            let mut module = crate::lower_source_profile(
                "function helper(x){return x;}export async function run(a,b){return a;}",
                false,
                Type::General,
                true,
            )
            .unwrap();
            let function = &mut module.functions[1];
            let result = ValueId(function.values.len());
            let span = function.body.span;
            if throwing {
                function.return_type = Type::General;
                function.values.push(Type::General);
                function.body.operations.push(ir::Operation {
                    span,
                    kind: Op::Nullish {
                        result,
                        value: ir::Nullish::Undefined,
                    },
                });
                function.body.exit = Exit::Throw(result);
            } else {
                let Exit::Return(input) = function.body.exit else {
                    panic!("helper return")
                };
                function.values.push(Type::Number);
                function.body.operations.push(ir::Operation {
                    span,
                    kind: Op::Await {
                        result,
                        input,
                        rejected: false,
                    },
                });
                function.body.exit = Exit::Return(result);
            }
            // Primitive-returning helpers are well typed; the backend still
            // rejects both forms of control without a helper exception boundary.
            ir::verify_module_with_entry_type(&module, Type::General).unwrap();
            assert!(
                emit_async(&module)
                    .unwrap_err()
                    .contains("bodies must be synchronous")
            );
        }
    }
}
