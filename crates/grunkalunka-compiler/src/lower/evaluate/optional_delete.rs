//! Delete the original optional reference without first reading its value.
use super::optional_member::{Key, parts};
use super::*;
use oxc_ast::ast::ChainExpression;

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        key: Key<'e, 'a>,
        receiver: Option<Type>,
        depth: usize,
    },
    Key {
        node: &'e Expression<'a>,
        input: Option<ValueId>,
        uint32: bool,
        condition: ValueId,
        skipped: Region,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_optional_delete<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        chain: &'e ChainExpression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Some((object, key)) = parts(chain, depth, &self.stored.uint32_keys) else {
            return Err(unsupported(
                node,
                "optional delete requires a single optional member reference",
            ));
        };
        let receiver = self.stored.optional_receiver(object.node_id());
        if receiver == Some(Type::ManagedValue)
            || (receiver.is_none() && !self.stored.nullish(object.node_id()))
        {
            return Err(unsupported(
                node,
                "optional delete requires a proven object, array or nullish receiver",
            ));
        }
        if let Key::Runtime(key) = &key
            && (!self.stored.string(key.node_id()) || receiver == Some(Type::ManagedArray))
        {
            return Err(unsupported(
                node,
                "optional delete runtime keys require a proven String on objects",
            ));
        }
        if receiver == Some(Type::ManagedArray)
            && let Key::Literal(key) = &key
            && key.as_slice() == [108, 101, 110, 103, 116, 104]
        {
            return Err(unsupported(
                node,
                "deleting non-configurable array length requires a source TypeError",
            ));
        }
        boolean_consumer(node, mode, "delete produces a Boolean", work)?;
        work.push(Work::PropertyDelete(property_delete::Task::Optional(
            Task::Receiver {
                node,
                key,
                receiver,
                depth,
            },
        )));
        work.push(Work::Eval(object, depth + 2, Mode::Managed));
        Ok(true)
    }

    pub(super) fn resume_optional_delete<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) {
        let (node, key, receiver, depth) = match task {
            Task::Receiver {
                node,
                key,
                receiver,
                depth,
            } => (node, key, receiver, depth),
            Task::Key {
                node,
                input,
                uint32,
                condition,
                skipped,
            } => {
                let key = values.pop().expect("optional delete key").id();
                let key = if uint32 {
                    ir::ManagedKey::Uint32(key)
                } else {
                    ir::ManagedKey::String(key)
                };
                let result = self.optional_delete_result(node, input, key, regions.current());
                let selected = regions.finish(result);
                self.finish_optional_delete(node, condition, skipped, selected, regions, values);
                return;
            }
        };
        let input = values.pop().expect("optional delete receiver").id();
        let mut skipped = self.region(node);
        let yes =
            self.optional_delete_result(node, None, ir::ManagedKey::Literal(vec![]), &mut skipped);
        skipped.exit = Exit::Yield(yes);
        let condition = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            Op::ManagedIsNullish {
                result: condition,
                input,
            },
        );
        let mut selected = self.region(node);
        let input = receiver.map(|ty| {
            let result = self.value(ty);
            Self::push(
                &mut selected,
                node,
                if ty == Type::ManagedArray {
                    Op::ManagedToArray { result, input }
                } else {
                    Op::ManagedToObject { result, input }
                },
            );
            result
        });
        match key {
            Key::Literal(key) => {
                let result = self.optional_delete_result(
                    node,
                    input,
                    ir::ManagedKey::Literal(key),
                    &mut selected,
                );
                selected.exit = Exit::Yield(result);
                self.finish_optional_delete(node, condition, skipped, selected, regions, values);
            }
            Key::Uint32(key) | Key::Runtime(key) => {
                let uint32 = self.stored.uint32_keys.contains(key, depth + 2);
                regions.children.push(selected);
                work.push(Work::PropertyDelete(property_delete::Task::Optional(
                    Task::Key {
                        node,
                        input,
                        uint32,
                        condition,
                        skipped,
                    },
                )));
                // Even a key in an unreachable branch is source-validated.
                work.push(Work::Eval(
                    key,
                    depth + 2,
                    if uint32 { Mode::Number } else { Mode::Managed },
                ));
            }
        }
    }

    fn optional_delete_result(
        &mut self,
        node: &Expression<'_>,
        input: Option<ValueId>,
        key: ir::ManagedKey,
        region: &mut Region,
    ) -> ValueId {
        let result = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            match input {
                Some(input) => Op::ManagedDeleteOwn { result, input, key },
                None => Op::Boolean {
                    result,
                    value: true,
                },
            },
        );
        result
    }

    fn finish_optional_delete(
        &mut self,
        node: &Expression<'_>,
        condition: ValueId,
        skipped: Region,
        selected: Region,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let result = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            Op::If {
                result: Some(result),
                condition,
                consequent: skipped,
                alternate: selected,
            },
        );
        values.push(Value::Predicate(result));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn optional_delete_requires_proven_receivers_keys_and_configurable_array_names() {
        for (body, admitted) in [
            ("const o=a?{x:b}:null;return delete o?.x?1:0;", true),
            (
                "const xs=a?[b]:undefined;return delete xs?.[a>>>0]?1:0;",
                true,
            ),
            (
                "const o=a?{x:b}:null;const k='x';return delete o?.[k]?1:0;",
                true,
            ),
            ("const k='x';return delete null?.[k]?1:0;", true),
            ("return delete undefined?.length?1:0;", true),
            ("return delete a?.x?1:0;", false),
            ("return delete 'abc'?.[0]?1:0;", false),
            ("const xs=[a];return delete xs?.length?1:0;", false),
            (
                "const xs=a?[b]:null;const k='0';return delete xs?.[k]?1:0;",
                false,
            ),
            ("const o={0:b};return delete o?.[a]?1:0;", false),
            ("return delete null?.[{}]?1:0;", false),
            ("return delete null?.[(1n,'x')]?1:0;", false),
            ("const o={x:{y:a}};return delete o?.x.y?1:0;", false),
            ("const f=()=>a;return delete f?.()?1:0;", false),
            (
                "const o={x:'safe'},alias=o;delete alias?.x;return o.x.length;",
                false,
            ),
            (
                "const xs=['safe'];delete xs?.[a>>>0];return xs[0].length;",
                false,
            ),
            (
                "const o={x:'safe'};const remove=t=>delete t?.x;remove(o);return o.x.length;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){return delete (a as object)?.x?1:0;}",
                true
            )
            .is_err()
        );
    }

    #[test]
    fn optional_delete_charges_key_depth_on_a_small_native_stack() {
        std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
            for (wraps, admitted) in [(125, true), (126, false)] {
                let source = format!("export function run(a,b){{const o={{x:a}},k='x';return delete o?.[{}k{}];}}", "(".repeat(wraps), ")".repeat(wraps));
                for typed in [false, true] {
                    let r=crate::compile_value(&source, typed);
                    assert_eq!(r.is_ok(), admitted, "{wraps}: {:?}", r.err());
                }
            }
        }).unwrap().join().unwrap();
    }
}
