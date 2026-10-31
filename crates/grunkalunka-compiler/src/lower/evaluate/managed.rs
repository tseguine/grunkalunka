//! Managed source values use the ordinary expression cursor and traced IR lanes.
use super::*;
use oxc_ast::ast::{ObjectExpression, ObjectPropertyKind, PropertyKey, PropertyKind};

pub(super) enum ManagedWork<'e, 'a> {
    BoxObject(&'e Expression<'a>),
    Primitive(&'e Expression<'a>),
    EqualRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        negated: bool,
    },
    Equal {
        node: &'e Expression<'a>,
        left: ValueId,
        negated: bool,
    },
    Literal {
        node: &'e Expression<'a>,
        literal: &'e ObjectExpression<'a>,
        index: usize,
        object: ValueId,
        depth: usize,
    },
    LiteralValue {
        node: &'e Expression<'a>,
        literal: &'e ObjectExpression<'a>,
        index: usize,
        object: ValueId,
        depth: usize,
        key: Vec<u16>,
    },
    Read {
        node: &'e Expression<'a>,
        key: Vec<u16>,
    },
    SetReceiver {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        key: Vec<u16>,
    },
    SetValue {
        node: &'e Expression<'a>,
        object: ValueId,
        key: Vec<u16>,
    },
    Truthy(&'e Expression<'a>),
    Store {
        node: &'e Expression<'a>,
        slot: SlotId,
    },
}

fn checked_key(node: &impl GetSpan, key: Vec<u16>) -> Result<Vec<u16>, String> {
    // Objects enter only by fresh literal construction. No admitted source can
    // mutate their prototype or import a foreign reference. Until prototype
    // operations exist, reject every intrinsic prototype key, even if shadowed.
    const INHERITED: &[&str] = &[
        "constructor",
        "__defineGetter__",
        "__defineSetter__",
        "hasOwnProperty",
        "__lookupGetter__",
        "__lookupSetter__",
        "isPrototypeOf",
        "propertyIsEnumerable",
        "toString",
        "valueOf",
        "__proto__",
        "toLocaleString",
    ];
    if INHERITED
        .iter()
        .any(|name| key.iter().copied().eq(name.encode_utf16()))
    {
        return Err(unsupported(
            node,
            "managed prototype properties are not yet admitted",
        ));
    }
    Ok(key)
}

fn member<'e, 'a>(
    node: &'e Expression<'a>,
) -> Result<Option<(&'e Expression<'a>, Vec<u16>)>, String> {
    match node {
        Expression::StaticMemberExpression(m) if !m.optional => Ok(Some((
            &m.object,
            checked_key(node, m.property.name.encode_utf16().collect())?,
        ))),
        Expression::ComputedMemberExpression(m) if !m.optional => {
            let Expression::StringLiteral(key) = &m.expression else {
                return Err(unsupported(
                    node,
                    "managed property keys require a String literal",
                ));
            };
            Ok(Some((
                &m.object,
                checked_key(node, key.value.encode_utf16().collect())?,
            )))
        }
        _ => Ok(None),
    }
}

fn assignment_member<'e, 'a>(
    node: &'e Expression<'a>,
) -> Result<Option<(&'e Expression<'a>, Vec<u16>)>, String> {
    let Expression::AssignmentExpression(a) = node else {
        return Ok(None);
    };
    let Some(target) = a.left.as_simple_assignment_target() else {
        return Ok(None);
    };
    match target {
        SimpleAssignmentTarget::StaticMemberExpression(m) if !m.optional => Ok(Some((
            &m.object,
            checked_key(node, m.property.name.encode_utf16().collect())?,
        ))),
        SimpleAssignmentTarget::ComputedMemberExpression(m) if !m.optional => {
            let Expression::StringLiteral(key) = &m.expression else {
                return Err(unsupported(
                    node,
                    "managed property keys require a String literal",
                ));
            };
            Ok(Some((
                &m.object,
                checked_key(node, key.value.encode_utf16().collect())?,
            )))
        }
        _ => Ok(None),
    }
}

