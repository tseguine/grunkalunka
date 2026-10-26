//! Split verified regions into a switched-resume coroutine. The frame currently
//! conservatively keeps source locals, captured-cell pointers and snapshots in
//! instance globals; captured cell contents live in the invocation's guest arena.
use super::{Emitter, NUMBER_REMAINDER, cells, closures, emit_function, layout};
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
    Reject(ValueId),
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
    return_to: Option<(ValueId, usize)>,
    catch_to: Option<(ValueId, usize)>,
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
                Op::InlineCall { result, body } => {
                    let outer = self.return_to.replace((*result, next));
                    let entry = self.region(body, next, None, None);
                    self.return_to = outer;
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
                Op::Await {
                    result,
                    input,
                    rejected,
                } => {
                    self.awaits += 1;
                    if self.awaits > 32 {
                        return Err("async profile admits at most 32 await sites".into());
                    }
                    // A caught rejection resumes into its lexical handler on the
                    // same reaction job. The host still supplies the original pair.
                    let caught = rejected.then_some(self.catch_to).flatten();
                    let (resumed, target) = caught.unwrap_or((*result, next));
                    let resume = self.state(&[], Some(resumed), End::Jump(target))?;
                    self.terminal(End::Suspend {
                        input: *input,
                        rejected: *rejected && caught.is_none(),
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
            | Op::TryCatch { .. }
            | Op::InlineCall { .. }
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
    ir::verify_module_with_entry_type(module, Type::General)?;
    let Some((function, lifted)) = module.functions.split_first() else {
        return Err("async profile currently requires a single entry function".into());
    };
    if lifted
        .iter()
        .any(|function| ir::closure_environment(function).is_none())
    {
        return Err("async profile currently requires a single source entry function".into());
    }
    for (index, function) in module.functions.iter().enumerate() {
        let mut source_call = false;
        let mut await_site = false;
        ir::visit_operations(&function.body, &mut |op| {
            source_call |= matches!(op, Op::Call { .. });
            await_site |= matches!(op, Op::Await { .. } | Op::TryCatch { .. });
        });
        // The source entry's logical ID does not denote a callable numeric Wasm
        // function in this module; its physical implementation is $advance.
        if source_call {
            return Err("async profile does not yet admit source function calls".into());
        }
        if index != 0 && await_site {
            return Err("async closure bodies must be synchronous".into());
        }
    }
    let has_closures = !lifted.is_empty();
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
        types.extend(layout::value(ty).parts.iter().map(|part| part.lane.wat()));
    }
    let mut offset = types.len();
    let cell_offsets = cells::offsets(function, &mut offset);
    let has_cells = has_closures || offset != types.len();
    let arena_temp = closures::temporary(function, offset);
    types.resize(offset + usize::from(arena_temp.is_some()), "i32");
    let mut emitter = Emitter {
        function,
        slot_offsets: offsets[..function.slots.len()].to_vec(),
        value_offsets: offsets[function.slots.len()..].to_vec(),
        cell_offsets,
        arena_temp,
        output: String::new(),
        loop_label: None,
        inline_return: None,
        needs_remainder: false,
    };
    let mut output = String::from("(module\n");
    for ty in types {
        writeln!(output, "(global (mut {ty}) ({ty}.const 0))").unwrap();
    }
    if has_cells {
        output.push_str(cells::MEMORY);
    }
    if has_closures {
        output.push_str(&closures::table(module, true));
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
        if let Some(value) = state.resumed {
            if function.values[value.0] == Type::General {
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
                emitter.value_offsets[value.0]
                    + usize::from(function.values[value.0] == Type::General)
            )
            .unwrap();
        }
        if id == fresh {
            emitter.init_cell_parameters();
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
                for part in 0..layout::value(function.values[input.0]).parts.len() {
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
                    "i32.const {}\nglobal.set $pc\ni32.const {}",
                    if rejected { -1 } else { resume as i32 },
                    if rejected { 2 } else { 1 }
                )
                .unwrap();
                primitive(&mut output, &emitter, input);
                output.push_str("return\n");
            }
            End::Return(value) => {
                output.push_str("i32.const -1\nglobal.set $pc\ni32.const 0\n");
                primitive(&mut output, &emitter, value);
                output.push_str("return\n");
            }
            End::Reject(value) => {
                output.push_str("i32.const -1\nglobal.set $pc\ni32.const 3\n");
                primitive(&mut output, &emitter, value);
                output.push_str("return\n");
            }
            End::Trap => output.push_str("unreachable\n"),
        }
    }
    output.push_str("end\nend\nunreachable\n)\n");
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
    for (index, function) in lifted.iter().enumerate() {
        let header = format!(
            "(func $closure_body{} (param i32 i32) (result {})\n",
            index + 1,
            layout::value(function.return_type).wat
        );
        let (body, remainder) = emit_function(function, header);
        output.push_str(&body);
        emitter.needs_remainder |= remainder;
    }
    if emitter.needs_remainder {
        output.push_str(NUMBER_REMAINDER);
    }
    if has_cells {
        output.push_str(cells::ALLOCATOR);
    }
    if has_closures {
        output.push_str(closures::FRAME_ALLOCATOR);
    }
    output.push_str(")\n");
    Ok(output)
}

fn primitive(output: &mut String, emitter: &Emitter<'_>, value: ValueId) {
    let offset = emitter.value_offsets[value.0];
    if emitter.function.values[value.0] == Type::General {
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
        assert!(
            crate::compile_async(source, false)
                .err()
                .unwrap()
                .contains("single source entry")
        );
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
}
