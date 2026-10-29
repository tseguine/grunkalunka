//! Split verified regions into a switched-resume coroutine. The frame currently
//! conservatively keeps source locals, captured-cell pointers and snapshots in
//! instance globals; captured cell contents live in the invocation's guest arena.
use super::{Emitter, NUMBER_REMAINDER, cells, closures, emit_function, layout};
use crate::ir::{self, Exit, Op, Region, Type, ValueId};
use std::fmt::Write;

mod errors;
mod frames;
mod promises;
mod starters;

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

enum End {
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
        arguments: Vec<ValueId>,
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
    end: End,
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
        end: End,
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

    fn terminal(&mut self, end: End) -> Result<usize, String> {
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

    // Build backwards from the suffix. Every internal edge stays in the current
    // host job; only Suspend returns to the scheduler. Loop targets are lexical.
    fn region(
        &mut self,
        region: &'a Region,
        normal: usize,
        yielded: Option<YieldTarget>,
        loops: Option<LoopTargets>,
    ) -> Result<usize, String> {
        let mut next = match region.exit {
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
        let mut end = region.operations.len();
        while end > 0 {
            let operation = &region.operations[end - 1];
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
                        arguments: arguments.clone(),
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
                        arguments: arguments.clone(),
                        returned,
                    })?
                }
                Op::InlineCall { result, body } => {
                    let outer = self.return_to.replace((*result, next));
                    let entry = self.region(body, next, None, None);
                    self.return_to = outer;
                    entry?
                }
                Op::AwaitCall { result, body } => {
                    // Two possible settlements share one logical await site.
                    // The body runs now; only the caller reaction suspends here.
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
                    let entry = self.region(body, fulfilled, None, None);
                    self.return_to = outer_return;
                    self.catch_to = outer_catch;
                    entry?
                }
                Op::TryCatch {
                    body,
                    reason,
                    handler,
                } => {
                    // The handler itself belongs to the enclosing protected scope.
                    let handler = self.region(handler, next, None, loops)?;
                    let outer = self.catch_to.replace((*reason, handler));
                    let entry = self.region(body, next, None, loops);
                    self.catch_to = outer;
                    entry?
                }
                Op::TryFinally {
                    body,
                    finalizer,
                    returned,
                    reason,
                    continuation,
                } => {
                    let outer_return = self.return_to;
                    let outer_catch = self.catch_to;
                    let resume_return = self.region(continuation, next, None, loops)?;
                    self.return_to = Some((*returned, resume_return));
                    let entries = (|| {
                        // Cleanup belongs to the outer lexical scope. A return in
                        // it overrides the pending completion through resume_return.
                        let on_return = self.region(finalizer, resume_return, None, loops)?;
                        let rethrow = self.terminal(match outer_catch {
                            Some((result, next)) => End::Copy {
                                input: *reason,
                                result,
                                next,
                            },
                            None => End::Reject(*reason),
                        })?;
                        let on_throw = self.region(finalizer, rethrow, None, loops)?;
                        let normal = self.region(finalizer, next, None, loops)?;
                        let protected_loops = if let Some(targets) = loops {
                            Some(LoopTargets {
                                exit: self.region(finalizer, targets.exit, None, loops)?,
                                next: self.region(finalizer, targets.next, None, loops)?,
                            })
                        } else {
                            None
                        };
                        self.return_to = Some((*returned, on_return));
                        self.catch_to = Some((*reason, on_throw));
                        self.region(body, normal, None, protected_loops)
                    })();
                    self.return_to = outer_return;
                    self.catch_to = outer_catch;
                    entries?
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
                Op::Block(child) => self.region(child, next, None, loops)?,
                Op::If {
                    result,
                    condition,
                    consequent,
                    alternate,
                } => {
                    let yielded = result.map(|result| YieldTarget::Copy { result, next });
                    let yes = self.region(consequent, next, yielded, loops)?;
                    let no = self.region(alternate, next, yielded, loops)?;
                    self.terminal(End::Test {
                        condition: *condition,
                        yes,
                        no,
                    })?
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    let test = self.terminal(End::Trap)?;
                    let step = match step {
                        Some(step) => self.region(step, test, None, loops)?,
                        None => test,
                    };
                    let body = self.region(
                        body,
                        step,
                        None,
                        Some(LoopTargets {
                            exit: next,
                            next: step,
                        }),
                    )?;
                    let condition = self.region(
                        condition,
                        next,
                        Some(YieldTarget::Test {
                            yes: body,
                            no: next,
                        }),
                        loops,
                    )?;
                    self.states[test].end = End::Jump(condition);
                    test
                }
                Op::DoWhile { body, condition } => {
                    let first = self.terminal(End::Trap)?;
                    let condition = self.region(
                        condition,
                        next,
                        Some(YieldTarget::Test {
                            yes: first,
                            no: next,
                        }),
                        loops,
                    )?;
                    let body = self.region(
                        body,
                        condition,
                        None,
                        Some(LoopTargets {
                            exit: next,
                            next: condition,
                        }),
                    )?;
                    self.states[first].end = End::Jump(body);
                    first
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
        Ok(next)
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
    let has_async = entries.iter().any(Option::is_some);
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
    for ty in types {
        writeln!(output, "(global (mut {ty}) ({ty}.const 0))").unwrap();
    }
    if has_cells {
        output.push_str(cells::MEMORY);
    }
    if has_closures {
        output.push_str(&closures::table(module, true, starters));
    }
    if has_async || queued {
        output.push_str(frames::GLOBALS);
    }
    if queued {
        output.push_str(promises::GLOBALS);
    }
    output.push_str("(global $pc (mut i32) (i32.const 0))\n(func $advance (param $awaited_tag i32) (param $awaited f64) (result i32 i32 f64)\nblock $invalid\nloop $dispatch\n");
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
                ref arguments,
                returned,
            } => {
                frame.spill(&mut output);
                let Type::Closure(signature) = emitter.function.values[callee.0] else {
                    unreachable!()
                };
                for (index, target) in frames.iter().enumerate().skip(1) {
                    let Some(target) = target else {
                        continue;
                    };
                    let parameters: Vec<_> = target
                        .emitter
                        .function
                        .slots
                        .iter()
                        .take(target.emitter.function.parameter_count)
                        .map(|slot| slot.ty)
                        .collect();
                    if module.closure_signatures.id_async(&parameters) != Some(signature) {
                        continue;
                    }
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
                for _ in 0..function.parameter_count {
                    header.push_str(" f64");
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
        let (body, remainder) = emit_function(function, header, errors);
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
            if throwing {
                assert!(ir::verify_module_with_entry_type(&module, Type::General).is_err());
                assert!(emit_async(&module).is_err());
            } else {
                ir::verify_module_with_entry_type(&module, Type::General).unwrap();
                assert!(
                    emit_async(&module)
                        .unwrap_err()
                        .contains("bodies must be synchronous")
                );
            }
        }
    }
}
