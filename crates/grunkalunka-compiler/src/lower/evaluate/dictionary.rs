//! Runtime dictionary keys are snapshots on the ordinary expression cursor.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum KeyAccess {
    Read,
    Write,
}

#[derive(Clone, Copy)]
pub(in crate::lower) enum RuntimeKey {
    Uint32,
    Scalar { numbers: bool },
    String,
}

impl RuntimeKey {
    pub(in crate::lower) fn finish(self, input: ValueId) -> ir::ManagedKey {
        match self {
            Self::Uint32 => ir::ManagedKey::Uint32(input),
            Self::Scalar { numbers } => ir::ManagedKey::Scalar { input, numbers },
            Self::String => ir::ManagedKey::String(input),
        }
    }
}

pub(super) enum Task<'e, 'a> {
    ReadReceiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
        kind: RuntimeKey,
    },
    ReadKey {
        node: &'e Expression<'a>,
        object: ValueId,
        kind: RuntimeKey,
    },
    SetReceiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        kind: RuntimeKey,
    },
    SetKey {
        node: &'e Expression<'a>,
        object: ValueId,
        right: &'e Expression<'a>,
        depth: usize,
        kind: RuntimeKey,
    },
    SetValue {
        node: &'e Expression<'a>,
        object: ValueId,
        key: ir::ManagedKey,
    },
}

impl Lowering<'_, '_> {
    pub(in crate::lower) fn runtime_key(
        &self,
        key: &Expression<'_>,
        depth: usize,
    ) -> Option<RuntimeKey> {
        if self.stored.uint32_keys.contains(key, depth) {
            Some(RuntimeKey::Uint32)
        } else if self.stored.primitive(key.node_id()) {
            Some(RuntimeKey::Scalar {
                numbers: self.stored.may_be_number(key.node_id()),
            })
        } else {
            self.stored
                .scalar_string_operand(key.node_id())
                .then_some(RuntimeKey::String)
        }
    }

    pub(super) fn schedule_runtime_key<'e, 'a>(
        &self,
        key: &'e Expression<'a>,
        depth: usize,
        kind: RuntimeKey,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let mode = match kind {
            RuntimeKey::Uint32 => Mode::Number,
            RuntimeKey::Scalar { .. } => Mode::General,
            RuntimeKey::String => return self.schedule_string_operand(key, depth, work),
        };
        work.push(Work::Eval(key, depth, mode));
        Ok(())
    }

    pub(in crate::lower) fn runtime_key_cursor<'e, 'a>(
        &self,
        output: Region,
        key: &'e Expression<'a>,
        depth: usize,
        kind: RuntimeKey,
    ) -> Result<(EvaluationCursor<'e, 'a>, Mode), String> {
        let mut work = Vec::new();
        self.schedule_runtime_key(key, depth, kind, &mut work)?;
        let mode = match kind {
            RuntimeKey::Uint32 => Mode::Number,
            RuntimeKey::Scalar { .. } => Mode::General,
            RuntimeKey::String => Mode::Managed,
        };
        Ok((EvaluationCursor::with_work(output, work), mode))
    }

    pub(super) fn runtime_member_receiver(
        &self,
        member: &oxc_ast::ast::ComputedMemberExpression<'_>,
        depth: usize,
        access: KeyAccess,
    ) -> Option<(Mode, RuntimeKey)> {
        if member.optional
            || closures::unwrapped(&member.expression, depth)
                .and_then(static_keys::literal)
                .is_some()
        {
            return None;
        }
        let receiver = self.receiver_type(&member.object, depth)?;
        self.owned_key_kind(&member.object, receiver, &member.expression, depth, access)
            .map(|kind| (Mode::managed(receiver), kind))
    }

    pub(in crate::lower) fn owned_key_kind(
        &self,
        object: &Expression<'_>,
        receiver: Type,
        key: &Expression<'_>,
        depth: usize,
        access: KeyAccess,
    ) -> Option<RuntimeKey> {
        self.binding_key_kind(
            receiver,
            self.stored.dictionary(object.node_id()),
            key,
            depth,
            access,
        )
    }

    pub(in crate::lower) fn binding_key_kind(
        &self,
        receiver: Type,
        dictionary: bool,
        key: &Expression<'_>,
        depth: usize,
        access: KeyAccess,
    ) -> Option<RuntimeKey> {
        let kind = self.runtime_key(key, depth)?;
        match (receiver, kind) {
            (
                Type::ManagedObject | Type::ManagedArray,
                RuntimeKey::Uint32 | RuntimeKey::Scalar { .. },
            ) => Some(kind),
            (Type::ManagedObject, RuntimeKey::String)
                if dictionary
                    || (self.stored.string(key.node_id())
                        && self.stored.record_names.contains(key, depth)) =>
            {
                Some(kind)
            }
            (Type::ManagedArray, RuntimeKey::String)
                if access == KeyAccess::Read
                    && self.stored.string(key.node_id())
                    && self
                        .stored
                        .record_names
                        .domain(key, depth)
                        .array_own_names() =>
            {
                Some(kind)
            }
            _ => None,
        }
    }

    pub(super) fn schedule_dictionary_read<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::ComputedMemberExpression(member) = node else {
            return Ok(false);
        };
        let Some((mode, kind)) = self.runtime_member_receiver(member, depth + 1, KeyAccess::Read)
        else {
            return Ok(false);
        };
        work.push(Work::Dictionary(Task::ReadReceiver {
            node,
            key: &member.expression,
            depth: depth + 1,
            kind,
        }));
        work.push(Work::Eval(&member.object, depth + 1, mode));
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
        let Some((mode, kind)) = self.runtime_member_receiver(member, depth + 1, KeyAccess::Write)
        else {
            return Ok(false);
        };
        if assignment.operator != AssignmentOperator::Assign {
            return Err(unsupported(
                node,
                "dictionary properties require plain assignment",
            ));
        }
        work.push(Work::Dictionary(Task::SetReceiver {
            node,
            key: &member.expression,
            right: &assignment.right,
            depth: depth + 1,
            kind,
        }));
        work.push(Work::Eval(&member.object, depth + 1, mode));
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
                kind,
            } => {
                let object = values.pop().expect("dictionary receiver").id();
                work.push(Work::Dictionary(Task::ReadKey { node, object, kind }));
                self.schedule_runtime_key(key, depth, kind, work)?;
            }
            Task::ReadKey { node, object, kind } => {
                let key = values.pop().expect("dictionary key").id();
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedGetOwnDynamic {
                        result,
                        object,
                        key: kind.finish(key),
                    },
                );
                values.push(Value::Managed(result));
            }
            Task::SetReceiver {
                node,
                key,
                right,
                depth,
                kind,
            } => {
                let object = values.pop().expect("dictionary assignment receiver").id();
                work.push(Work::Dictionary(Task::SetKey {
                    node,
                    object,
                    right,
                    depth,
                    kind,
                }));
                self.schedule_runtime_key(key, depth, kind, work)?;
            }
            Task::SetKey {
                node,
                object,
                right,
                depth,
                kind,
            } => {
                let key = values.pop().expect("dictionary assignment key").id();
                let key = kind.finish(key);
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
            ("const xs=[];xs[a]=b;return a;", true),
            ("const xs=[];xs[a>>0]=b;return a;", true),
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
            ("const xs=[a];return +xs[b];", true),
            ("const xs=[a];return +xs[b>>0];", true),
            ("const xs=[a];return +xs[b?(0>>>0):a];", true),
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
