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
    Value,
}

impl RuntimeKey {
    pub(in crate::lower) fn finish(self, input: ValueId) -> ir::ManagedKey {
        match self {
            Self::Uint32 => ir::ManagedKey::Uint32(input),
            Self::Scalar { numbers } => ir::ManagedKey::Scalar { input, numbers },
            Self::String => ir::ManagedKey::String(input),
            Self::Value => ir::ManagedKey::Value(input),
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
    // The provenance of a wrapper or a compound expression is not always
    // attached to its outer NodeId. Check every value-producing alternative
    // before falling back to primitive-only guest ToPropertyKey. Evaluation
    // of conditions and sequence prefixes has effects but does not supply
    // the property key. Keep this traversal iterative.
    pub(in crate::lower) fn boxed_key_needs_to_primitive(
        &self,
        key: &Expression<'_>,
        depth: usize,
    ) -> bool {
        let mut pending = vec![(key, depth)];
        while let Some((key, depth)) = pending.pop() {
            let Some((key, depth)) = closures::unwrapped_at(key, depth) else {
                return true;
            };
            if self
                .stored
                .property_key_requires_to_primitive(key.node_id())
                || matches!(
                    self.managed_source_type(key, depth),
                    Some(Type::ManagedObject | Type::ManagedArray | Type::Closure(_))
                )
                || self.stored.container(key.node_id()).is_some()
            {
                return true;
            }
            match key {
                Expression::ObjectExpression(_)
                | Expression::ArrayExpression(_)
                | Expression::ArrowFunctionExpression(_)
                | Expression::FunctionExpression(_) => return true,
                Expression::ConditionalExpression(conditional) => {
                    pending.push((&conditional.consequent, depth + 1));
                    pending.push((&conditional.alternate, depth + 1));
                }
                Expression::LogicalExpression(logical) => {
                    pending.push((&logical.left, depth + 1));
                    pending.push((&logical.right, depth + 1));
                }
                Expression::SequenceExpression(sequence) => {
                    if let Some(value) = sequence.expressions.last() {
                        pending.push((value, depth + 1));
                    }
                }
                Expression::AssignmentExpression(assignment)
                    if assignment.operator == AssignmentOperator::Assign =>
                {
                    pending.push((&assignment.right, depth + 1));
                }
                _ => {}
            }
        }
        false
    }

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
            RuntimeKey::Value => Mode::Managed,
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
            RuntimeKey::String | RuntimeKey::Value => Mode::Managed,
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

    // Shared by ordinary computed Get/Set and logical dictionary assignments.
    // Array length writes and prototype-bearing ordinary Get/Set still need
    // guest-visible completion and prototype semantics.
    pub(in crate::lower) fn runtime_member_receiver_with_fallback(
        &self,
        member: &oxc_ast::ast::ComputedMemberExpression<'_>,
        depth: usize,
        access: KeyAccess,
    ) -> Option<(Mode, RuntimeKey)> {
        if let Some(known) = self.runtime_member_receiver(member, depth, access) {
            return Some(known);
        }
        if member.optional
            || closures::unwrapped(&member.expression, depth)
                .and_then(static_keys::literal)
                .is_some()
        {
            return None;
        }
        let receiver = self.receiver_type(&member.object, depth)?;
        let dictionary = self.stored.dictionary(member.object.node_id());
        let kind = self.boxed_runtime_key_fallback(
            &member.expression,
            depth,
            receiver,
            dictionary,
            access,
        )?;
        Some((Mode::managed(receiver), kind))
    }

    // Runtime key normalization is shared by direct and optional reads.
    // The receiver and access checks keep Array length writes and ordinary
    // prototype-bearing Get/Set outside this partial own-data operation.
    pub(in crate::lower) fn boxed_runtime_key_fallback(
        &self,
        key: &Expression<'_>,
        depth: usize,
        receiver: Type,
        dictionary: bool,
        access: KeyAccess,
    ) -> Option<RuntimeKey> {
        let supported_receiver = (receiver == Type::ManagedObject && dictionary)
            || (receiver == Type::ManagedArray && access == KeyAccess::Read);
        // Known reference keys need user-visible ToPrimitive completions,
        // not an internal trap or a TypeScript annotation-based coercion.
        let known_reference = self.boxed_key_needs_to_primitive(key, depth);
        (supported_receiver && !known_reference).then_some(RuntimeKey::Value)
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
        let Some((mode, kind)) =
            self.runtime_member_receiver_with_fallback(member, depth + 1, KeyAccess::Read)
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
        let Some((mode, kind)) =
            self.runtime_member_receiver_with_fallback(member, depth + 1, KeyAccess::Write)
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

#[cfg(test)]
mod boxed_reference_key_guard_tests {
    #[test]
    fn boxed_key_fallback_rejects_reference_alternatives_in_compound_expressions() {
        for body in [
            "return (a?{}:b) in Object.create(null)?1:0;",
            "return (a?b:[]) in {}?1:0;",
            "return Object.hasOwn({x:a},(a?{}:b))?1:0;",
            "return Object.hasOwn([a],(a?[0]:b))?1:0;",
            "const d=Object.create(null);return d[(a?{}:b)]===undefined?1:0;",
            "const d=Object.create(null);return delete d[(a?[]:b)]?1:0;",
            "const d=Object.create(null);const {[a?{}:b]:v}=d;return a;",
            "const d=Object.create(null);return (a?(b,{}):b) in d?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let error = crate::compile(&source, typed)
                    .err()
                    .unwrap_or_else(|| panic!("admitted reference-valued key: {source}"));
                assert!(error.contains("unsupported"), "{source}: {error}");
            }
        }
        let ts = "export function run(a:number,b:number){const d=Object.create(null);return (a?({} as never):b) in d?1:0;}";
        assert!(crate::compile(ts, true).is_err(), "{ts}");
    }
}
