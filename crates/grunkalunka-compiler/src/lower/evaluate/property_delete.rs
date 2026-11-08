//! Delete reference operands without reading the property value.
use super::*;

pub(super) enum Task<'e, 'a> {
    Effect(&'e Expression<'a>),
    Literal {
        node: &'e Expression<'a>,
        key: Vec<u16>,
    },
    Receiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
        uint32: bool,
    },
    Key {
        node: &'e Expression<'a>,
        input: ValueId,
        uint32: bool,
    },
}

// Source operators, never annotations, prove the key's numeric range. The
// ordinary evaluator still validates every operand and effect of these forms.
pub(in crate::lower) fn uint32_key(expression: &Expression<'_>, depth: usize) -> bool {
    let mut pending = vec![(expression, depth)];
    while let Some((expression, depth)) = pending.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return false;
        };
        match expression {
            Expression::BinaryExpression(binary)
                if binary.operator == BinaryOperator::ShiftRightZeroFill => {}
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                pending.push((&assignment.right, depth + 1))
            }
            Expression::ConditionalExpression(conditional) => {
                pending.push((&conditional.consequent, depth + 1));
                pending.push((&conditional.alternate, depth + 1));
            }
            _ => return false,
        }
    }
    true
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
                (&member.object, key, Some(&member.expression))
            }
            Expression::Identifier(_)
            | Expression::ChainExpression(_)
            | Expression::PrivateFieldExpression(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_) => {
                return Err(unsupported(
                    node,
                    "delete requires an admitted member reference",
                ));
            }
            _ => {
                // Sequence, assignment and conditional expressions produce
                // values, even when their selected child was a reference.
                boolean_consumer(node, mode, "delete produces a Boolean", work)?;
                work.push(Work::PropertyDelete(Task::Effect(node)));
                work.push(Work::Eval(&unary.argument, depth + 1, Mode::Discard));
                return Ok(true);
            }
        };
        let primitive = self.numeric_result_shape(object, member_depth + 1)
            || self.operand_boolean_shape(object, member_depth + 1);
        let ty = self.receiver_type(object, member_depth + 1);
        if !primitive && !matches!(ty, Some(Type::ManagedObject | Type::ManagedArray)) {
            return Err(unsupported(
                node,
                "delete requires a proven Number, Boolean, object or array receiver",
            ));
        }
        let uint32 = dynamic.is_some_and(|key| uint32_key(key, member_depth + 1));
        if let Some(key) = dynamic
            && literal.is_none()
            && !uint32
            && !self.stored.string(key.node_id())
            && !(primitive
                && (self.numeric_result_shape(key, member_depth + 1)
                    || self.operand_boolean_shape(key, member_depth + 1)
                    || has_own::literal_property_name(key, self.semantic, member_depth + 1)
                        .is_some()))
        {
            return Err(unsupported(
                node,
                "runtime delete keys require a source-proven String",
            ));
        }
        if primitive {
            // Primitive Number/Boolean wrappers have no own properties.
            // Their admitted primitive key conversion cannot call user code.
            boolean_consumer(node, mode, "delete produces a Boolean", work)?;
            work.push(Work::PropertyDelete(Task::Effect(node)));
            if let Some(key) = dynamic {
                work.push(Work::Eval(key, member_depth + 1, Mode::Discard));
            }
            work.push(Work::Eval(object, member_depth + 1, Mode::Discard));
            return Ok(true);
        }
        let ty = ty.expect("validated managed delete receiver");
        if ty == Type::ManagedArray
            && !uint32
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
                uint32,
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
            Task::Effect(node) => {
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    Op::Boolean {
                        result,
                        value: true,
                    },
                );
                values.push(Value::Predicate(result));
                return;
            }
            Task::Literal { node, key } => (
                node,
                values.pop().expect("delete receiver").id(),
                ir::ManagedKey::Literal(key),
            ),
            Task::Receiver {
                node,
                key,
                depth,
                uint32,
            } => {
                let input = values.pop().expect("saved delete receiver").id();
                work.push(Work::PropertyDelete(Task::Key {
                    node,
                    input,
                    uint32,
                }));
                work.push(Work::Eval(
                    key,
                    depth,
                    if uint32 { Mode::Number } else { Mode::Managed },
                ));
                return;
            }
            Task::Key {
                node,
                input,
                uint32,
            } => {
                let key = values.pop().expect("delete key").id();
                (
                    node,
                    input,
                    if uint32 {
                        ir::ManagedKey::Uint32(key)
                    } else {
                        ir::ManagedKey::String(key)
                    },
                )
            }
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
    fn uint32_delete_keys_use_source_operators_and_retain_mutation_guards() {
        for (body, admitted) in [
            ("const xs=[a,b];delete xs[a>>>0];return xs.length;", true),
            (
                "const o={0:a,1:b};delete o[a>>>b];return Object.keys(o).length;",
                true,
            ),
            (
                "const xs=[a,b];delete xs[(b=1,a>>>0)];return xs.length;",
                true,
            ),
            (
                "const xs=[a,b];delete xs[a?(0>>>0):(1>>>0)];return xs.length;",
                true,
            ),
            (
                "const xs=[a,b];let k=0;delete xs[k=a>>>0];return xs.length;",
                true,
            ),
            ("const xs=[a];delete xs[a];return a;", false),
            ("const xs=[a];delete xs[a>>0];return a;", false),
            ("const xs=[a];delete xs[a?(0>>>0):b];return a;", false),
            ("const xs=[a];delete xs.length;return a;", false),
            (
                "const xs=['x'];const alias=xs;delete alias[a>>>0];return xs[0].trim().length;",
                false,
            ),
            (
                "const xs=[a];delete xs[(Math.random(),a>>>0)];return a;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        let js = "export function run(a,b){const xs=[a];delete xs[a>>>0];return xs.length;}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[a];delete xs[((a>>>0) as never)];return xs.length;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
        assert!(crate::compile("export function run(a:number,b:number){const xs=[a];delete xs[a as number];return a;}",true).is_err());
    }

    #[test]
    fn uint32_delete_proofs_use_bounded_heap_traversal() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(||{
            for depth in [16,129] {
                let key=format!("{}a>>>0{}","(".repeat(depth),")".repeat(depth));
                let source=format!("export function run(a,b){{const xs=[a];delete xs[{key}];return xs.length;}}");
                let result=crate::compile(&source,false);
                assert_eq!(result.is_ok(),depth==16,"{depth}: {:?}",result.err());
            }
        }).unwrap().join().unwrap();
    }
    #[test]
    fn primitive_and_nonreference_delete_keep_effects_and_nullish_guards() {
        for (body, admitted) in [
            ("return delete a.x?1:0;", true),
            ("return delete (a<b).length?1:0;", true),
            ("return delete a[b]?1:0;", true),
            ("return delete a[null]?1:0;", true),
            ("const o={x:a};return delete (0,o.x)&&o.x===a?1:0;", true),
            ("return delete (a=b)?a:0;", true),
            ("return delete null.x?1:0;", false),
            ("return delete (void 0).x?1:0;", false),
            ("return delete a[{}]?1:0;", false),
            ("const o={x:a};return delete o?.x?1:0;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }

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
            ("delete a.x;return a;", true),
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
