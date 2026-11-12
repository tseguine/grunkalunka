//! Own-property calls and dictionary membership share a presence check after
//! evaluating their operands in the order required by the source operation.
use super::*;

pub(super) enum Membership {
    OwnLiteral(Vec<u16>),
    OwnRuntime(dictionary::RuntimeKey),
    Inherited,
    Prototype,
}

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    call: &'e CallExpression<'a>,
    arguments: invocation::Arguments<'e, 'a>,
    depth: usize,
    index: usize,
    input: Option<ValueId>,
    key: Option<ir::ManagedKey>,
    runtime_key: Option<dictionary::RuntimeKey>,
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
        let receiver = binary.right.node_id();
        let Some(ty @ (Type::ManagedObject | Type::ManagedArray)) =
            self.receiver_type(&binary.right, depth + 1)
        else {
            return Err(unsupported(
                &binary.right,
                "in requires a proven object or array",
            ));
        };
        let dictionary = self.stored.dictionary(receiver);
        let known_prototype = ty == Type::ManagedArray || self.stored.ordinary_object(receiver);
        let (membership, dynamic) = if let Some(key) = literal {
            let inherited = !dictionary
                && (crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &key)
                    || (ty == Type::ManagedArray
                        && crate::prototype_keys::contains(crate::prototype_keys::ARRAY, &key)));
            if inherited && !known_prototype {
                // The live object may be a null-prototype dictionary. Use the
                // same runtime prototype query as for a computed String name.
                (Membership::Prototype, Some(dictionary::RuntimeKey::String))
            } else if inherited {
                (Membership::Inherited, None)
            } else {
                (Membership::OwnLiteral(key), None)
            }
        } else {
            let kind = self.runtime_key(&binary.left, depth + 1).ok_or_else(|| {
                unsupported(
                    &binary.left,
                    "in requires a proven String or supported primitive key",
                )
            })?;
            // Non-String primitive names never match pristine prototype properties.
            let membership = if dictionary || !matches!(kind, dictionary::RuntimeKey::String) {
                Membership::OwnRuntime(kind)
            } else {
                Membership::Prototype
            };
            (membership, Some(kind))
        };
        work.push(Work::In { node, membership });
        work.push(Work::Eval(&binary.right, depth + 1, Mode::managed(ty)));
        if let Some(kind) = dynamic {
            self.schedule_runtime_key(&binary.left, depth + 1, kind, work)?;
        } else {
            work.push(Work::Eval(&binary.left, depth + 1, Mode::Discard));
        }
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
        let arguments = invocation::all_arguments(call);
        if arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "at most 1000 Object.hasOwn arguments are admitted",
            ));
        }
        work.push(Work::HasOwn(Task {
            node,
            call,
            arguments,
            depth,
            index: 0,
            input: None,
            key: None,
            runtime_key: None,
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
            let key = values.pop().expect("has-own key").id();
            task.key = Some(task.runtime_key.expect("dynamic has-own key").finish(key));
        }
        let Some(actual) = task.arguments.get(task.index) else {
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
        let arg = actual.as_expression().ok_or_else(|| {
            unsupported(
                task.node,
                "only fixed array-literal call spreads are admitted",
            )
        })?;
        let depth = task.depth + 1 + actual.depth;
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
            } else {
                // This intrinsic observes only own-property presence. Its
                // receiver is already checked, so a boxed primitive/String key
                // can use the general guest ToPropertyKey operation without
                // introducing prototype Get/Set semantics.
                let kind = self.runtime_key(arg, depth).or_else(|| {
                    let known_reference = matches!(
                        self.managed_source_type(arg, depth),
                        Some(Type::ManagedObject | Type::ManagedArray | Type::Closure(_))
                    ) || self.stored.container(arg.node_id()).is_some();
                    (!known_reference).then_some(dictionary::RuntimeKey::Value)
                });
                let kind = kind.ok_or_else(|| {
                    unsupported(
                        arg,
                        "Object.hasOwn needs represented primitive/String key coercion",
                    )
                })?;
                task.runtime_key = Some(kind);
                task.index += 1;
                work.push(Work::HasOwn(task));
                return self.schedule_runtime_key(arg, depth, kind, work);
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
    crate::lower::string_methods::undefined(key, semantic)
        .then(|| "undefined".encode_utf16().collect())
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
            ("const xs=[a];return a in xs?1:0;", true),
            ("const xs=[a];return (a>>0) in xs?1:0;", true),
            ("return (a>>>0) in a?1:0;", false),
            ("const xs=[a];return (a?(0>>>0):b) in xs?1:0;", true),
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
        assert!(crate::compile("export function run(a:number,b:number){const xs=[a];return ({} as number) in xs?1:0;}",true).is_err());
    }
}

#[cfg(test)]
mod uint32_has_own_tests {
    #[test]
    fn has_own_unsigned_keys_require_syntax_and_receiver_proofs() {
        for (body, admitted) in [
            ("const xs=[a,,b];return Object.hasOwn(xs,a>>>0)?1:0;", true),
            ("const o={0:a};return Object.hasOwn(o,a>>>b)?1:0;", true),
            (
                "const d=Object.create(null);return Object.hasOwn(d,a>>>0)?1:0;",
                true,
            ),
            ("return Object.hasOwn('abc',a>>>0)?1:0;", true),
            ("return Object.hasOwn(a,b>>>0)?1:0;", true),
            ("const xs=[a];return Object['hasOwn'](xs,a>>>0)?1:0;", true),
            ("const xs=[a];return Object.hasOwn(xs,a)?1:0;", true),
            ("const xs=[a];return Object.hasOwn(xs,a>>0)?1:0;", true),
            (
                "const xs=[a];return Object.hasOwn(xs,a?(0>>>0):b)?1:0;",
                true,
            ),
            ("return Object.hasOwn(null,a>>>0)?1:0;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        let js = "export function run(a,b){const xs=[a];return Object.hasOwn(xs,a>>>0)?1:0;}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[a];return Object.hasOwn(xs,(a>>>0) as never)?1:0;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }

    #[test]
    fn has_own_unsigned_keys_respect_small_stack_depth_limits() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (depth, admitted) in [(16, true), (129, false)] {
                    let source = format!(
                        "export function run(a,b){{const xs=[a];return Object.hasOwn(xs,{}a>>>0{})?1:0;}}",
                        "(".repeat(depth),
                        ")".repeat(depth)
                    );
                    let result = crate::compile(&source, false);
                    assert_eq!(result.is_ok(), admitted, "{depth}: {:?}", result.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod boxed_has_own_key_tests {
    #[test]
    fn mixed_primitive_and_string_keys_use_the_guest_normalizer() {
        for body in [
            "const o={x:b};const key=a?'x':b;return Object.hasOwn(o,key)?1:0;",
            "const xs=[a,,b];const key=a?'length':b;return Object.hasOwn(xs,key)?1:0;",
            "const d=Object.create(null);const key=a?'__proto__':b;d[key]=b;return Object.hasOwn(d,key)?1:0;",
            "const key=a?'length':b;return Object.hasOwn('💠',key)?1:0;",
            "const o={x:b};let count=0;const key=a?'x':b;return Object.hasOwn(o,key,count++)&&count===1?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn boxed_has_own_keys_still_reject_source_known_reference_coercion() {
        for source in [
            "export function run(a,b){const key={};return Object.hasOwn({x:a},key)?1:0;}",
            "export function run(a,b){return Object.hasOwn([a],{} as number)?1:0;}",
            "export function run(a,b){const key=[0];return Object.hasOwn({0:a},key)?1:0;}",
        ] {
            assert!(crate::compile(source, true).is_err(), "{source}");
        }
    }
}

#[cfg(test)]
mod fixed_has_own_spread_tests {
    #[test]
    fn fixed_literal_spreads_reuse_ordinary_argument_proofs() {
        for (body, admitted) in [
            (
                "const xs=[a];return Object.hasOwn(...[xs,0>>>0])?1:0;",
                true,
            ),
            ("const o={x:a};return Object.hasOwn(...[o,'x'])?1:0;", true),
            ("return Object.hasOwn(...[[...[a]],0>>>0])?1:0;", true),
            (
                "const xs=[a];return Object.hasOwn(...[xs,...[],0>>>0])?1:0;",
                true,
            ),
            ("return Object.hasOwn(...[...[{0:a}],0>>>0])?1:0;", true),
            (
                "const xs=[a];return Object.hasOwn(...[xs],0>>>0)?1:0;",
                true,
            ),
            (
                "const xs=[a];return Object.hasOwn(xs,...[0>>>0])?1:0;",
                true,
            ),
            ("const xs=[a];return Object.hasOwn(...xs)?1:0;", false),
            ("const xs=[a];return Object.hasOwn(...[xs,a])?1:0;", true),
            ("const xs=[a];return Object.hasOwn(...[xs,a>>0])?1:0;", true),
            (
                "const xs=[a];return Object.hasOwn(...[xs,,0>>>0])?1:0;",
                false,
            ),
            ("return Object.hasOwn(...[null,0>>>0])?1:0;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn nested_fixed_spreads_keep_the_small_native_stack_boundary() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (depth, admitted) in [(16, true), (129, false)] {
                    let spread = (0..depth).fold("[a]".to_owned(), |s, _| format!("[...{s}]"));
                    let source = format!(
                        "export function run(a,b){{return Object.hasOwn(...[{spread},0>>>0])?1:0;}}"
                    );
                    let result = crate::compile(&source, false);
                    assert_eq!(result.is_ok(), admitted, "{depth}: {:?}", result.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