impl Lowering<'_, '_> {
    pub(in crate::lower) fn managed_source_type(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Option<Type> {
        enum Shape<'e, 'a> {
            Expr(&'e Expression<'a>, usize),
            Member,
            Join,
        }
        let mut work = vec![Shape::Expr(node, depth)];
        let mut values = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Shape::Expr(node, depth) => {
                    if depth > 128 {
                        values.push(None);
                        continue;
                    }
                    if let Some(inner) = erased_inner(node) {
                        work.push(Shape::Expr(inner, depth + 1));
                        continue;
                    }
                    match node {
                        Expression::ParenthesizedExpression(w) => {
                            work.push(Shape::Expr(&w.expression, depth + 1))
                        }
                        Expression::ObjectExpression(_) => values.push(Some(Type::ManagedObject)),
                        Expression::StringLiteral(_) => values.push(Some(Type::ManagedValue)),
                        Expression::CallExpression(call) => values.push(
                            managed_calls::function(call, self.semantic, self.functions)
                                .map(|f| f.return_type)
                                .filter(|t| t.is_managed()),
                        ),
                        Expression::Identifier(i) => values.push(
                            numeric_binding(i, self.semantic, &self.bindings)
                                .ok()
                                .map(|b| self.slots[b.slot.0].ty)
                                .filter(|ty| ty.is_managed()),
                        ),
                        Expression::StaticMemberExpression(m) if !m.optional => {
                            work.push(Shape::Member);
                            work.push(Shape::Expr(&m.object, depth + 1));
                        }
                        Expression::ComputedMemberExpression(m) if !m.optional => {
                            work.push(Shape::Member);
                            work.push(Shape::Expr(&m.object, depth + 1));
                        }
                        Expression::SequenceExpression(s) => {
                            if let Some(last) = s.expressions.last() {
                                work.push(Shape::Expr(last, depth + 1));
                            } else {
                                values.push(None);
                            }
                        }
                        Expression::ConditionalExpression(c) => {
                            work.push(Shape::Join);
                            work.push(Shape::Expr(&c.alternate, depth + 1));
                            work.push(Shape::Expr(&c.consequent, depth + 1));
                        }
                        Expression::AssignmentExpression(a) => {
                            match a.left.as_simple_assignment_target() {
                                Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => {
                                    work.push(Shape::Member);
                                    work.push(Shape::Expr(&m.object, depth + 1));
                                }
                                Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => {
                                    work.push(Shape::Member);
                                    work.push(Shape::Expr(&m.object, depth + 1));
                                }
                                Some(SimpleAssignmentTarget::AssignmentTargetIdentifier(i)) => {
                                    values.push(
                                        numeric_binding(i, self.semantic, &self.bindings)
                                            .ok()
                                            .map(|b| self.slots[b.slot.0].ty)
                                            .filter(|ty| ty.is_managed()),
                                    )
                                }
                                _ => values.push(None),
                            }
                        }
                        _ => values.push(None),
                    }
                }
                Shape::Member => {
                    let receiver = values.pop().expect("member shape");
                    values.push(
                        (receiver == Some(Type::ManagedObject)).then_some(Type::ManagedValue),
                    );
                }
                Shape::Join => {
                    let right = values.pop().expect("alternate shape");
                    let left = values.pop().expect("consequent shape");
                    values.push(if left == Some(Type::ManagedObject) && right == left {
                        left
                    } else {
                        left.or(right).map(|_| Type::ManagedValue)
                    });
                }
            }
        }
        values.pop().flatten()
    }

    pub(super) fn schedule_managed<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        regions: &mut Regions<'_>,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if matches!(mode, Mode::Predicate)
            && let Expression::BinaryExpression(binary) = node
            && matches!(
                binary.operator,
                BinaryOperator::Equality
                    | BinaryOperator::Inequality
                    | BinaryOperator::StrictEquality
                    | BinaryOperator::StrictInequality
            )
            && (self.managed_source_type(&binary.left, depth + 1).is_some()
                || self.managed_source_type(&binary.right, depth + 1).is_some())
        {
            if !matches!(
                binary.operator,
                BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
            ) {
                return Err(unsupported(
                    node,
                    "expression is outside the numeric experiment: managed comparisons require strict equality",
                ));
            }
            work.push(Work::Managed(ManagedWork::EqualRight {
                node,
                right: &binary.right,
                depth: depth + 1,
                negated: binary.operator == BinaryOperator::StrictInequality,
            }));
            work.push(Work::Eval(&binary.left, depth + 1, Mode::Managed));
            return Ok(true);
        }
        if matches!(mode, Mode::Managed)
            && self.managed_source_type(node, depth) == Some(Type::ManagedObject)
        {
            work.push(Work::Managed(ManagedWork::BoxObject(node)));
            work.push(Work::Eval(node, depth, Mode::Object));
            return Ok(true);
        }
        if matches!(mode, Mode::Predicate) && self.managed_source_type(node, depth).is_some() {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
            work.push(Work::Eval(node, depth, Mode::Managed));
            return Ok(true);
        }
        if matches!(mode, Mode::Object | Mode::Managed)
            && let Expression::AssignmentExpression(a) = node
            && let Some(target) = a.left.as_simple_assignment_target()
            && let Some(identifier) = target_identifier(target, depth)?
        {
            let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
            let slot = binding.slot;
            let ty = self.slots[slot.0].ty;
            if ty.is_managed() {
                if !binding.mutable {
                    return Err(unsupported(
                        node,
                        "assignment to const binding is not admitted",
                    ));
                }
                if a.operator != AssignmentOperator::Assign {
                    return Err(unsupported(
                        node,
                        "managed bindings require plain assignment",
                    ));
                }
                work.push(Work::Managed(ManagedWork::Store { node, slot }));
                work.push(Work::Eval(
                    &a.right,
                    depth + 1,
                    if ty == Type::ManagedObject {
                        Mode::Object
                    } else {
                        Mode::Managed
                    },
                ));
                return Ok(true);
            }
        }
        if matches!(mode, Mode::Object) {
            if schedule_selection(work, node, depth, mode)? {
                return Ok(true);
            }
            let kind = match node {
                Expression::ObjectExpression(literal) => {
                    let object = self.value(Type::ManagedObject);
                    Self::push(
                        regions.current(),
                        node,
                        Op::ManagedObjectNew { result: object },
                    );
                    work.push(Work::Managed(ManagedWork::Literal {
                        node,
                        literal,
                        index: 0,
                        object,
                        depth,
                    }));
                    return Ok(true);
                }
                Expression::CallExpression(call) => {
                    self.schedule_direct_call(node, call, depth, mode, work)?;
                    return Ok(true);
                }
                Expression::Identifier(identifier) => {
                    let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
                    if self.slots[slot.0].ty != Type::ManagedObject {
                        return Err(unsupported(node, "expected a known ordinary object"));
                    }
                    slot
                }
                _ => {
                    return Err(unsupported(
                        node,
                        "ordinary object source expression is not yet admitted",
                    ));
                }
            };
            let result = self.value(Type::ManagedObject);
            Self::push(regions.current(), node, Op::Load { result, slot: kind });
            values.push(Value::Object(result));
            return Ok(true);
        }
        if matches!(mode, Mode::Managed) {
            if let Expression::StringLiteral(literal) = node {
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedString {
                        result,
                        units: literal.value.encode_utf16().collect(),
                    },
                );
                values.push(Value::Managed(result));
                return Ok(true);
            }
            if let Expression::CallExpression(call) = node
                && self.managed_source_type(node, depth) == Some(Type::ManagedValue)
            {
                self.schedule_direct_call(node, call, depth, mode, work)?;
                return Ok(true);
            }
            if schedule_selection(work, node, depth, mode)? {
                return Ok(true);
            }
            if self.managed_source_type(node, depth) == Some(Type::ManagedValue)
                && let Some((object, key)) = member(node)?
            {
                if self.managed_source_type(object, depth + 1) != Some(Type::ManagedObject) {
                    return Err(unsupported(
                        node,
                        "managed property receiver must be a known ordinary object",
                    ));
                }
                work.push(Work::Managed(ManagedWork::Read { node, key }));
                work.push(Work::Eval(object, depth + 1, Mode::Object));
                return Ok(true);
            }
            if let Some((object, key)) = assignment_member(node)? {
                let Expression::AssignmentExpression(a) = node else {
                    unreachable!()
                };
                if a.operator != AssignmentOperator::Assign {
                    return Err(unsupported(
                        node,
                        "managed properties require plain assignment",
                    ));
                }
                work.push(Work::Managed(ManagedWork::SetReceiver {
                    node,
                    right: &a.right,
                    depth: depth + 1,
                    key,
                }));
                work.push(Work::Eval(object, depth + 1, Mode::Object));
                return Ok(true);
            }
            if let Expression::Identifier(identifier) = node
                && let Ok(binding) = numeric_binding(identifier, self.semantic, &self.bindings)
                && self.slots[binding.slot.0].ty == Type::ManagedValue
            {
                let slot = binding.slot;
                let result = self.value(Type::ManagedValue);
                Self::push(regions.current(), node, Op::Load { result, slot });
                values.push(Value::Managed(result));
            } else {
                work.push(Work::Managed(ManagedWork::Primitive(node)));
                work.push(Work::Eval(node, depth, Mode::General));
            }
            return Ok(true);
        }
        if self.managed_source_type(node, depth).is_some() {
            return Err(unsupported(
                node,
                "expression is outside the numeric experiment: managed value requires an identity or managed storage position",
            ));
        }
        Ok(false)
    }

    pub(super) fn complete_managed<'e, 'a>(
        &mut self,
        task: ManagedWork<'e, 'a>,
        regions: &mut Regions<'_>,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        match task {
            ManagedWork::Truthy(node) => {
                let input = values.pop().expect("managed truthiness").id();
                let result = self.value(Type::Predicate);
                Self::push(regions.current(), node, Op::ManagedTruthy { result, input });
                values.push(Value::Predicate(result));
            }
            ManagedWork::Store { node, slot } => {
                let value = values.last().expect("managed assignment result").id();
                Self::push(regions.current(), node, Op::Store { slot, value });
            }
            ManagedWork::Literal {
                node,
                literal,
                index,
                object,
                depth,
            } => {
                let Some(property) = literal.properties.get(index) else {
                    values.push(Value::Object(object));
                    return Ok(());
                };
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(unsupported(
                        node,
                        "managed object spread is not yet admitted",
                    ));
                };
                if property.kind != PropertyKind::Init || property.method {
                    return Err(unsupported(
                        property.as_ref(),
                        "managed accessors and methods are not yet admitted",
                    ));
                }
                let key = match &property.key {
                    PropertyKey::StaticIdentifier(i) if !property.computed => {
                        i.name.encode_utf16().collect()
                    }
                    PropertyKey::StringLiteral(s) => s.value.encode_utf16().collect(),
                    _ => {
                        return Err(unsupported(
                            property.as_ref(),
                            "managed literal keys require an identifier or String literal",
                        ));
                    }
                };
                let key = checked_key(property.as_ref(), key)?;
                work.push(Work::Managed(ManagedWork::LiteralValue {
                    node,
                    literal,
                    index,
                    object,
                    depth,
                    key,
                }));
                work.push(Work::Eval(&property.value, depth + 1, Mode::Managed));
            }
            ManagedWork::LiteralValue {
                node,
                literal,
                index,
                object,
                depth,
                key,
            } => {
                let value = values.pop().expect("literal property value").id();
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedSetOwn { object, key, value },
                );
                work.push(Work::Managed(ManagedWork::Literal {
                    node,
                    literal,
                    index: index + 1,
                    object,
                    depth,
                }));
            }
            ManagedWork::Read { node, key } => {
                let object = values.pop().expect("property receiver").id();
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedGetOwn {
                        result,
                        object,
                        key,
                    },
                );
                values.push(Value::Managed(result));
            }
            ManagedWork::SetReceiver {
                node,
                right,
                depth,
                key,
            } => {
                let object = values.pop().expect("assignment receiver").id();
                work.push(Work::Managed(ManagedWork::SetValue { node, object, key }));
                work.push(Work::Eval(right, depth, Mode::Managed));
            }
            ManagedWork::SetValue { node, object, key } => {
                let value = values.last().expect("assigned managed value").id();
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedSetOwn { object, key, value },
                );
            }
            ManagedWork::BoxObject(node) | ManagedWork::Primitive(node) => {
                let input = values.pop().expect("managed input");
                let result = self.value(Type::ManagedValue);
                let kind = match input {
                    Value::Object(input) => Op::ManagedFromObject { result, input },
                    Value::General(input) => Op::ManagedPrimitive { result, input },
                    _ => unreachable!("checked managed bridge"),
                };
                Self::push(regions.current(), node, kind);
                values.push(Value::Managed(result));
            }
            ManagedWork::EqualRight {
                node,
                right,
                depth,
                negated,
            } => {
                let left = values.pop().expect("managed equality left").id();
                work.push(Work::Managed(ManagedWork::Equal {
                    node,
                    left,
                    negated,
                }));
                work.push(Work::Eval(right, depth, Mode::Managed));
            }
            ManagedWork::Equal {
                node,
                left,
                negated,
            } => {
                let right = values.pop().expect("managed equality right").id();
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedEqual {
                        result,
                        left,
                        right,
                    },
                );
                let result = if negated {
                    let negated = self.value(Type::Predicate);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Not {
                            result: negated,
                            input: result,
                        },
                    );
                    negated
                } else {
                    result
                };
                values.push(Value::Predicate(result));
            }
        }
        Ok(())
    }
}
