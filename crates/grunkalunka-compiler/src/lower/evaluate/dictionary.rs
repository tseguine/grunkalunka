//! Runtime dictionary keys are snapshots on the ordinary expression cursor.
use super::*;

pub(super) enum Task<'e, 'a> {
    ReadReceiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
        uint32: bool,
    },
    ReadKey {
        node: &'e Expression<'a>,
        object: ValueId,
        uint32: bool,
    },
    SetReceiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        uint32: bool,
    },
    SetKey {
        node: &'e Expression<'a>,
        object: ValueId,
        right: &'e Expression<'a>,
        depth: usize,
        uint32: bool,
    },
    SetValue {
        node: &'e Expression<'a>,
        object: ValueId,
        key: ir::ManagedKey,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_dictionary_read<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::ComputedMemberExpression(member) = node else {
            return Ok(false);
        };
        let uint32 = self
            .stored
            .uint32_keys
            .contains(&member.expression, depth + 1);
        let dictionary = self.stored.dictionary(member.object.node_id());
        let receiver = self.receiver_type(&member.object, depth + 1);
        if member.optional
            || !(dictionary
                || (uint32 && matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray))))
        {
            return Ok(false);
        }
        if closures::unwrapped(&member.expression, depth + 1)
            .and_then(static_keys::literal)
            .is_some()
        {
            return Ok(false);
        }
        if !uint32 && !self.stored.string(member.expression.node_id()) {
            return Err(unsupported(
                node,
                "runtime dictionary keys require a known String",
            ));
        }
        work.push(Work::Dictionary(Task::ReadReceiver {
            node,
            key: &member.expression,
            depth: depth + 1,
            uint32,
        }));
        work.push(Work::Eval(
            &member.object,
            depth + 1,
            if dictionary {
                Mode::Object
            } else {
                Mode::managed(receiver.expect("validated read receiver"))
            },
        ));
        Ok(true)
    }

    pub(super) fn schedule_dictionary_write<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::AssignmentExpression(assignment) = node else {
            return Ok(false);
        };
        let Some(SimpleAssignmentTarget::ComputedMemberExpression(member)) =
            assignment.left.as_simple_assignment_target()
        else {
            return Ok(false);
        };
        let uint32 = self
            .stored
            .uint32_keys
            .contains(&member.expression, depth + 1);
        let dictionary = self.stored.dictionary(member.object.node_id());
        let receiver = self.receiver_type(&member.object, depth + 1);
        if member.optional
            || !(dictionary
                || (uint32 && matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray))))
        {
            return Ok(false);
        }
        if closures::unwrapped(&member.expression, depth + 1)
            .and_then(static_keys::literal)
            .is_some()
        {
            return Ok(false);
        }
        if assignment.operator != AssignmentOperator::Assign {
            return Err(unsupported(
                node,
                "dictionary properties require plain assignment",
            ));
        }
        if !uint32 && !self.stored.string(member.expression.node_id()) {
            return Err(unsupported(
                node,
                "runtime dictionary keys require a known String",
            ));
        }
        work.push(Work::Dictionary(Task::SetReceiver {
            node,
            key: &member.expression,
            right: &assignment.right,
            depth: depth + 1,
            uint32,
        }));
        work.push(Work::Eval(
            &member.object,
            depth + 1,
            if dictionary {
                Mode::Object
            } else {
                Mode::managed(receiver.expect("validated unsigned write receiver"))
            },
        ));
        Ok(true)
    }

    pub(super) fn resume_dictionary<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        match task {
            Task::ReadReceiver {
                node,
                key,
                depth,
                uint32,
            } => {
                let object = values.pop().expect("dictionary receiver").id();
                work.push(Work::Dictionary(Task::ReadKey {
                    node,
                    object,
                    uint32,
                }));
                work.push(Work::Eval(
                    key,
                    depth,
                    if uint32 { Mode::Number } else { Mode::Managed },
                ));
            }
            Task::ReadKey {
                node,
                object,
                uint32,
            } => {
                let key = values.pop().expect("dictionary key").id();
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedGetOwnDynamic {
                        result,
                        object,
                        key: if uint32 {
                            ir::ManagedKey::Uint32(key)
                        } else {
                            ir::ManagedKey::String(key)
                        },
                    },
                );
                values.push(Value::Managed(result));
            }
            Task::SetReceiver {
                node,
                key,
                right,
                depth,
                uint32,
            } => {
                let object = values.pop().expect("dictionary assignment receiver").id();
                work.push(Work::Dictionary(Task::SetKey {
                    node,
                    object,
                    right,
                    depth,
                    uint32,
                }));
                work.push(Work::Eval(
                    key,
                    depth,
                    if uint32 { Mode::Number } else { Mode::Managed },
                ));
            }
            Task::SetKey {
                node,
                object,
                right,
                depth,
                uint32,
            } => {
                let key = values.pop().expect("dictionary assignment key").id();
                let key = if uint32 {
                    ir::ManagedKey::Uint32(key)
                } else {
                    ir::ManagedKey::String(key)
                };
                work.push(Work::Dictionary(Task::SetValue { node, object, key }));
                work.push(Work::Eval(right, depth, Mode::Managed));
            }
            Task::SetValue { node, object, key } => {
                let value = values.last().expect("dictionary assignment value").id();
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedSetOwnDynamic { object, key, value },
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod uint32_write_tests {
    #[test]
    fn unsigned_property_writes_retain_range_receiver_and_alias_guards() {
        for (body, admitted) in [
            ("const xs=[];xs[a>>>0]=b;return xs.length;", true),
            ("const o={};o[a>>>b]=b;return Object.keys(o).length;", true),
            (
                "const d=Object.create(null);d[a>>>0]=b;return Object.keys(d).length;",
                true,
            ),
            (
                "const xs=[];xs[a?(0>>>0):(1>>>0)]=b;return xs.length;",
                true,
            ),
            ("const xs=[];xs[a]=b;return a;", false),
            ("const xs=[];xs[a>>0]=b;return a;", false),
            ("const xs=[];xs[a>>>0]+=b;return a;", true),
            ("const xs=[];const key='length';xs[key]=b;return a;", false),
            (
                "const xs=['text'];const alias=xs;alias[a>>>0]=b;return xs.length;",
                true,
            ),
            (
                "const xs=['text'];const alias=xs;alias[a>>>0]=b;return xs[0].trim().length;",
                false,
            ),
            (
                "const box={d:Object.create(null)};const key='x';box.d[key]=a;return Object.keys(box.d).length;",
                true,
            ),
            ("a[0>>>0]=b;return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        let js = "export function run(a,b){const xs=[];xs[a>>>0]=b;return xs.length;}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[];(xs as never)[((a>>>0) as number)]=b;return xs.length;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
}

#[cfg(test)]
mod unsigned_read_tests {
    #[test]
    fn unsigned_reads_keep_range_receiver_and_value_guards() {
        for (body, ok) in [
            ("const xs=[a];return +xs[b>>>0];", true),
            ("const o={0:a};return +o[b>>>0];", true),
            ("const xs=[a];return +xs[b];", false),
            ("const xs=[a];return +xs[b>>0];", false),
            ("const xs=[a];return +xs[b?(0>>>0):a];", false),
            ("return +a[b>>>0];", false),
            (
                "const xs=['text'];const alias=xs;alias[0]=a;return xs[b>>>0].trim().length;",
                false,
            ),
        ] {
            let s = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&s, ts);
                assert_eq!(r.is_ok(), ok, "{s}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){const xs=[a];return +xs[b>>>0];}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[a];return +xs[(b>>>0) as never];}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
}
