//! Define one data property at a time. A runtime key is saved before the
//! value expression runs, including when either expression suspends.
use super::*;
use oxc_ast::ast::{ObjectExpression, ObjectPropertyKind, PropertyKind};

pub(in crate::lower::evaluate) struct ObjectWork<'e, 'a> {
    node: &'e Expression<'a>,
    literal: &'e ObjectExpression<'a>,
    object: ValueId,
    depth: usize,
    index: usize,
    pending: Pending<'e, 'a>,
}

enum Pending<'e, 'a> {
    Next,
    Key {
        value: &'e Expression<'a>,
        uint32: bool,
    },
    // None copies a spread source into the literal.
    Value(Option<ir::ManagedKey>),
}

impl<'e, 'a> ObjectWork<'e, 'a> {
    pub(super) fn new(
        node: &'e Expression<'a>,
        literal: &'e ObjectExpression<'a>,
        object: ValueId,
        depth: usize,
    ) -> Self {
        Self {
            node,
            literal,
            object,
            depth,
            index: 0,
            pending: Pending::Next,
        }
    }
}

impl Lowering<'_, '_> {
    pub(super) fn complete_object<'e, 'a>(
        &mut self,
        mut task: ObjectWork<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let depth = task.depth + 1;
        match std::mem::replace(&mut task.pending, Pending::Next) {
            Pending::Next => (),
            Pending::Key { value, uint32 } => {
                let key = values.pop().expect("literal computed key").id();
                task.pending = Pending::Value(Some(if uint32 {
                    ir::ManagedKey::Uint32(key)
                } else {
                    ir::ManagedKey::String(key)
                }));
                work.push(Work::Managed(ManagedWork::Literal(task)));
                work.push(Work::Eval(value, depth, Mode::Managed));
                return Ok(());
            }
            Pending::Value(key) => {
                let value = values.pop().expect("literal property value").id();
                let object = task.object;
                Self::push(
                    regions.current(),
                    task.node,
                    match key {
                        Some(ir::ManagedKey::Literal(key)) => {
                            Op::ManagedSetOwn { object, key, value }
                        }
                        Some(key) => Op::ManagedSetOwnDynamic { object, key, value },
                        None => Op::ManagedObjectSpread {
                            object,
                            input: value,
                        },
                    },
                );
                task.index += 1;
            }
        }
        let Some(property) = task.literal.properties.get(task.index) else {
            values.push(Value::Object(task.object));
            return Ok(());
        };
        let mut mode = Mode::Managed;
        let expr = match property {
            ObjectPropertyKind::SpreadProperty(spread) => {
                task.pending = Pending::Value(None);
                &spread.argument
            }
            ObjectPropertyKind::ObjectProperty(property) => {
                if property.kind != PropertyKind::Init || property.method {
                    return Err(unsupported(
                        property.as_ref(),
                        "managed accessors and methods are not yet admitted",
                    ));
                }
                if let Some(key) = static_keys::property(&property.key, property.computed, depth) {
                    // A computed definition, including ["__proto__"], creates
                    // an own data property. The uncomputed prototype setter
                    // and guarded ordinary property names keep their old boundary.
                    let key = if property.computed {
                        key
                    } else {
                        checked_key(property.as_ref(), key)?
                    };
                    task.pending = Pending::Value(Some(ir::ManagedKey::Literal(key)));
                    &property.value
                } else if property.computed
                    && let Some(key) = property.key.as_expression()
                    && (self.stored.string(key.node_id())
                        || self.stored.uint32_keys.contains(key, depth))
                {
                    let uint32 = self.stored.uint32_keys.contains(key, depth);
                    if uint32 {
                        mode = Mode::Number;
                    }
                    task.pending = Pending::Key {
                        value: &property.value,
                        uint32,
                    };
                    key
                } else {
                    return Err(unsupported(
                        property.as_ref(),
                        "managed literal keys require a static name or proven String",
                    ));
                }
            }
        };
        work.push(Work::Managed(ManagedWork::Literal(task)));
        work.push(Work::Eval(expr, depth, mode));
        Ok(())
    }
}

#[cfg(test)]
mod uint32_tests {
    #[test]
    fn unsigned_computed_keys_retain_operand_and_annotation_guards() {
        for (body, admitted) in [
            ("const o={[a>>>0]:b};return Object.keys(o).length;", true),
            (
                "const o={[(a=1,b>>>0)]:a};return Object.keys(o).length;",
                true,
            ),
            (
                "const o={[a?(0>>>0):(1>>>0)]:b};return Object.keys(o).length;",
                true,
            ),
            (
                "let key=0;const o={[key=a>>>0]:b};return Object.keys(o).length;",
                true,
            ),
            ("const o={[a]:b};return a;", false),
            ("const o={[a>>0]:b};return a;", false),
            ("const o={[a?(0>>>0):b]:a};return a;", false),
            ("const o={[(Math.random(),a>>>0)]:b};return a;", false),
            ("const key=a>>>0;const o={[key]:b};return a;", true),
            ("let key=a>>>0;key=a;const o={[key]:b};return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        let js = "export function run(a,b){const o={[a>>>0]:b};return Object.keys(o).length;}";
        let ts = "export function run(a:string,b:boolean):never{const o={[(a>>>0) as never]:b};return Object.keys(o).length;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
        assert!(
            crate::compile(
                "export function run(a:number,b:number){const o={[a as number]:b};return a;}",
                true
            )
            .is_err()
        );
    }
}
