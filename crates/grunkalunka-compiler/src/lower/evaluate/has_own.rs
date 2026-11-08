//! Own-property calls and dictionary membership share a presence check after
//! evaluating their operands in the order required by the source operation.
use super::*;

pub(super) enum Membership {
    Own(Option<Vec<u16>>),
    Uint32,
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
        let uint32 = property_delete::uint32_key(&binary.left, depth + 1);
        let key_mode = if literal.is_some() {
            Mode::Discard
        } else if uint32 {
            Mode::Number
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
            (
                Mode::Object,
                if uint32 {
                    Membership::Uint32
                } else {
                    Membership::Own(literal)
                },
            )
        } else if let Some(ty @ (Type::ManagedObject | Type::ManagedArray)) = ty {
            let known_prototype = ty == Type::ManagedArray || self.stored.ordinary_object(receiver);
            let membership = if uint32 {
                // Decimal uint32 names are absent from the pristine intrinsic
                // prototypes, just like supported literal numeric keys.
                Membership::Uint32
            } else if let Some(key) = literal {
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

#[cfg(test)]
mod uint32_membership_tests {
    #[test]
    fn unsigned_in_keys_retain_source_range_receiver_and_operand_checks() {
        for (body, admitted) in [
            ("const xs=[a,,b];return (a>>>0) in xs?1:0;", true),
            ("const o={0:a};return (a>>>b) in o?1:0;", true),
            ("const d=Object.create(null);return (a>>>0) in d?1:0;", true),
            (
                "const o=a?{}:Object.create(null);return (a>>>0) in o?1:0;",
                true,
            ),
            ("const xs=[a];let key=0;return (key=a>>>0) in xs?1:0;", true),
            ("const xs=[a];return a in xs?1:0;", false),
            ("const xs=[a];return (a>>0) in xs?1:0;", false),
            ("return (a>>>0) in a?1:0;", false),
            ("const xs=[a];return (a?(0>>>0):b) in xs?1:0;", false),
            (
                "const xs=[a];return (Math.random(),a>>>0) in xs?1:0;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        let js = "export function run(a,b){const xs=[a];return (a>>>0) in xs?1:0;}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[a];return ((a>>>0) as never) in xs?1:0;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
        assert!(crate::compile("export function run(a:number,b:number){const xs=[a];return (a as number) in xs?1:0;}",true).is_err());
    }
}
