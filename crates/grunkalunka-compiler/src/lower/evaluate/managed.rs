//! Managed source values use the ordinary expression cursor and traced IR lanes.
use super::*;
use oxc_ast::ast::{ObjectExpression, ObjectPropertyKind, PropertyKey, PropertyKind};
mod arrays;

pub(super) enum ManagedWork<'e, 'a> {
    Array(arrays::ArrayWork<'e, 'a>),
    ArrayLength(&'e Expression<'a>),
    ResizeReceiver {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        length: u32,
        depth: usize,
    },
    ResizeValue {
        node: &'e Expression<'a>,
        array: ValueId,
        length: u32,
    },
    BoxReference(&'e Expression<'a>),
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
    array: bool,
    depth: usize,
) -> Result<Option<(&'e Expression<'a>, Vec<u16>)>, String> {
    match node {
        Expression::StaticMemberExpression(m) if !m.optional => Ok(Some((
            &m.object,
            member_key(node, m.property.name.encode_utf16().collect(), array)?,
        ))),
        Expression::ComputedMemberExpression(m) if !m.optional => {
            let key = computed_key(node, &m.expression, array, depth + 1)?;
            Ok(Some((&m.object, member_key(node, key, array)?)))
        }
        _ => Ok(None),
    }
}

fn number_consumer<'e, 'a>(
    work: &mut Vec<Work<'e, 'a>>,
    node: &'e Expression<'a>,
    mode: Mode,
) -> Result<(), String> {
    match mode {
        Mode::Number => (),
        Mode::General => work.push(Work::Pack(node)),
        Mode::Predicate => work.push(Work::Truthy(node)),
        Mode::Managed => {
            work.push(Work::Managed(ManagedWork::Primitive(node)));
            work.push(Work::Pack(node));
        }
        _ => {
            return Err(unsupported(
                node,
                "array length requires a numeric value position",
            ));
        }
    }
    Ok(())
}

fn member_key(node: &impl GetSpan, key: Vec<u16>, array: bool) -> Result<Vec<u16>, String> {
    let key = checked_key(node, key)?;
    const ARRAY_INHERITED: &[&str] = &[
        "at",
        "concat",
        "copyWithin",
        "fill",
        "find",
        "findIndex",
        "findLast",
        "findLastIndex",
        "lastIndexOf",
        "pop",
        "push",
        "reverse",
        "shift",
        "unshift",
        "slice",
        "sort",
        "splice",
        "includes",
        "indexOf",
        "join",
        "keys",
        "entries",
        "values",
        "forEach",
        "filter",
        "flat",
        "flatMap",
        "map",
        "every",
        "some",
        "reduce",
        "reduceRight",
        "toReversed",
        "toSorted",
        "toSpliced",
        "with",
    ];
    if array
        && ARRAY_INHERITED
            .iter()
            .any(|s| key.iter().copied().eq(s.encode_utf16()))
    {
        return Err(unsupported(
            node,
            "managed array prototype properties are not yet admitted",
        ));
    }
    Ok(key)
}

fn computed_key(
    node: &impl GetSpan,
    key: &Expression<'_>,
    array: bool,
    depth: usize,
) -> Result<Vec<u16>, String> {
    let key = if array {
        closures::unwrapped_at(key, depth)
            .ok_or_else(|| unsupported(node, "expression nesting exceeds this experiment's limit"))?
            .0
    } else {
        key
    };
    match key {
        Expression::StringLiteral(s) => Ok(s.value.encode_utf16().collect()),
        Expression::NumericLiteral(n)
            if array
                && n.value >= 0.0
                && n.value <= f64::from(u32::MAX)
                && n.value.fract() == 0.0 =>
        {
            Ok((n.value as u32).to_string().encode_utf16().collect())
        }
        _ => Err(unsupported(
            node,
            "managed property keys require a String literal or supported array index literal",
        )),
    }
}

fn assignment_member<'e, 'a>(
    node: &'e Expression<'a>,
    array: bool,
    depth: usize,
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
            member_key(node, m.property.name.encode_utf16().collect(), array)?,
        ))),
        SimpleAssignmentTarget::ComputedMemberExpression(m) if !m.optional => {
            let key = computed_key(node, &m.expression, array, depth + 1)?;
            Ok(Some((&m.object, member_key(node, key, array)?)))
        }
        _ => Ok(None),
    }
}

