//! Split verified regions into a switched-resume coroutine. The frame currently
//! conservatively keeps every source cell and value snapshot in instance globals.
use super::{Emitter, NUMBER_REMAINDER};
use crate::ir::{self, Exit, Op, Region, Type, ValueId};
use std::fmt::Write;

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
    Suspend {
        input: ValueId,
        rejected: bool,
        resume: usize,
    },
    Return(ValueId),
    Trap,
}

struct State<'a> {
    operations: &'a [ir::Operation],
    resumed: Option<ValueId>,
    end: End,
}

#[derive(Default)]
struct Splitter<'a> {
    states: Vec<State<'a>>,
    awaits: usize,
}

impl<'a> Splitter<'a> {
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
            operations,
            resumed,
            end,
        });
        Ok(id)
    }

    fn terminal(&mut self, end: End) -> Result<usize, String> {
        self.state(&[], None, end)
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
            Exit::Return(value) => self.terminal(End::Return(value))?,
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
                Op::Await {
                    result,
                    input,
                    rejected,
                } => {
                    self.awaits += 1;
                    if self.awaits > 32 {
                        return Err("async profile admits at most 32 await sites".into());
                    }
                    // Only the host resume entry supplies the awaited payload.
                    let resume = self.state(&[], Some(*result), End::Jump(next))?;
                    self.terminal(End::Suspend {
                        input: *input,
                        rejected: *rejected,
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
                Op::Call { .. } => {
                    return Err("async profile does not yet admit source function calls".into());
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
        Op::Await { .. }
            | Op::Block(_)
            | Op::If { .. }
            | Op::While { .. }
            | Op::DoWhile { .. }
            | Op::Call { .. }
    )
}

fn jump(output: &mut String, target: usize) {
    writeln!(output, "i32.const {target}\nglobal.set $pc\nbr $dispatch").unwrap();
}

pub(crate) fn emit_async(module: &ir::Module) -> Result<String, String> {
    ir::verify_module(module)?;
    let [function] = module.functions.as_slice() else {
        return Err("async profile currently requires a single entry function".into());
    };
    let mut split = Splitter::default();
    let fresh = split.terminal(End::Trap)?;
    let unreachable = split.terminal(End::Trap)?;
    let entry = split.region(&function.body, unreachable, None, None)?;
    split.states[fresh].end = End::Jump(entry);

    let mut types = Vec::new();
    let mut offsets = Vec::new();
    for ty in function
        .slots
        .iter()
        .map(|slot| slot.ty)
        .chain(function.values.iter().copied())
    {
        offsets.push(types.len());
        match ty {
            Type::Number => types.push("f64"),
            Type::Predicate => types.push("i32"),
            Type::General => types.extend(["i32", "f64"]),
        }
    }
    let mut emitter = Emitter {
        function,
        slot_offsets: offsets[..function.slots.len()].to_vec(),
        value_offsets: offsets[function.slots.len()..].to_vec(),
        output: String::new(),
        loop_label: None,
        needs_remainder: false,
    };
    let mut output = String::from("(module\n");
    for ty in types {
        writeln!(output, "(global (mut {ty}) ({ty}.const 0))").unwrap();
    }
    output.push_str("(global $pc (mut i32) (i32.const 0))\n(func $advance (param $awaited f64) (result i32 f64)\nblock $invalid\nloop $dispatch\n");
    // A br_table dispatch avoids scanning every state on each loop iteration.
    for id in 0..split.states.len() {
        writeln!(output, "block $state{id}").unwrap();
    }
    output.push_str("global.get $pc\nbr_table");
    for id in 0..split.states.len() {
        write!(output, " $state{id}").unwrap();
    }
    output.push_str(" $invalid\n");
    for state in split.states.iter().rev() {
        output.push_str("end\n");
        if let Some(value) = state.resumed {
            writeln!(
                output,
                "local.get $awaited\nglobal.set {}",
                emitter.value_offsets[value.0]
            )
            .unwrap();
        }
        emitter.operations(state.operations);
        // Atomic emission only reads/writes numeric-indexed locals. Private
        // helpers retain their own locals and are appended separately below.
        output.push_str(
            &emitter
                .output
                .replace("local.get ", "global.get ")
                .replace("local.set ", "global.set "),
        );
        emitter.output.clear();
        match state.end {
            End::Jump(target) => jump(&mut output, target),
            End::Copy {
                input,
                result,
                next,
            } => {
                let width = if function.values[input.0] == Type::General {
                    2
                } else {
                    1
                };
                for part in 0..width {
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
            End::Suspend {
                input,
                rejected,
                resume,
            } => {
                writeln!(
                    output,
                    "i32.const {}\nglobal.set $pc\ni32.const {}\nglobal.get {}\nreturn",
                    if rejected { -1 } else { resume as i32 },
                    if rejected { 2 } else { 1 },
                    emitter.value_offsets[input.0]
                )
                .unwrap();
            }
            End::Return(value) => {
                writeln!(
                    output,
                    "i32.const -1\nglobal.set $pc\ni32.const 0\nglobal.get {}\nreturn",
                    emitter.value_offsets[value.0]
                )
                .unwrap();
            }
            End::Trap => output.push_str("unreachable\n"),
        }
    }
    output.push_str("end\nend\nunreachable\n)\n");
    output.push_str(
        r#"(func (export "start") (param f64 f64) (result i32 f64)
      global.get $pc if unreachable end
      local.get 0 global.set 0
      local.get 1 global.set 1
      f64.const 0 call $advance)
      (func (export "resume") (param f64) (result i32 f64)
      global.get $pc i32.const 0 i32.le_s if unreachable end
      local.get 0 call $advance)
    "#,
    );
    if emitter.needs_remainder {
        output.push_str(NUMBER_REMAINDER);
    }
    output.push_str(")\n");
    Ok(output)
}
