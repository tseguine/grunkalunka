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
    Key { value: &'e Expression<'a> },
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
            Pending::Key { value } => {
                let key = values.pop().expect("literal computed key").id();
                task.pending = Pending::Value(Some(ir::ManagedKey::String(key)));
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
                        Some(ir::ManagedKey::String(key)) => {
                            Op::ManagedSetOwnDynamic { object, key, value }
                        }
                        Some(ir::ManagedKey::Uint32(_)) => {
                            unreachable!("literal keys are static or String")
                        }
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
                    && self.stored.string(key.node_id())
                {
                    task.pending = Pending::Key {
                        value: &property.value,
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
        work.push(Work::Eval(expr, depth, Mode::Managed));
        Ok(())
    }
}
