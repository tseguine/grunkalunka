//! Delete reference operands without reading the property value.
use super::*;

pub(super) enum Task<'e, 'a> {
    Literal {
        node: &'e Expression<'a>,
        key: Vec<u16>,
    },
    Receiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
    },
    Key {
        node: &'e Expression<'a>,
        input: ValueId,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_property_delete<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::UnaryExpression(unary) = node else {
            return Ok(false);
        };
        if unary.operator != UnaryOperator::Delete {
            return Ok(false);
        }
        let Some((argument, member_depth)) = closures::unwrapped_at(&unary.argument, depth + 1)
        else {
            return Err(unsupported(node, "delete operand nesting exceeds 128"));
        };
        let (object, literal, dynamic) = match argument {
            Expression::StaticMemberExpression(member) if !member.optional => (
                &member.object,
                Some(member.property.name.encode_utf16().collect()),
                None,
            ),
            Expression::ComputedMemberExpression(member) if !member.optional => {
                let key = closures::unwrapped(&member.expression, member_depth + 1)
                    .and_then(static_keys::literal);
                if key.is_none() && !self.stored.string(member.expression.node_id()) {
                    return Err(unsupported(
                        node,
                        "runtime delete keys require a source-proven String",
                    ));
                }
                (&member.object, key, Some(&member.expression))
            }
            _ => {
                return Err(unsupported(
                    node,
                    "delete requires an admitted member reference",
                ));
            }
        };
        let ty = self
            .receiver_type(object, member_depth + 1)
            .filter(|ty| matches!(ty, Type::ManagedObject | Type::ManagedArray))
            .ok_or_else(|| unsupported(node, "delete requires a source-proven object or array"))?;
        if ty == Type::ManagedArray
            && !literal
                .as_ref()
                .is_some_and(|key| key.as_slice() != [108, 101, 110, 103, 116, 104])
        {
            return Err(unsupported(
                node,
                "deleting non-configurable array length requires a source TypeError",
            ));
        }
        boolean_consumer(node, mode, "delete produces a Boolean", work)?;
        work.push(Work::PropertyDelete(if let Some(key) = literal {
            Task::Literal { node, key }
        } else {
            Task::Receiver {
                node,
                key: dynamic.expect("computed runtime key"),
                depth: member_depth + 1,
            }
        }));
        work.push(Work::Eval(object, member_depth + 1, Mode::managed(ty)));
        Ok(true)
    }

    pub(super) fn resume_property_delete<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) {
        let (node, input, key) = match task {
            Task::Literal { node, key } => (
                node,
                values.pop().expect("delete receiver").id(),
                ir::ManagedKey::Literal(key),
            ),
            Task::Receiver { node, key, depth } => {
                let input = values.pop().expect("saved delete receiver").id();
                work.push(Work::PropertyDelete(Task::Key { node, input }));
                work.push(Work::Eval(key, depth, Mode::Managed));
                return;
            }
            Task::Key { node, input } => (
                node,
                input,
                ir::ManagedKey::String(values.pop().expect("delete key").id()),
            ),
        };
        let result = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            Op::ManagedDeleteOwn { result, input, key },
        );
        values.push(Value::Predicate(result));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_delete_keys_keep_string_and_alias_proofs() {
        for (body, admitted) in [
            (
                "const o={x:a},key='x';delete o[key];return o.x===undefined?1:0;",
                true,
            ),
            (
                "const o={x:'text'},alias=o,key='x';delete alias[key];return o.x.length;",
                false,
            ),
            (
                "const o={x:'text'},key='x';delete o[key];const {x='fallback'}=o;return x.length;",
                true,
            ),
            (
                "const o={x:a};let key='x';key=a;delete o[key];return a;",
                false,
            ),
            ("const xs=[a],key='0';delete xs[key];return a;", false),
            ("const o={x:a};delete o[a];return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        assert!(crate::compile("export function run(a:number,b:number){const o={x:a};delete o[a as string];return a;}", true).is_err());
    }

    #[test]
    fn own_property_deletion_retains_receiver_and_provenance_guards() {
        for (body, admitted) in [
            ("const o={x:a};delete o.x;return o.x===undefined?1:0;", true),
            ("const o=[a,b];delete o[0];return o.length;", true),
            (
                "const o={x:'text'};const alias=o;delete alias.x;const {x='fallback'}=o;return x.length;",
                true,
            ),
            ("const o={x:'text'};delete o.x;return o.x.length;", false),
            ("const o={x:[]};delete o.x;return o.x.length;", false),
            ("const o={x:()=>a};delete o.x;return o.x();", false),
            ("const o=[a];delete o.length;return a;", false),
            ("delete a.x;return a;", false),
            ("delete null.x;return a;", false),
            ("delete 'abc'[0];return a;", false),
            ("const o={x:a},key='x';delete o[key];return a;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
