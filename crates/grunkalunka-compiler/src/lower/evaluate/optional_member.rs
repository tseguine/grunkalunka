//! Original-source optional own-property reads, using the ordinary owned cursor.
use super::managed::ManagedWork;
use super::*;
use oxc_ast::ast::{ChainElement, ChainExpression};

pub(in crate::lower) fn parts<'e, 'a>(
    chain: &'e ChainExpression<'a>,
    depth: usize,
) -> Option<(&'e Expression<'a>, Vec<u16>)> {
    if depth + 2 > 128 {
        return None;
    }
    match &chain.expression {
        ChainElement::StaticMemberExpression(member) if member.optional => Some((
            &member.object,
            member.property.name.encode_utf16().collect(),
        )),
        ChainElement::ComputedMemberExpression(member) if member.optional => {
            let (key, _) = closures::unwrapped_at(&member.expression, depth + 2)?;
            Some((&member.object, static_keys::literal(key)?))
        }
        _ => None,
    }
}

pub(super) struct Read<'e, 'a> {
    node: &'e Expression<'a>,
    key: Vec<u16>,
    receiver: Option<Type>,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_optional_member<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::ChainExpression(chain) = node else {
            return Ok(false);
        };
        let Some((object, key)) = parts(chain, depth) else {
            return Ok(false);
        };
        let receiver = self.stored.optional_container(object.node_id());
        if receiver.is_none() && !self.stored.nullish(object.node_id()) {
            return Ok(false);
        }
        let array = receiver == Some(Type::ManagedArray);
        let length = array && key.as_slice() == [108, 101, 110, 103, 116, 104];
        let key = if length || receiver.is_none() {
            key
        } else {
            managed_member_key(node, key, array, self.stored.dictionary(object.node_id()))?
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
        work.push(Work::Managed(ManagedWork::OptionalRead(Read {
            node,
            key,
            receiver,
        })));
        // The original receiver executes once, before either branch. No AST
        // replacement or synthetic member expression changes the source route.
        work.push(Work::Eval(object, depth + 2, Mode::Managed));
        Ok(true)
    }

    pub(super) fn complete_optional_member(
        &mut self,
        task: Read<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let Read {
            node,
            key,
            receiver,
        } = task;
        let input = values.pop().expect("optional member receiver").id();
        let Some(ty) = receiver else {
            // Source-proven nullish receiver: evaluation is retained and its
            // static selection is never performed.
            let result = self.optional_undefined(node, regions.current());
            values.push(Value::Managed(result));
            return;
        };
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
        let result = self.value(Type::ManagedValue);
        if ty == Type::ManagedArray && key.as_slice() == [108, 101, 110, 103, 116, 104] {
            let length = self.value(Type::Number);
            Self::push(
                &mut selected,
                node,
                Op::ManagedArrayLength {
                    result: length,
                    array: object,
                },
            );
            let primitive = self.value(Type::General);
            Self::push(
                &mut selected,
                node,
                Op::Pack {
                    result: primitive,
                    input: length,
                },
            );
            Self::push(
                &mut selected,
                node,
                Op::ManagedPrimitive {
                    result,
                    input: primitive,
                },
            );
        } else {
            Self::push(
                &mut selected,
                node,
                if ty == Type::ManagedArray {
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
        selected.exit = Exit::Yield(result);
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
