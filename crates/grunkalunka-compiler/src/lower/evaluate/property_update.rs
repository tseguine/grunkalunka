//! Numeric own-data-property writes use saved references and Number snapshots.
use super::*;

pub(in crate::lower) fn compound(operator: AssignmentOperator) -> Option<Binary> {
    Some(match operator {
        AssignmentOperator::Addition => Binary::Add,
        AssignmentOperator::Subtraction => Binary::Subtract,
        AssignmentOperator::Multiplication => Binary::Multiply,
        AssignmentOperator::Division => Binary::Divide,
        AssignmentOperator::Remainder => Binary::Remainder,
        AssignmentOperator::BitwiseAnd => Binary::BitAnd,
        AssignmentOperator::BitwiseOR => Binary::BitOr,
        AssignmentOperator::BitwiseXOR => Binary::BitXor,
        AssignmentOperator::ShiftLeft => Binary::ShiftLeft,
        AssignmentOperator::ShiftRight => Binary::ShiftRight,
        AssignmentOperator::ShiftRightZeroFill => Binary::ShiftRightUnsigned,
        _ => return None,
    })
}

pub(super) enum Key<'e, 'a> {
    Literal(Vec<u16>),
    Uint32(&'e Expression<'a>),
    String(&'e Expression<'a>),
}

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        key: Key<'e, 'a>,
        operator: Binary,
        right: Option<&'e Expression<'a>>,
        postfix: bool,
        depth: usize,
    },
    Key {
        uint32: bool,
        node: &'e Expression<'a>,
        object: ValueId,
        operator: Binary,
        right: Option<&'e Expression<'a>>,
        postfix: bool,
        depth: usize,
    },
    Read {
        node: &'e Expression<'a>,
        object: ValueId,
        key: ir::ManagedKey,
        operator: Binary,
        right: Option<&'e Expression<'a>>,
        postfix: bool,
        depth: usize,
    },
    Write {
        node: &'e Expression<'a>,
        object: ValueId,
        key: ir::ManagedKey,
        old: ValueId,
        operator: Binary,
        postfix: bool,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_property_update<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if !matches!(
            mode,
            Mode::Number | Mode::General | Mode::Managed | Mode::Predicate | Mode::Discard
        ) {
            return Ok(false);
        }
        let (target, operator, right, postfix) = match node {
            Expression::UpdateExpression(update) => (
                &update.argument,
                if update.operator == UpdateOperator::Increment {
                    Binary::Add
                } else {
                    Binary::Subtract
                },
                None,
                !update.prefix,
            ),
            Expression::AssignmentExpression(assignment) => {
                let Some(operator) = compound(assignment.operator) else {
                    return Ok(false);
                };
                let Some(target) = assignment.left.as_simple_assignment_target() else {
                    return Ok(false);
                };
                (target, operator, Some(&assignment.right), false)
            }
            _ => return Ok(false),
        };
        let (object, key) = match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) if !member.optional => (
                &member.object,
                Key::Literal(member.property.name.encode_utf16().collect()),
            ),
            SimpleAssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
                let key = closures::unwrapped_at(&member.expression, depth + 2)
                    .and_then(|(key, _)| static_keys::literal(key));
                if let Some(key) = key {
                    (&member.object, Key::Literal(key))
                } else if self
                    .stored
                    .uint32_keys
                    .contains(&member.expression, depth + 2)
                {
                    (&member.object, Key::Uint32(&member.expression))
                } else if self.stored.dictionary(member.object.node_id())
                    && self.stored.string(member.expression.node_id())
                {
                    (&member.object, Key::String(&member.expression))
                } else {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        };
        let receiver = self.receiver_type(object, depth + 2);
        if !matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray))
            || !self.stored.primitive(target.node_id())
        {
            // Keep existing validation/first diagnostics for unsupported writes.
            return Ok(false);
        }
        let array = receiver == Some(Type::ManagedArray);
        let key = match key {
            Key::Literal(key) => Key::Literal(managed_member_key(
                node,
                key,
                array,
                self.stored.dictionary(object.node_id()),
            )?),
            key => key,
        };
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else {
            managed::number_consumer(work, node, mode)?;
        }
        work.push(Work::PropertyUpdate(Task::Receiver {
            node,
            key,
            operator,
            right,
            postfix,
            depth: depth + 1,
        }));
        work.push(Work::Eval(
            object,
            depth + 2,
            if array { Mode::Array } else { Mode::Object },
        ));
        Ok(true)
    }

    pub(super) fn resume_property_update<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) {
        match task {
            Task::Receiver {
                node,
                key,
                operator,
                right,
                postfix,
                depth,
            } => {
                let object = values.pop().expect("numeric write receiver").id();
                match key {
                    Key::Uint32(key) | Key::String(key) => {
                        let uint32 = self.stored.uint32_keys.contains(key, depth + 1);
                        work.push(Work::PropertyUpdate(Task::Key {
                            uint32,
                            node,
                            object,
                            operator,
                            right,
                            postfix,
                            depth,
                        }));
                        work.push(Work::Eval(
                            key,
                            depth + 1,
                            if uint32 { Mode::Number } else { Mode::Managed },
                        ));
                    }
                    Key::Literal(key) => work.push(Work::PropertyUpdate(Task::Read {
                        node,
                        object,
                        key: ir::ManagedKey::Literal(key),
                        operator,
                        right,
                        postfix,
                        depth,
                    })),
                }
            }
            Task::Key {
                uint32,
                node,
                object,
                operator,
                right,
                postfix,
                depth,
            } => {
                let key = values.pop().expect("numeric write key").id();
                work.push(Work::PropertyUpdate(Task::Read {
                    node,
                    object,
                    key: if uint32 {
                        ir::ManagedKey::Uint32(key)
                    } else {
                        ir::ManagedKey::String(key)
                    },
                    operator,
                    right,
                    postfix,
                    depth,
                }));
            }
            Task::Read {
                node,
                object,
                key,
                operator,
                right,
                postfix,
                depth,
            } => {
                let input = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    match &key {
                        ir::ManagedKey::Literal(key)
                            if self.values[object.0] == Type::ManagedArray =>
                        {
                            Op::ManagedArrayGetOwn {
                                result: input,
                                array: object,
                                key: key.clone(),
                            }
                        }
                        ir::ManagedKey::Literal(key) => Op::ManagedGetOwn {
                            result: input,
                            object,
                            key: key.clone(),
                        },
                        _ => Op::ManagedGetOwnDynamic {
                            result: input,
                            object,
                            key: key.clone(),
                        },
                    },
                );
                let old = self.value(Type::Number);
                // Source alternatives exclude String/reference/BigInt coercion.
                let primitive = self.value(Type::General);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedToPrimitive {
                        result: primitive,
                        input,
                    },
                );
                Self::push(
                    regions.current(),
                    node,
                    Op::NumberFromGeneral {
                        result: old,
                        input: primitive,
                    },
                );
                work.push(Work::PropertyUpdate(Task::Write {
                    node,
                    object,
                    key,
                    old,
                    operator,
                    postfix,
                }));
                if let Some(right) = right {
                    work.push(Work::Eval(
                        right,
                        depth,
                        if matches!(operator, Binary::Add) {
                            Mode::Arithmetic
                        } else {
                            Mode::Numeric
                        },
                    ));
                } else {
                    let one = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Constant {
                            result: one,
                            value: 1.0,
                        },
                    );
                    values.push(Value::Number(one));
                }
            }
            Task::Write {
                node,
                object,
                key,
                old,
                operator,
                postfix,
            } => {
                let right = values.pop().expect("numeric write right").number();
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::Binary {
                        result,
                        operator,
                        left: old,
                        right,
                    },
                );
                let primitive = self.value(Type::General);
                Self::push(
                    regions.current(),
                    node,
                    Op::Pack {
                        result: primitive,
                        input: result,
                    },
                );
                let value = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedPrimitive {
                        result: value,
                        input: primitive,
                    },
                );
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedSetOwnDynamic { object, key, value },
                );
                values.push(Value::Number(if postfix { old } else { result }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn static_primitive_updates_include_missing_fields_but_reject_coercing_kinds() {
        for (body, admitted) in [
            ("const o={n:false};return o.n++;", true),
            ("const o={n:null};return ++o.n;", true),
            ("const o={};return o.n+=b;", true),
            ("const xs=[,];return xs[0]++;", true),
            ("const xs=[false,null];return xs[0]+=b;", true),
            ("const xs=[];return xs[2]|=b;", true),
            ("const o={n:a?false:null};return o.n-=b;", true),
            ("const o={n:false};o.n++;return o.n;", false),
            ("const o={n:false};o.n++;return +o.n;", true),
            ("const o={n:'text'};return o.n++;", false),
            ("const o={n:{}};return o.n-=b;", false),
            ("const o={n:false};return o.n+='text';", false),
            ("const o={};return o.toString++;", false),
            ("const xs=[];return xs.length++;", false),
            ("const xs=[];return xs.map++;", false),
        ] {
            let s = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&s, ts);
                assert_eq!(r.is_ok(), admitted, "{s}: {:?}", r.err());
            }
        }
    }

    #[test]
    fn dictionary_numeric_updates_require_runtime_string_and_primitive_proofs() {
        for (body, admitted) in [
            (
                "const d=Object.create(null);const k='x';return d[k]++;",
                true,
            ),
            (
                "const d=Object.create(null);const k='x';d.x=false;return ++d[k];",
                true,
            ),
            (
                "const d=Object.create(null);const k='x';d.x=null;return d[k]+=b;",
                true,
            ),
            (
                "const d=Object.create(null);const k='x';d.x=a;return d[k]>>>=b;",
                true,
            ),
            ("const d={x:a};const k='x';return d[k]++;", false),
            ("const d=[a];const k='0';return d[k]++;", false),
            ("const d=Object.create(null);return d[a]++;", false),
            (
                "const d=Object.create(null);const k='x';d.x='text';return d[k]++;",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';d.x={};return d[k]-=b;",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';d.x=a;return d[k]+='text';",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';d.x=a;return d[k]-=1n;",
                false,
            ),
        ] {
            let s = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&s, ts);
                assert_eq!(r.is_ok(), admitted, "{s}: {:?}", r.err());
            }
        }
    }

    #[test]
    fn unsigned_numeric_updates_require_primitive_fields_and_range_proofs() {
        for (body, admitted) in [
            ("const xs=[a];return xs[0>>>0]++;", true),
            ("const xs=[a];return --xs[0>>>0];", true),
            ("const xs=[];return xs[a>>>0]++;", true),
            ("const o={0:a};return o[0>>>0]-=b;", true),
            ("const xs=[false,null];return xs[a>>>0]+=b;", true),
            ("const xs=[a];return xs[0>>>0]>>>=b;", true),
            ("const d=Object.create(null);return ++d[a>>>0];", true),
            ("const xs=[a];return xs[a]++;", false),
            ("const xs=[a];return xs[a>>0]++;", false),
            ("const xs=['text'];return xs[0>>>0]++;", false),
            ("const xs=[{}];return xs[0>>>0]-=b;", false),
            ("const xs=[a];return xs[0>>>0]+='text';", false),
            ("const xs=[a];return xs[0>>>0]-=1n;", false),
            ("return a[0>>>0]++;", false),
            (
                "const xs=[a];xs[0>>>0]++;return xs[0].trim().length;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){const xs=[a];return xs[0>>>0]-=b;}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[a];return xs[(0>>>0) as never]-=b;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }

    #[test]
    fn member_update_receivers_keep_source_depth_charges() {
        for (wraps, admitted) in [(126, true), (127, false)] {
            let source = format!(
                "export function run(a,b){{const o={{n:a}};return {}o{}.n++;}}",
                "(".repeat(wraps),
                ")".repeat(wraps)
            );
            let result = crate::compile(&source, false);
            assert_eq!(
                result.is_ok(),
                admitted,
                "wraps {wraps}: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn primitive_property_writes_require_known_kinds_and_keep_result_positions() {
        for (body, admitted) in [
            ("const o={n:a};return o.n++;", true),
            ("const o={n:a};return ++o.n;", true),
            ("const o={n:a};return o.n+=b;", true),
            ("const xs=[a];return xs[0]>>>=b;", true),
            ("const o={n:a};const box={x:o.n++};return box.x+b;", true),
            ("const o={n:a};o.n+=undefined;return o.n;", true),
            ("const o={n:'text'};o.n++;return a;", false),
            ("const o={};o.n++;return a;", true),
            ("const o={n:a};delete o.n;o.n++;return a;", true),
            ("const o={n:a};o.n='text';o.n++;return a;", false),
            ("const o={n:a};return o.n+='text';", false),
            ("const o={n:a};const k='n';return o[k]++;", false),
            ("const xs=[a];return xs.length++;", false),
            ("return a.n++;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
