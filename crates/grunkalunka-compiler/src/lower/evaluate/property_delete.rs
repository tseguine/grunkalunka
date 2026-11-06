//! Delete reference operands without reading the property value.
use super::*;

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    key: Vec<u16>,
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
        let (object, key) = match argument {
            Expression::StaticMemberExpression(member) if !member.optional => (
                &member.object,
                member.property.name.encode_utf16().collect(),
            ),
            Expression::ComputedMemberExpression(member) if !member.optional => {
                let key = closures::unwrapped(&member.expression, member_depth + 1)
                    .and_then(static_keys::literal)
                    .ok_or_else(|| {
                        unsupported(node, "delete keys require a String or numeric literal")
                    })?;
                (&member.object, key)
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
        if ty == Type::ManagedArray && key.as_slice() == [108, 101, 110, 103, 116, 104] {
            return Err(unsupported(
                node,
                "deleting non-configurable array length requires a source TypeError",
            ));
        }
        boolean_consumer(node, mode, "delete produces a Boolean", work)?;
        work.push(Work::PropertyDelete(Task { node, key }));
        work.push(Work::Eval(object, member_depth + 1, Mode::managed(ty)));
        Ok(true)
    }

    pub(super) fn finish_property_delete(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let input = values.pop().expect("delete receiver").id();
        let result = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedDeleteOwn {
                result,
                input,
                key: ir::ManagedKey::Literal(task.key),
            },
        );
        values.push(Value::Predicate(result));
    }
}

#[cfg(test)]
mod tests {
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
            ("const o={x:a},key='x';delete o[key];return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
