//! Original-source optional own-property reads, using the ordinary owned cursor.
use super::managed::ManagedWork;
use super::*;
use oxc_ast::ast::{ChainExpression, MemberExpression};

pub(in crate::lower) enum Key<'e, 'a> {
    Literal(Vec<u16>),
    Uint32(&'e Expression<'a>),
    Runtime(&'e Expression<'a>),
}

pub(in crate::lower) fn parts<'e, 'a>(
    chain: &'e ChainExpression<'a>,
    depth: usize,
    uint32_keys: &Uint32Keys,
) -> Option<(&'e Expression<'a>, Key<'e, 'a>)> {
    let member = chain.expression.as_member_expression()?;
    member_parts(member, depth, uint32_keys, false)
}

pub(in crate::lower) fn computed_key<'e, 'a>(
    expression: &'e Expression<'a>,
    depth: usize,
    uint32_keys: &Uint32Keys,
) -> Option<Key<'e, 'a>> {
    let (key, _) = closures::unwrapped_at(expression, depth)?;
    Some(if let Some(key) = static_keys::literal(key) {
        Key::Literal(key)
    } else if uint32_keys.contains(expression, depth) {
        Key::Uint32(expression)
    } else {
        Key::Runtime(expression)
    })
}

fn member_parts<'e, 'a>(
    member: &'e MemberExpression<'a>,
    depth: usize,
    uint32_keys: &Uint32Keys,
    required_continuation: bool,
) -> Option<(&'e Expression<'a>, Key<'e, 'a>)> {
    if depth + 2 > 128 || !(member.optional() || required_continuation) {
        return None;
    }
    match member {
        MemberExpression::StaticMemberExpression(m) => Some((
            &m.object,
            Key::Literal(m.property.name.encode_utf16().collect()),
        )),
        MemberExpression::ComputedMemberExpression(m) => Some((
            &m.object,
            computed_key(&m.expression, depth + 2, uint32_keys)?,
        )),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub(super) enum Computed {
    StringIndex,
    StringKey,
    Own(dictionary::RuntimeKey),
}

pub(super) enum Read<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        key: Key<'e, 'a>,
        receiver: Option<Type>,
        computed: Option<Computed>,
        depth: usize,
    },
    Key {
        node: &'e Expression<'a>,
        object: Option<ValueId>,
        computed: Computed,
        condition: ValueId,
        skipped: Region,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn optional_read_parts<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
    ) -> Option<(&'e Expression<'a>, Key<'e, 'a>)> {
        let member = match node {
            Expression::ChainExpression(chain) => chain.expression.as_member_expression(),
            _ => node.as_member_expression(),
        };
        member.and_then(|m| {
            member_parts(
                m,
                depth,
                &self.stored.uint32_keys,
                self.stored.required_chain_reads.contains(&m.node_id()),
            )
        })
    }

    pub(super) fn schedule_optional_member<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Some((object, key)) = self.optional_read_parts(node, depth) else {
            return Ok(false);
        };
        let receiver = self.stored.optional_receiver(object.node_id());
        if receiver.is_none() && !self.stored.nullish(object.node_id()) {
            return Ok(false);
        }
        let array = receiver == Some(Type::ManagedArray);
        let string = receiver == Some(Type::ManagedValue);
        let dictionary = self.stored.optional_dictionary(object.node_id());
        let computed = match &key {
            Key::Literal(_) => None,
            Key::Uint32(_) if string => Some(Computed::StringIndex),
            Key::Uint32(_) => Some(Computed::Own(dictionary::RuntimeKey::Uint32)),
            Key::Runtime(expr) if string && self.stored.number(expr.node_id()) => {
                Some(Computed::StringIndex)
            }
            Key::Runtime(expr) if string && self.string_own_key(expr, depth + 2) => {
                Some(Computed::StringKey)
            }
            Key::Runtime(expr)
                if string
                    && self.stored.primitive(expr.node_id())
                    && !self.boxed_key_needs_to_primitive(expr, depth + 2) =>
            {
                // Reuse the guest String key normalizer for mixed
                // non-String primitives, but not inherited String names.
                Some(Computed::StringKey)
            }
            Key::Runtime(expr) if !string => {
                let kind = match receiver {
                    Some(ty) => {
                        let known =
                            self.binding_key_kind(ty, dictionary, expr, depth + 2, KeyAccess::Read);
                        known.or_else(|| {
                            self.boxed_runtime_key_fallback(
                                expr,
                                depth + 2,
                                ty,
                                dictionary,
                                KeyAccess::Read,
                            )
                        })
                    }
                    None => self.runtime_key(expr, depth + 2),
                };
                let Some(kind) = kind else { return Ok(false) };
                Some(Computed::Own(kind))
            }
            _ => return Ok(false),
        };
        if string
            && let Key::Literal(key) = &key
            && key.as_slice() != [108, 101, 110, 103, 116, 104]
            && binding_patterns::string_index(key).is_none()
        {
            return Err(unsupported(
                node,
                "optional String reads require length or a canonical index",
            ));
        }
        let key = match key {
            Key::Literal(key)
                if receiver.is_some()
                    && !string
                    && !(array && key.as_slice() == [108, 101, 110, 103, 116, 104]) =>
            {
                Key::Literal(managed_member_key(node, key, array, dictionary)?)
            }
            key => key,
        };
        match mode {
            Mode::Managed => (),
            Mode::Discard => work.push(Work::Forget),
            Mode::Predicate => work.push(Work::Managed(ManagedWork::Truthy(node))),
            Mode::Number if self.stored.number(node.node_id()) => {
                work.push(Work::Managed(ManagedWork::NumberProjection(node)));
            }
            Mode::General if self.stored.primitive(node.node_id()) => {
                work.push(Work::Managed(ManagedWork::PrimitiveProjection(node)));
            }
            Mode::Object | Mode::Array => {
                let ty = if matches!(mode, Mode::Array) {
                    Type::ManagedArray
                } else {
                    Type::ManagedObject
                };
                if self.stored.container(node.node_id()) != Some(ty) {
                    return Ok(false);
                }
                work.push(Work::Managed(ManagedWork::ReferenceProjection { node, ty }));
            }
            _ => return Ok(false),
        }
        work.push(Work::Managed(ManagedWork::OptionalRead(Read::Receiver {
            node,
            key,
            receiver,
            computed,
            depth,
        })));
        // The original receiver executes once, before either branch. No AST
        // replacement or synthetic member expression changes the source route.
        work.push(Work::Eval(object, depth + 2, Mode::Managed));
        Ok(true)
    }

    pub(super) fn complete_optional_member<'e, 'a>(
        &mut self,
        task: Read<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let (node, key, receiver, computed, depth) = match task {
            Read::Receiver {
                node,
                key,
                receiver,
                computed,
                depth,
            } => (node, key, receiver, computed, depth),
            Read::Key {
                node,
                object,
                computed,
                condition,
                skipped,
            } => {
                let key = values.pop().expect("optional computed key");
                let key = key.id();
                let result = if let Some(object) = object {
                    let result = self.value(Type::ManagedValue);
                    Self::push(
                        regions.current(),
                        node,
                        match computed {
                            Computed::StringIndex => Op::ManagedStringGetIndex {
                                result,
                                input: object,
                                index: key,
                            },
                            Computed::StringKey => Op::ManagedStringGetKey {
                                result,
                                input: object,
                                key,
                            },
                            Computed::Own(kind) => Op::ManagedGetOwnDynamic {
                                result,
                                object,
                                key: kind.finish(key),
                            },
                        },
                    );
                    result
                } else {
                    // Validate even the unreachable source key while preserving
                    // runtime short-circuiting for a nullish-only receiver.
                    self.optional_undefined(node, regions.current())
                };
                let selected = regions.finish(result);
                self.finish_optional_read(node, condition, skipped, selected, regions, values);
                return Ok(());
            }
        };
        let input = values.pop().expect("optional member receiver").id();
        if receiver.is_none() && matches!(key, Key::Literal(_)) {
            let result = self.optional_undefined(node, regions.current());
            values.push(Value::Managed(result));
            return Ok(());
        }
        let mut skipped = self.region(node);
        let undefined = self.optional_undefined(node, &mut skipped);
        skipped.exit = Exit::Yield(undefined);
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
        let object = receiver.map(|ty| {
            if ty == Type::ManagedValue {
                return input;
            }
            let object = self.value(ty);
            Self::push(
                &mut selected,
                node,
                if ty == Type::ManagedArray {
                    Op::ManagedToArray {
                        result: object,
                        input,
                    }
                } else {
                    Op::ManagedToObject {
                        result: object,
                        input,
                    }
                },
            );
            object
        });
        match key {
            Key::Uint32(key) | Key::Runtime(key) => {
                let computed = computed.expect("validated computed key");
                regions.children.push(selected);
                work.push(Work::Managed(ManagedWork::OptionalRead(Read::Key {
                    node,
                    object,
                    computed,
                    condition,
                    skipped,
                })));
                match computed {
                    Computed::StringKey => work.push(Work::Eval(key, depth + 2, Mode::Managed)),
                    Computed::StringIndex => work.push(Work::Eval(key, depth + 2, Mode::Number)),
                    Computed::Own(kind) => self.schedule_runtime_key(key, depth + 2, kind, work)?,
                }
            }
            Key::Literal(key) => {
                let result = self.optional_literal_get(
                    node,
                    key,
                    object.expect("non-nullish literal receiver"),
                    &mut selected,
                );
                selected.exit = Exit::Yield(result);
                self.finish_optional_read(node, condition, skipped, selected, regions, values);
            }
        }
        Ok(())
    }

    fn finish_optional_read(
        &mut self,
        node: &Expression<'_>,
        condition: ValueId,
        skipped: Region,
        selected: Region,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let result = self.value(Type::ManagedValue);
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
        values.push(Value::Managed(result));
    }

    fn optional_literal_get(
        &mut self,
        node: &Expression<'_>,
        key: Vec<u16>,
        object: ValueId,
        region: &mut Region,
    ) -> ValueId {
        let result = self.value(Type::ManagedValue);
        let array = self.values[object.0] == Type::ManagedArray;
        let string = self.values[object.0] == Type::ManagedValue;
        if (array || string) && key.as_slice() == [108, 101, 110, 103, 116, 104] {
            let length = self.value(Type::Number);
            Self::push(
                region,
                node,
                if string {
                    Op::ManagedStringLength {
                        result: length,
                        input: object,
                    }
                } else {
                    Op::ManagedArrayLength {
                        result: length,
                        array: object,
                    }
                },
            );
            let primitive = self.value(Type::General);
            Self::push(
                region,
                node,
                Op::Pack {
                    result: primitive,
                    input: length,
                },
            );
            Self::push(
                region,
                node,
                Op::ManagedPrimitive {
                    result,
                    input: primitive,
                },
            );
        } else if string {
            let index = self.value(Type::Number);
            Self::push(
                region,
                node,
                Op::Constant {
                    result: index,
                    value: f64::from(
                        binding_patterns::string_index(&key)
                            .expect("validated optional String index"),
                    ),
                },
            );
            Self::push(
                region,
                node,
                Op::ManagedStringGetIndex {
                    result,
                    input: object,
                    index,
                },
            );
        } else {
            Self::push(
                region,
                node,
                if array {
                    Op::ManagedArrayGetOwn {
                        result,
                        array: object,
                        key,
                    }
                } else {
                    Op::ManagedGetOwn {
                        result,
                        object,
                        key,
                    }
                },
            );
        }
        result
    }

    fn optional_undefined(&mut self, node: &Expression<'_>, region: &mut Region) -> ValueId {
        let primitive = self.value(Type::General);
        Self::push(
            region,
            node,
            Op::Nullish {
                result: primitive,
                value: ir::Nullish::Undefined,
            },
        );
        let result = self.value(Type::ManagedValue);
        Self::push(
            region,
            node,
            Op::ManagedPrimitive {
                result,
                input: primitive,
            },
        );
        result
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn optional_dictionary_reads_require_source_proven_keys_and_receivers() {
        for (body, admitted) in [
            (
                "const d=Object.create(null);d.x=b;const o=a?d:null;const k='x';return +(o?.[k]);",
                true,
            ),
            (
                "const d=Object.create(null);d.constructor=b;const o=a?d:null;return +(o?.constructor);",
                true,
            ),
            ("const k='x';return null?.[k]===undefined?1:0;", true),
            ("const d={x:b};const k='x';return +(d?.[k]);", true),
            (
                "const d=a?Object.create(null):{};const k='x';return +(d?.[k]);",
                true,
            ),
            (
                "const o=a?{x:b}:null;const k=b?'x':'constructor';return +(o?.[k]);",
                false,
            ),
            ("const o=a?[b]:null;const k='x';return +(o?.[k]);", true),
            ("const d=Object.create(null);return +(d?.[b]);", true),
            (
                "const d=Object.create(null);const k=a?'x':1;return +(d?.[k]);",
                true,
            ),
            ("const k={};return null?.[k]===undefined?1:0;", false),
            (
                "const k=()=>{const bad=1n;return 'x';};return null?.[k()]===undefined?1:0;",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';return d?.[k];",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';return d?.[k]();",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';return d?.[k].x;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        for source in [
            "export function run(a,b){const d={x:a};const k={} as unknown as 'x';return +((d as never)?.[k]);}",
            "export function run(a,b){const d=Object.create(null);return +(d?.[({} as unknown as string)]);}",
        ] {
            assert!(crate::compile(source, true).is_err(), "{source}");
        }
    }

    #[test]
    fn optional_boxed_keys_use_the_general_guest_getter_on_known_receivers() {
        for body in [
            "const d=Object.create(null);d.x=b;const row=a?d:null;const k=b?'x':a;return row?.[k]===(a&&b?b:undefined)?1:0;",
            "const xs=a?[b,,a]:null;const k=b?'length':0;return xs?.[k]===(a?(b?3:b):undefined)?1:0;",
            "const xs=a?[b]:null;let n=0;const value=xs?.[(n++,b?'length':0)];return n===(a?1:0)&&value===(a?(b?1:b):undefined)?1:0;",
            "const d=Object.create(null);d.x=b;let row=a?d:null;const value=row?.[(row=null,b?'x':0)];return value===(a&&b?b:undefined)?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn optional_boxed_keys_keep_reference_and_prototype_guards() {
        for body in [
            "const d=Object.create(null);const row=a?d:null;const k={};return +(row?.[k]??7);",
            "const xs=a?[b]:null;const k={};return +(xs?.[k]??7);",
            "const o=a?{x:b}:null;const k=b?'constructor':'x';return +(o?.[k]??7);",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
    }

    #[test]
    fn optional_dictionary_key_depth_stays_bounded_on_a_small_native_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (wraps, admitted) in [(126, true), (127, false)] {
                    let source = format!(
                        "export function run(a,b){{const d=Object.create(null);d.x=b;const k='x';return d?.[{}k{}];}}",
                        "(".repeat(wraps),
                        ")".repeat(wraps)
                    );
                    for typed in [false, true] {
                        let r = crate::compile_value(&source, typed);
                        assert_eq!(r.is_ok(), admitted, "{wraps}: {:?}", r.err());
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn optional_number_string_keys_require_actual_number_and_string_proofs() {
        for (body, ok) in [
            ("const s=a?'ab':null;return s?.[b]===undefined?1:0;", true),
            ("const s='ab';return s?.[a/2]===undefined?1:0;", true),
            ("const s='ab';const k='0';return s?.[k]==='a'?1:0;", true),
            (
                "const s='ab';const k={};return s?.[k]===undefined?1:0;",
                false,
            ),
            ("const xs=[a];return xs?.[b]===undefined?1:0;", true),
            ("const o={0:a};return o?.[b]===undefined?1:0;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){const s='ab';const k='trim';return s?.[(k as number)]==='a'?1:0;}",
                true
            )
            .is_err()
        );
    }

    #[test]
    fn optional_string_reads_require_own_keys_and_actual_string_alternatives() {
        for (body, ok) in [
            ("const s='ab';return s?.length;", true),
            ("const s=a?'ab':null;return +(s?.length);", true),
            ("const s=a?'ab':undefined;return s?.[0]==='a'?1:0;", true),
            ("const s=a?'ab':null;return s?.[a>>>0]==='a'?1:0;", true),
            ("const s='ab';return s?.trim===undefined?1:0;", false),
            ("const s='ab';return s?.[-1]===undefined?1:0;", false),
            ("const s=a?'ab':{};return s?.length;", false),
            ("const s='ab';return s?.[a]==='a'?1:0;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){const s=a?'ab':null;return +(s?.length);}";
        let ts = "export function run(a:string,b:boolean):never{const s=a?'ab':null;return +((s as never)?.length);}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
        assert!(
            crate::compile(
                "export function run(a,b){return (a as string)?.length;}",
                true
            )
            .is_err()
        );
    }

    #[test]
    fn optional_unsigned_key_depth_is_charged_on_a_small_native_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (wraps, admitted) in [(125, true), (126, false)] {
                    let source = format!(
                        "export function run(a,b){{const xs=[a,b];return xs?.[{}a>>>0{}];}}",
                        "(".repeat(wraps),
                        ")".repeat(wraps)
                    );
                    let r = crate::compile_value(&source, false);
                    assert_eq!(r.is_ok(), admitted, "{wraps}: {:?}", r.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn unsigned_optional_reads_require_range_receiver_and_operand_proofs() {
        for (body, ok) in [
            ("const xs=[a,b];return +(xs?.[a>>>0]);", true),
            ("const xs=a?[b]:null;return +(xs?.[0>>>0]);", true),
            ("const xs=null;let n=0;xs?.[(n++,0>>>0)];return n;", true),
            ("const o={0:a};return +(o?.[a>>>0]);", true),
            ("const xs=[a];return +(xs?.[a]);", true),
            ("const xs=[a];return +(xs?.[(xs.length=0,0>>>0)]);", false),
            ("const xs=[a];return +(xs?.[a>>0]);", true),
            ("const xs=[a];return +(xs?.[1n>>>0]);", false),
            ("const xs=null;return +(xs?.[1n>>>0]);", false),
            ("return +(a?.[0>>>0]);", false),
            (
                "const xs=['text'];return (xs?.[0>>>0]).trim().length;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){const xs=a?[b]:null;return +(xs?.[a>>>0]);}";
        let ts = "export function run(a:string,b:boolean):never{const xs=a?[b]:null;return +((xs as never)?.[((a>>>0) as never)]);}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }

    #[test]
    fn optional_receiver_depth_is_charged_on_a_small_native_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (wraps, admitted) in [(126, true), (127, false)] {
                    let source = format!(
                        "export function run(a,b){{const o={{x:a}};return {}o{}?.x;}}",
                        "(".repeat(wraps),
                        ")".repeat(wraps)
                    );
                    let r = crate::compile(&source, false);
                    assert_eq!(r.is_ok(), admitted, "{wraps}: {:?}", r.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn optional_static_reads_keep_source_receiver_and_projection_proofs() {
        for (body, ok) in [
            ("const o={x:a};return o?.x;", true),
            ("const o=a?{x:b}:null;return +(o?.x);", true),
            (
                "const o=a?{x:b}:undefined;return o?.x===undefined?1:0;",
                true,
            ),
            ("const xs=[a,b];return xs?.length;", true),
            ("const xs=a?[b]:null;return +(xs?.[0]);", true),
            ("return null?.x===undefined?1:0;", true),
            ("return null?.constructor===undefined?1:0;", true),
            ("const o={x:a};const n=o?.x;return n;", true),
            ("return a?.x;", false),
            ("const o={x:a};return o?.[b];", false),
            ("const o={x:a};return o?.x.y;", false),
            ("const o={x:a};return o?.x?.y;", false),
            ("const f=x=>x;return f?.(a);", false),
            ("const o=a?{x:b}:null;return o?.x;", false),
            ("const o={x:a};return o?.toString;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){const o=a?{x:b}:null;return +(o?.x);}";
        let ts = "export function run(a:string,b:boolean):never{const o=a?{x:b}:null;return +((o as never)?.x);}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
        assert!(crate::compile("export function run(a,b){return (a as any)?.x;}", true).is_err());
    }
}

#[cfg(test)]
mod boxed_optional_string_keys_tests {
    #[test]
    fn optional_string_reads_accept_guest_coerced_primitive_keys() {
        for body in [
            "const s=a?'ab':null;const k=b?0:false;return s?.[k]===(a&&b?'a':undefined)?1:0;",
            "const s=a?'ab':undefined;const k=b?1:null;return s?.[k]===(a&&b?'b':undefined)?1:0;",
            "const s=a?'ab':null;const k=b?true:0;return s?.[k]===(a?(b?undefined:'a'):undefined)?1:0;",
            "const s=a?'ab':null;let n=0;const k=b?0:false;const ch=s?.[(n++,k)];return n===(a?1:0)&&ch===(a&&b?'a':undefined)?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn optional_string_boxed_keys_retain_to_primitive_and_prototype_guards() {
        for body in [
            "const s=a?'ab':null;const k=b?{}:0;return s?.[k]===undefined?1:0;",
            "const s=a?'ab':null;const k=b?[]:0;return s?.[k]===undefined?1:0;",
            "const s=a?'ab':null;const k=b?'constructor':false;return s?.[k]===undefined?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
    }
}
