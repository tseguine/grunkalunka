//! Delete reference operands without reading the property value.
use super::*;

pub(super) enum Task<'e, 'a> {
    Optional(super::optional_delete::Task<'e, 'a>),
    Effect(&'e Expression<'a>),
    Literal {
        node: &'e Expression<'a>,
        key: Vec<u16>,
    },
    Receiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
        kind: dictionary::RuntimeKey,
    },
    Key {
        node: &'e Expression<'a>,
        input: ValueId,
        kind: dictionary::RuntimeKey,
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
            Expression::ChainExpression(chain) => {
                return self.schedule_optional_delete(node, chain, member_depth, mode, work);
            }
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
        let kind = dynamic.and_then(|key| {
            self.runtime_key(key, member_depth + 1).or_else(|| {
                // Own data fields on both ordinary objects and dictionaries
                // are configurable. Delete never consults inherited names.
                // Array length still has its separate error contract.
                let data_object = ty == Some(Type::ManagedObject);
                let known_reference = self
                    .stored
                    .property_key_requires_to_primitive(key.node_id())
                    || matches!(
                        self.managed_source_type(key, member_depth + 1),
                        Some(Type::ManagedObject | Type::ManagedArray | Type::Closure(_))
                    )
                    || self.stored.container(key.node_id()).is_some();
                (data_object && !known_reference).then_some(dictionary::RuntimeKey::Value)
            })
        });
        if let Some(key) = dynamic
            && literal.is_none()
            && kind.is_none()
            && !(primitive
                && (self.numeric_result_shape(key, member_depth + 1)
                    || self.operand_boolean_shape(key, member_depth + 1)
                    || has_own::literal_property_name(key, self.semantic, member_depth + 1)
                        .is_some()))
        {
            return Err(unsupported(
                node,
                "runtime delete keys require a represented primitive",
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
            && !matches!(
                kind,
                Some(dictionary::RuntimeKey::Uint32 | dictionary::RuntimeKey::Scalar { .. })
            )
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
                kind: kind.expect("validated primitive key"),
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
    ) -> Result<(), String> {
        let (node, input, key) = match task {
            Task::Optional(task) => {
                return self.resume_optional_delete(task, regions, values, work);
            }
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
                return Ok(());
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
                kind,
            } => {
                let input = values.pop().expect("saved delete receiver").id();
                work.push(Work::PropertyDelete(Task::Key { node, input, kind }));
                self.schedule_runtime_key(key, depth, kind, work)?;
                return Ok(());
            }
            Task::Key { node, input, kind } => {
                let key = values.pop().expect("delete key").id();
                (node, input, kind.finish(key))
            }
        };
        let result = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            Op::ManagedDeleteOwn { result, input, key },
        );
        values.push(Value::Predicate(result));
        Ok(())
    }
}

#[cfg(test)]
mod boxed_dictionary_delete_tests {
    #[test]
    fn boxed_primitive_string_deletion_uses_guest_own_property_keys() {
        for body in [
            "const d=Object.create(null);const k=a?'x':b;d[k]=b;delete d[k];return Object.keys(d).length;",
            "const d=Object.create(null);d.x=b;const k=a?'x':b;delete d[k];return Object.hasOwn(d,'x')?1:0;",
            "const d=Object.create(null);d.true=b;const k=a?true:'true';delete d[k];return Object.keys(d).length;",
            "const d=Object.create(null);d.null=b;const k=a?null:'null';delete d[k];return Object.keys(d).length;",
            "const d=Object.create(null);d['__proto__']=b;const k=a?'__proto__':b;delete d[k];return Object.keys(d).length;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let result = crate::compile(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn boxed_keys_delete_configurable_own_data_on_ordinary_objects() {
        for body in [
            "const o={x:b};const k=a?'x':b;return delete o[k]&&!Object.hasOwn(o,k)?1:0;",
            "const o={x:b};const k=a?'constructor':b;return delete o[k]&&('constructor' in o)?1:0;",
            "const o={x:b};const k=a?'__proto__':b;return delete o[k]&&(o instanceof Object)?1:0;",
            "const o={x:b};const k=a?null:'null';o.null=b;delete o[k];return Object.keys(o).length;",
            "const o={x:b};const k=a?true:'true';o.true=b;delete o[k];return Object.keys(o).length;",
            "const o={x:b};const k=a?'x':b;const alias=o;delete o[k];return Object.hasOwn(alias,'x')===(!a)?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let result = crate::compile(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn boxed_delete_remains_guarded_on_reference_keys_and_array_length() {
        for source in [
            "export function run(a,b){const d=Object.create(null);const k={};delete d[k];return a;}",
            "export function run(a,b){const xs=[a];const k=a?'length':b;delete xs[k];return a;}",
            "export function run(a,b){const o={x:a};const k={};delete o[k];return a;}",
        ] {
            assert!(crate::compile(source, false).is_err(), "{source}");
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn masked_index_requires_a_clear_sign_bit_and_valid_operands() {
        for (key, admitted) in [
            ("a & 0x7fffffff", true),
            ("0 & (a=b)", true),
            ("a & (7 as never)", true),
            ("a & 0x80000000", false),
            ("-1 & a", false),
            ("a & b", false),
            ("(a | 7)", false),
            ("Math.random() & 0", false),
        ] {
            let source = format!(
                "export function run(a,b){{const xs=[a];return Object.hasOwn(xs,{key})?1:0;}}"
            );
            assert_eq!(
                crate::lower::uint32_keys::has_unsigned_presence(&source, true),
                admitted,
                "{source}"
            );
        }
    }

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
            ("const xs=[a];delete xs[a];return a;", true),
            ("const xs=[a];delete xs[a>>0];return a;", true),
            ("const xs=[a];delete xs[a?(0>>>0):b];return a;", true),
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
        assert!(crate::compile("export function run(a:number,b:number){const xs=[a];delete xs[{} as unknown as number];return a;}",true).is_err());
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
            ("const o={x:a};return delete o?.x?1:0;", true),
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
                true,
            ),
            ("const xs=[a],key='0';delete xs[key];return a;", false),
            ("const o={x:a};delete o[a];return a;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        assert!(crate::compile("export function run(a:number,b:number){const o={x:a};delete o[{} as unknown as string];return a;}", true).is_err());
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
