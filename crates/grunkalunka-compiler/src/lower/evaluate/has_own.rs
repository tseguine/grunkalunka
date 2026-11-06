//! Own-property calls and dictionary membership share a presence check after
//! evaluating their operands in the order required by the source operation.
use super::*;

pub(super) enum Membership {
    Own(Option<Vec<u16>>),
    Inherited,
    Prototype,
}

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    call: &'e CallExpression<'a>,
    depth: usize,
    index: usize,
    input: Option<ValueId>,
    key: Option<ir::ManagedKey>,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_in<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        binary: &'e oxc_ast::ast::BinaryExpression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        boolean_consumer(node, mode, "property presence produces a Boolean", work)?;
        let literal = literal_property_name(&binary.left, self.semantic, depth + 1);
        let key_mode = if literal.is_some() {
            Mode::Discard
        } else if self.stored.string(binary.left.node_id()) {
            Mode::Managed
        } else {
            return Err(unsupported(
                &binary.left,
                "in requires a proven String or supported primitive key",
            ));
        };
        let receiver = binary.right.node_id();
        let ty = self.receiver_type(&binary.right, depth + 1);
        let (mode, membership) = if self.stored.dictionary(receiver) {
            (Mode::Object, Membership::Own(literal))
        } else if let Some(ty @ (Type::ManagedObject | Type::ManagedArray)) = ty {
            let known_prototype = ty == Type::ManagedArray || self.stored.ordinary_object(receiver);
            let membership = if let Some(key) = literal {
                let inherited =
                    crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &key)
                        || (ty == Type::ManagedArray
                            && crate::prototype_keys::contains(crate::prototype_keys::ARRAY, &key));
                if inherited && !known_prototype {
                    return Err(unsupported(
                        &binary.right,
                        "in cannot assume an inherited key on a mixed-prototype receiver",
                    ));
                }
                if inherited {
                    Membership::Inherited
                } else {
                    Membership::Own(Some(key))
                }
            } else if known_prototype {
                Membership::Prototype
            } else {
                return Err(unsupported(
                    &binary.right,
                    "a dynamic in key requires a known receiver prototype",
                ));
            };
            (Mode::managed(ty), membership)
        } else {
            return Err(unsupported(
                &binary.right,
                "in requires a proven object or array",
            ));
        };
        // Stack the RHS first so the cursor saves the LHS key before RHS effects.
        work.push(Work::In { node, membership });
        work.push(Work::Eval(&binary.right, depth + 1, mode));
        work.push(Work::Eval(&binary.left, depth + 1, key_mode));
        Ok(())
    }

    pub(super) fn finish_property_presence(
        &mut self,
        node: &Expression<'_>,
        output: &mut Region,
        values: &mut Vec<Value>,
        input: ValueId,
        key: ir::ManagedKey,
    ) {
        let result = self.value(Type::Predicate);
        Self::push(output, node, Op::ManagedHasOwn { result, input, key });
        values.push(Value::Predicate(result));
    }

    pub(super) fn schedule_has_own<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        boolean_consumer(node, mode, "property presence produces a Boolean", work)?;
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
        } else if task.index == offset + 2 && task.key.is_none() {
            task.key = Some(ir::ManagedKey::String(
                values.pop().expect("has-own key").id(),
            ));
        }
        let Some(arg) = task.call.arguments.get(task.index) else {
            let input = task.input.ok_or_else(|| {
                unsupported(
                    task.node,
                    "Object.hasOwn requires a proven object, array, String, Number or Boolean",
                )
            })?;
            let key = task
                .key
                .unwrap_or_else(|| ir::ManagedKey::Literal("undefined".encode_utf16().collect()));
            self.finish_property_presence(task.node, regions.current(), values, input, key);
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
                _ if self.stored.string(arg.node_id()) => Mode::Managed,
                _ if self.operand_boolean_shape(arg, depth) => Mode::Predicate,
                _ if self.numeric_result_shape(arg, depth) => Mode::Number,
                _ => {
                    return Err(unsupported(
                        arg,
                        "Object.hasOwn requires a proven object, array, String, Number or Boolean",
                    ));
                }
            }
        } else if task.index == offset + 1 {
            if let Some(units) = literal_property_name(arg, self.semantic, depth) {
                task.key = Some(ir::ManagedKey::Literal(units));
                // In particular, void's operand still runs at the key's position.
                Mode::Discard
            } else if self.stored.string(arg.node_id()) {
                Mode::Managed
            } else {
                return Err(unsupported(
                    arg,
                    "Object.hasOwn requires a known String or supported primitive key",
                ));
            }
        } else {
            Mode::Discard
        };
        task.index += 1;
        work.push(Work::HasOwn(task));
        work.push(Work::Eval(arg, depth, mode));
        Ok(())
    }
}

pub(super) fn literal_property_name(
    arg: &Expression<'_>,
    semantic: &Semantic<'_>,
    depth: usize,
) -> Option<Vec<u16>> {
    let key = closures::unwrapped(arg, depth)?;
    if let Some(units) = static_keys::literal(key) {
        return Some(units);
    }
    let name = match key {
        Expression::BooleanLiteral(value) => {
            if value.value {
                "true"
            } else {
                "false"
            }
        }
        Expression::NullLiteral(_) => "null",
        _ if crate::lower::string_methods::undefined(key, semantic) => "undefined",
        _ => return None,
    };
    Some(name.encode_utf16().collect())
}
