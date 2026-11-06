//! Save both operands before executing surplus arguments or testing presence.
use super::*;

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    call: &'e CallExpression<'a>,
    depth: usize,
    index: usize,
    input: Option<ValueId>,
    key: Option<ValueId>,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_has_own<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        match mode {
            Mode::Predicate => (),
            Mode::General => work.push(Work::Pack(node)),
            Mode::Managed => {
                work.push(Work::Managed(managed::ManagedWork::Primitive(node)));
                work.push(Work::Pack(node));
            }
            Mode::Discard => work.push(Work::Forget),
            _ => return Err(unsupported(node, "Object.hasOwn returns a Boolean")),
        }
        if call.arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "at most 1000 Object.hasOwn arguments are admitted",
            ));
        }
        work.push(Work::HasOwn(Task {
            node,
            call,
            depth,
            index: 0,
            input: None,
            key: None,
        }));
        Ok(())
    }

    pub(super) fn resume_has_own<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let offset = invocation::offset(task.call);
        if task.index == offset + 1 {
            task.input = Some(values.pop().expect("has-own receiver").id());
        } else if task.index == offset + 2 {
            task.key = Some(values.pop().expect("has-own key").id());
        }
        let Some(arg) = task.call.arguments.get(task.index) else {
            let input = task.input.ok_or_else(|| {
                unsupported(task.node, "Object.hasOwn requires a proven object or array")
            })?;
            let key = task.key.ok_or_else(|| {
                unsupported(task.node, "Object.hasOwn requires a proven String key")
            })?;
            let result = self.value(Type::Predicate);
            Self::push(
                regions.current(),
                task.node,
                Op::ManagedHasOwn { result, input, key },
            );
            values.push(Value::Predicate(result));
            return Ok(());
        };
        let arg = arg
            .as_expression()
            .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
        let depth = task.depth + 1;
        let mode = if task.index == offset {
            match self.receiver_type(arg, depth) {
                Some(Type::ManagedObject) => Mode::Object,
                Some(Type::ManagedArray) => Mode::Array,
                _ => {
                    return Err(unsupported(
                        arg,
                        "Object.hasOwn requires a proven object or array",
                    ));
                }
            }
        } else if task.index == offset + 1 {
            if !self.stored.string(arg.node_id()) {
                return Err(unsupported(
                    arg,
                    "Object.hasOwn requires a proven String key",
                ));
            }
            Mode::Managed
        } else {
            Mode::Discard
        };
        task.index += 1;
        work.push(Work::HasOwn(task));
        work.push(Work::Eval(arg, depth, mode));
        Ok(())
    }
}