impl Lowering<'_, '_> {
    pub(in crate::lower) fn array_length_receiver<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
    ) -> Option<&'e Expression<'a>> {
        let (node, depth) = closures::unwrapped_at(node, depth)?;
        let receiver = match node {
            Expression::StaticMemberExpression(m) if !m.optional && m.property.name == "length" => {
                &m.object
            }
            Expression::ComputedMemberExpression(m)
                if !m.optional
                    && closures::unwrapped_at(&m.expression, depth + 1).is_some_and(
                        |(key, _)| matches!(key,Expression::StringLiteral(s) if s.value=="length"),
                    ) =>
            {
                &m.object
            }
            _ => return None,
        };
        (self.managed_source_type(receiver, depth + 1) == Some(Type::ManagedArray))
            .then_some(receiver)
    }

    fn member_array(&self, node: &Expression<'_>, depth: usize) -> bool {
        let object = match node {
            Expression::StaticMemberExpression(m) => Some(&m.object),
            Expression::ComputedMemberExpression(m) => Some(&m.object),
            Expression::AssignmentExpression(a) => match a.left.as_simple_assignment_target() {
                Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => Some(&m.object),
                Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => Some(&m.object),
                _ => None,
            },
            _ => None,
        };
        object.is_some_and(|object| {
            self.managed_source_type(object, depth + 1) == Some(Type::ManagedArray)
        })
    }
    pub(in crate::lower) fn managed_source_type(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Option<Type> {
        enum Shape<'e, 'a> {
            Expr(&'e Expression<'a>, usize),
            Member(bool),
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
                        Expression::ArrayExpression(_) => values.push(Some(Type::ManagedArray)),
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
                            work.push(Shape::Member(m.property.name == "length"));
                            work.push(Shape::Expr(&m.object, depth + 1));
                        }
                        Expression::ComputedMemberExpression(m) if !m.optional => {
                            work.push(Shape::Member(closures::unwrapped_at(&m.expression,depth+1).is_some_and(|(key,_)|matches!(key,Expression::StringLiteral(s) if s.value=="length"))));
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
                        Expression::LogicalExpression(l) => {
                            work.push(Shape::Join);
                            work.push(Shape::Expr(&l.right, depth + 1));
                            work.push(Shape::Expr(&l.left, depth + 1));
                        }
                        Expression::AssignmentExpression(a) => {
                            match a.left.as_simple_assignment_target() {
                                Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => {
                                    work.push(Shape::Member(m.property.name == "length"));
                                    work.push(Shape::Expr(&m.object, depth + 1));
                                }
                                Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => {
                                    work.push(Shape::Member(closures::unwrapped_at(&m.expression,depth+1).is_some_and(|(key,_)|matches!(key,Expression::StringLiteral(s) if s.value=="length"))));
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
                Shape::Member(length) => {
                    let receiver = values.pop().expect("member shape");
                    values.push(
                        (receiver == Some(Type::ManagedObject)
                            || (receiver == Some(Type::ManagedArray) && !length))
                            .then_some(Type::ManagedValue),
                    );
                }
                Shape::Join => {
                    let right = values.pop().expect("alternate shape");
                    let left = values.pop().expect("consequent shape");
                    values.push(
                        if matches!(left, Some(Type::ManagedObject | Type::ManagedArray))
                            && right == left
                        {
                            left
                        } else {
                            left.or(right).map(|_| Type::ManagedValue)
                        },
                    );
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
        if self.member_array(node, depth)
            && let Expression::AssignmentExpression(a) = node
            && let Some((object, key)) = assignment_member(node, true, depth)?
            && key.as_slice() == [108, 101, 110, 103, 116, 104]
        {
            if a.operator != AssignmentOperator::Assign {
                return Err(unsupported(node, "array length requires plain assignment"));
            }
            let length = arrays::literal_length(&a.right, depth + 1)?;
            number_consumer(work, node, mode)?;
            work.push(Work::Managed(ManagedWork::ResizeReceiver {
                node,
                right: &a.right,
                length,
                depth: depth + 1,
            }));
            work.push(Work::Eval(object, depth + 1, Mode::Array));
            return Ok(true);
        }
        if matches!(
            mode,
            Mode::Number | Mode::General | Mode::Managed | Mode::Predicate
        ) && let Some(receiver) = self.array_length_receiver(node, depth)
        {
            number_consumer(work, node, mode)?;
            work.push(Work::Managed(ManagedWork::ArrayLength(node)));
            work.push(Work::Eval(receiver, depth + 1, Mode::Array));
            return Ok(true);
        }
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
            && matches!(
                self.managed_source_type(node, depth),
                Some(Type::ManagedObject | Type::ManagedArray)
            )
        {
            work.push(Work::Managed(ManagedWork::BoxReference(node)));
            let mode = if self.managed_source_type(node, depth) == Some(Type::ManagedArray) {
                Mode::Array
            } else {
                Mode::Object
            };
            work.push(Work::Eval(node, depth, mode));
            return Ok(true);
        }
        if matches!(mode, Mode::Predicate) && self.managed_source_type(node, depth).is_some() {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
            work.push(Work::Eval(node, depth, Mode::Managed));
            return Ok(true);
        }
        if matches!(mode, Mode::Object | Mode::Array | Mode::Managed)
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
                    } else if ty == Type::ManagedArray {
                        Mode::Array
                    } else {
                        Mode::Managed
                    },
                ));
                return Ok(true);
            }
        }
        if matches!(mode, Mode::Array) {
            if schedule_selection(work, node, depth, mode)? {
                return Ok(true);
            }
            if let Expression::ArrayExpression(literal) = node {
                let length = u32::try_from(literal.elements.len())
                    .map_err(|_| unsupported(node, "array length exceeds the supported range"))?;
                let array = self.value(Type::ManagedArray);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedArrayNew {
                        result: array,
                        length,
                    },
                );
                work.push(Work::Managed(ManagedWork::Array(arrays::ArrayWork {
                    node,
                    literal,
                    array,
                    index: 0,
                    depth,
                    awaiting: false,
                })));
                return Ok(true);
            }
            if let Expression::CallExpression(call) = node {
                self.schedule_direct_call(node, call, depth, mode, work)?;
                return Ok(true);
            }
            let result = self.value(Type::ManagedArray);
            let kind = match node {
                Expression::Identifier(i) => {
                    let slot = numeric_binding(i, self.semantic, &self.bindings)?.slot;
                    if self.slots[slot.0].ty != Type::ManagedArray {
                        return Err(unsupported(node, "expected a known array"));
                    }
                    Op::Load { result, slot }
                }
                _ => {
                    return Err(unsupported(
                        node,
                        "array source expression is not yet admitted",
                    ));
                }
            };
            Self::push(regions.current(), node, kind);
            values.push(Value::Array(result));
            return Ok(true);
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
                && let Some((object, key)) = member(node, self.member_array(node, depth), depth)?
            {
                let receiver = self.managed_source_type(object, depth + 1);
                if !matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray)) {
                    return Err(unsupported(
                        node,
                        "managed property receiver must be a known ordinary object",
                    ));
                }
                work.push(Work::Managed(ManagedWork::Read { node, key }));
                work.push(Work::Eval(
                    object,
                    depth + 1,
                    if receiver == Some(Type::ManagedArray) {
                        Mode::Array
                    } else {
                        Mode::Object
                    },
                ));
                return Ok(true);
            }
            if let Some((object, key)) =
                assignment_member(node, self.member_array(node, depth), depth)?
            {
                let Expression::AssignmentExpression(a) = node else {
                    unreachable!()
                };
                if a.operator != AssignmentOperator::Assign {
                    return Err(unsupported(
                        node,
                        "managed properties require plain assignment",
                    ));
                }
                let array = self.managed_source_type(object, depth + 1) == Some(Type::ManagedArray);
                if array && key.as_slice() == [108, 101, 110, 103, 116, 104] {
                    return Err(unsupported(
                        node,
                        "array length writes require a valid literal length",
                    ));
                }
                work.push(Work::Managed(ManagedWork::SetReceiver {
                    node,
                    right: &a.right,
                    depth: depth + 1,
                    key,
                }));
                work.push(Work::Eval(
                    object,
                    depth + 1,
                    if array { Mode::Array } else { Mode::Object },
                ));
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
            ManagedWork::ResizeReceiver {
                node,
                right,
                length,
                depth,
            } => {
                let array = values.pop().expect("resize receiver").id();
                work.push(Work::Managed(ManagedWork::ResizeValue {
                    node,
                    array,
                    length,
                }));
                work.push(Work::Eval(right, depth, Mode::Number));
            }
            ManagedWork::ResizeValue {
                node,
                array,
                length,
            } => {
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedArrayResize { array, length },
                );
            }
            ManagedWork::Array(task) => self.complete_array(task, regions, values, work)?,
            ManagedWork::ArrayLength(node) => {
                let array = values.pop().expect("array length receiver").id();
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedArrayLength { result, array },
                );
                values.push(Value::Number(result));
            }
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
                    if self.values[object.0] == Type::ManagedArray {
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
                    if self.values[object.0] == Type::ManagedArray {
                        Op::ManagedArraySetOwn {
                            array: object,
                            key,
                            value,
                        }
                    } else {
                        Op::ManagedSetOwn { object, key, value }
                    },
                );
            }
            ManagedWork::BoxReference(node) | ManagedWork::Primitive(node) => {
                let input = values.pop().expect("managed input");
                let result = self.value(Type::ManagedValue);
                let kind = match input {
                    Value::Object(input) => Op::ManagedFromObject { result, input },
                    Value::Array(input) => Op::ManagedFromArray { result, input },
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
