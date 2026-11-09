//! Managed source values use the ordinary expression cursor and traced IR lanes.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum EqualityKind {
    Exact,
    LeftNullish,
    RightNullish,
}
mod arrays;
mod objects;

pub(super) enum ManagedWork<'e, 'a> {
    OptionalRead(optional_member::Read<'e, 'a>),
    StringIndexReceiver {
        node: &'e Expression<'a>,
        index: &'e Expression<'a>,
        depth: usize,
    },
    StringIndex {
        node: &'e Expression<'a>,
        input: ValueId,
    },
    DictionaryNew(&'e Expression<'a>),
    Array(arrays::ArrayWork<'e, 'a>),
    SpreadCopy(&'e Expression<'a>),
    ArrayLength(&'e Expression<'a>),
    StringLength(&'e Expression<'a>),
    ConcatRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
    },
    Concat {
        node: &'e Expression<'a>,
        left: ValueId,
    },
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
    StringCompareRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        operator: Relational,
    },
    StringCompare {
        node: &'e Expression<'a>,
        left: ValueId,
        operator: Relational,
    },
    EqualRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        negated: bool,
        kind: EqualityKind,
    },
    Equal {
        node: &'e Expression<'a>,
        left: ValueId,
        negated: bool,
        kind: EqualityKind,
    },
    Literal(objects::ObjectWork<'e, 'a>),
    Read {
        node: &'e Expression<'a>,
        key: Vec<u16>,
    },
    FunctionProjection {
        node: &'e Expression<'a>,
        signature: usize,
    },
    ReferenceProjection {
        node: &'e Expression<'a>,
        ty: Type,
    },
    NumberProjection(&'e Expression<'a>),
    PrimitiveProjection(&'e Expression<'a>),
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
    Typeof(&'e Expression<'a>),
    ConstantTypeof(&'e Expression<'a>, &'static str),
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
    if crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &key) {
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
    dictionary: bool,
    depth: usize,
) -> Result<Option<(&'e Expression<'a>, Vec<u16>)>, String> {
    match node {
        Expression::StaticMemberExpression(m) if !m.optional => Ok(Some((
            &m.object,
            member_key(
                node,
                m.property.name.encode_utf16().collect(),
                array,
                dictionary,
            )?,
        ))),
        Expression::ComputedMemberExpression(m) if !m.optional => {
            let key = computed_key(node, &m.expression, depth + 1)?;
            Ok(Some((&m.object, member_key(node, key, array, dictionary)?)))
        }
        _ => Ok(None),
    }
}

pub(super) fn number_consumer<'e, 'a>(
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
                "length requires a numeric value position",
            ));
        }
    }
    Ok(())
}

pub(in crate::lower) fn member_key(
    node: &impl GetSpan,
    key: Vec<u16>,
    array: bool,
    dictionary: bool,
) -> Result<Vec<u16>, String> {
    if dictionary {
        return Ok(key);
    }
    let key = checked_key(node, key)?;
    if array && crate::prototype_keys::contains(crate::prototype_keys::ARRAY, &key) {
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
    depth: usize,
) -> Result<Vec<u16>, String> {
    let (key, _) = closures::unwrapped_at(key, depth)
        .ok_or_else(|| unsupported(node, "expression nesting exceeds this experiment's limit"))?;
    static_keys::literal(key).ok_or_else(|| {
        unsupported(
            node,
            "managed property keys require a String or numeric literal",
        )
    })
}

fn assignment_member<'e, 'a>(
    node: &'e Expression<'a>,
    array: bool,
    dictionary: bool,
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
            member_key(
                node,
                m.property.name.encode_utf16().collect(),
                array,
                dictionary,
            )?,
        ))),
        SimpleAssignmentTarget::ComputedMemberExpression(m) if !m.optional => {
            let key = computed_key(node, &m.expression, depth + 1)?;
            Ok(Some((&m.object, member_key(node, key, array, dictionary)?)))
        }
        _ => Ok(None),
    }
}

impl Lowering<'_, '_> {
    pub(in crate::lower) fn receiver_type(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Option<Type> {
        match self.managed_source_type(node, depth) {
            Some(Type::ManagedValue) => self.stored.container(node.node_id()),
            ty => ty,
        }
    }
    pub(super) fn schedule_function_member<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        signature: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if !matches!(
            node,
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)
        ) {
            return Ok(false);
        }
        if self.function_arity(node, depth) != Some(signature) {
            return Err(unsupported(
                node,
                "stored function field requires a complete agreeing signature",
            ));
        }
        let Some((object, key)) = member(
            node,
            self.member_array(node, depth),
            self.member_dictionary(node),
            depth,
        )?
        else {
            return Ok(false);
        };
        let receiver = self.receiver_type(object, depth + 1);
        if !matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray)) {
            return Err(unsupported(
                node,
                "stored function receiver must be a known object or array",
            ));
        }
        work.push(Work::Managed(ManagedWork::FunctionProjection {
            node,
            signature,
        }));
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
        Ok(true)
    }
    pub(in crate::lower) fn managed_length_receiver<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
    ) -> Option<(&'e Expression<'a>, Type)> {
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
        if self.receiver_type(receiver, depth + 1) == Some(Type::ManagedArray) {
            Some((receiver, Type::ManagedArray))
        } else {
            self.stored
                .string(receiver.node_id())
                .then_some((receiver, Type::ManagedValue))
        }
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
        object
            .is_some_and(|object| self.receiver_type(object, depth + 1) == Some(Type::ManagedArray))
    }

    fn member_dictionary(&self, node: &Expression<'_>) -> bool {
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
        object.is_some_and(|object| self.stored.dictionary(object.node_id()))
    }
    pub(in crate::lower) fn managed_source_type(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Option<Type> {
        enum Shape<'e, 'a> {
            Expr(&'e Expression<'a>, usize),
            Member(&'e Expression<'a>, bool),
            ArrayElement(&'e Expression<'a>, Option<Type>, Type),
            ArraySlice(Option<Type>),
            Join,
        }
        let mut work = vec![Shape::Expr(node, depth)];
        let mut values = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Shape::ArraySlice(fallback) => {
                    let receiver = values.pop().expect("slice receiver shape");
                    values.push(if receiver == Some(Type::ManagedArray) {
                        Some(Type::ManagedArray)
                    } else {
                        fallback
                    });
                }
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
                        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
                            values.push(Some(Type::ManagedValue))
                        }
                        Expression::AwaitExpression(_) if self.stored.string(node.node_id()) => {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::BinaryExpression(b)
                            if b.operator == BinaryOperator::Addition
                                && self.stored.string(node.node_id()) =>
                        {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::ArrowFunctionExpression(arrow)
                            if self
                                .managed_kinds
                                .identity_bodies
                                .contains(&arrow.node_id.get()) =>
                        {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::UnaryExpression(u) if u.operator == UnaryOperator::Typeof => {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::CallExpression(call)
                            if self
                                .managed_kinds
                                .identity_calls
                                .contains(&call.node_id.get())
                                && self.function_arity(node, depth).is_some() =>
                        {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::CallExpression(call)
                            if object_intrinsics::enumeration(call, self.semantic).is_some() =>
                        {
                            values.push(Some(Type::ManagedArray))
                        }
                        Expression::CallExpression(call)
                            if object_create::argument(call, self.semantic).is_some() =>
                        {
                            values.push(Some(Type::ManagedObject))
                        }
                        Expression::CallExpression(call)
                            if crate::lower::string_methods::call(call).is_some_and(
                                |(receiver, method)| {
                                    method.result() == Type::ManagedValue
                                        && self.stored.string(receiver.node_id())
                                },
                            ) =>
                        {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::CallExpression(call) => {
                            let fallback =
                                managed_calls::function(call, self.semantic, self.functions)
                                    .map(|f| f.return_type)
                                    .filter(|t| t.is_managed())
                                    .or_else(|| {
                                        self.managed_kinds.calls.get(&call.node_id.get()).copied()
                                    })
                                    .or_else(|| {
                                        self.closure_target(
                                            invocation::callee(call),
                                            invocation::depth(call, depth + 1),
                                        )
                                        .filter(|closure| {
                                            closure.checked
                                                && !closure.factory
                                                && !self.arrow(closure.arrow).r#async
                                        })
                                        .map(|closure| closure.result)
                                        .filter(|ty| ty.is_managed())
                                    })
                                    .or_else(|| {
                                        self.runtime_closure_callee(
                                            invocation::callee(call),
                                            invocation::depth(call, depth + 1),
                                        )
                                        .and_then(|(_, id)| {
                                            self.closure_signatures.borrow().result(id)
                                        })
                                        .filter(|ty| ty.is_managed())
                                    });
                            if let Some((receiver, crate::lower::string_methods::Method::Slice)) =
                                crate::lower::string_methods::call(call)
                            {
                                work.push(Shape::ArraySlice(fallback));
                                work.push(Shape::Expr(receiver, depth + 1));
                            } else if let Some((receiver, result)) =
                                crate::lower::array_at::receiver(call)
                                    .map(|r| (r, Type::ManagedValue))
                                    .or_else(|| {
                                        crate::lower::array_mutations::operation(call)
                                            .map(|(op, receiver)| (receiver, op.result()))
                                    })
                            {
                                work.push(Shape::ArrayElement(receiver, fallback, result));
                                work.push(Shape::Expr(receiver, depth + 1));
                            } else {
                                values.push(fallback);
                            }
                        }
                        Expression::Identifier(i) => values.push(
                            numeric_binding(i, self.semantic, &self.bindings)
                                .ok()
                                .and_then(|binding| {
                                    let ty = self.slots[binding.slot.0].ty;
                                    if ty.is_managed() {
                                        Some(ty)
                                    } else {
                                        (matches!(ty, Type::Closure(_))
                                            && self
                                                .managed_kinds
                                                .identity_symbols
                                                .contains(&binding.symbol))
                                        .then_some(Type::ManagedValue)
                                    }
                                })
                                .or_else(|| {
                                    self.named_module_function(node, depth)
                                        .map(|_| Type::ManagedValue)
                                }),
                        ),
                        Expression::ChainExpression(chain)
                            if optional_member::parts(chain, depth).is_some() =>
                        {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::StaticMemberExpression(m) if !m.optional => {
                            work.push(Shape::Member(&m.object, m.property.name == "length"));
                            work.push(Shape::Expr(&m.object, depth + 1));
                        }
                        Expression::ComputedMemberExpression(m) if !m.optional => {
                            work.push(Shape::Member(&m.object,closures::unwrapped_at(&m.expression,depth+1).is_some_and(|(key,_)|matches!(key,Expression::StringLiteral(s) if s.value=="length"))));
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
                        Expression::AssignmentExpression(a)
                            if a.operator == AssignmentOperator::Addition
                                && self.stored.string(a.node_id()) =>
                        {
                            values.push(Some(Type::ManagedValue));
                        }
                        Expression::AssignmentExpression(a)
                            if property_update::compound(a.operator).is_some() =>
                        {
                            // Numeric compound writes return a Number even
                            // when the target stores that result boxed.
                            values.push(None);
                        }
                        Expression::AssignmentExpression(a) => {
                            if a.left.as_assignment_target_pattern().is_some() {
                                work.push(Shape::Expr(&a.right, depth + 1));
                                continue;
                            }
                            match a.left.as_simple_assignment_target() {
                                Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => {
                                    work.push(Shape::Member(
                                        &m.object,
                                        m.property.name == "length",
                                    ));
                                    work.push(Shape::Expr(&m.object, depth + 1));
                                }
                                Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => {
                                    work.push(Shape::Member(&m.object,closures::unwrapped_at(&m.expression,depth+1).is_some_and(|(key,_)|matches!(key,Expression::StringLiteral(s) if s.value=="length"))));
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
                Shape::ArrayElement(source, fallback, result) => {
                    let receiver = match values.pop().expect("array element receiver shape") {
                        Some(Type::ManagedValue) => self.stored.container(source.node_id()),
                        ty => ty,
                    };
                    values.push(if receiver == Some(Type::ManagedArray) {
                        Some(result)
                    } else {
                        fallback
                    });
                }
                Shape::Member(source, length) => {
                    let receiver = match values.pop().expect("member shape") {
                        Some(Type::ManagedValue) => self.stored.container(source.node_id()),
                        ty => ty,
                    };
                    values.push(
                        (receiver == Some(Type::ManagedObject)
                            || (self.stored.string(source.node_id()) && !length)
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
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if matches!(mode, Mode::Managed)
            && matches!(node, Expression::AwaitExpression(_))
            && self.stored.primitive(node.node_id())
        {
            // Keep the General await protocol and primitive language tag,
            // then box its completed result for managed storage.
            work.push(Work::Managed(ManagedWork::Primitive(node)));
            work.push(Work::Eval(node, depth, Mode::General));
            return Ok(true);
        }
        if matches!(mode, Mode::General)
            && self.stored.primitive(node.node_id())
            && self.managed_source_type(node, depth) == Some(Type::ManagedValue)
        {
            // Preserve primitive tags and Number bits. Coercion belongs to
            // the existing scalar consumer, never to this storage bridge.
            work.push(Work::Managed(ManagedWork::PrimitiveProjection(node)));
            work.push(Work::Eval(node, depth, Mode::Managed));
            return Ok(true);
        }
        if self.schedule_optional_member(node, depth, mode, work)?
            || self.schedule_string_await(node, depth, mode, work)?
            || self.schedule_string_addition(node, depth, mode, regions, work)?
            || self.schedule_local_logical(node, depth, mode, regions, work)?
            || self.schedule_local_update(node, depth, mode, regions, values, work)?
            || self.schedule_dictionary_logical(node, depth, mode, work)?
            || self.schedule_property_logical(node, depth, mode, work)?
            || self.schedule_property_update(node, depth, mode, work)?
            || self.schedule_property_delete(node, depth, mode, work)?
            || self.schedule_destructure(node, depth, mode, work)?
        {
            return Ok(true);
        }
        if matches!(mode, Mode::Number | Mode::General)
            && self.stored.number(node.node_id())
            && self.managed_source_type(node, depth) == Some(Type::ManagedValue)
        {
            if matches!(mode, Mode::General) {
                work.push(Work::Pack(node));
            }
            work.push(Work::Managed(ManagedWork::NumberProjection(node)));
            work.push(Work::Eval(node, depth, Mode::Managed));
            return Ok(true);
        }
        if matches!(mode, Mode::Managed)
            && let Expression::ComputedMemberExpression(member) = node
            && !member.optional
            && self.stored.string(member.object.node_id())
            && !matches!(
                closures::unwrapped_at(&member.expression, depth + 1).map(|(e, _)| e),
                Some(Expression::StringLiteral(_))
            )
        {
            work.push(Work::Managed(ManagedWork::StringIndexReceiver {
                node,
                index: &member.expression,
                depth: depth + 1,
            }));
            work.push(Work::Eval(&member.object, depth + 1, Mode::Managed));
            return Ok(true);
        }
        if matches!(mode, Mode::Managed)
            && (self.schedule_dictionary_read(node, depth, work)?
                || self.schedule_dictionary_write(node, depth, work)?)
        {
            return Ok(true);
        }
        if matches!(mode, Mode::Object)
            && let Expression::CallExpression(call) = node
            && let Some(argument) = object_create::argument(call, self.semantic)
        {
            work.push(Work::Managed(ManagedWork::DictionaryNew(node)));
            work.push(Work::Eval(argument, depth + 1, Mode::Discard));
            return Ok(true);
        }
        if let Some(ty) = match mode {
            Mode::Object => Some(Type::ManagedObject),
            Mode::Array => Some(Type::ManagedArray),
            _ => None,
        } && self.managed_source_type(node, depth) == Some(Type::ManagedValue)
            && self.stored.container(node.node_id()) == Some(ty)
        {
            work.push(Work::Managed(ManagedWork::ReferenceProjection { node, ty }));
            work.push(Work::Eval(node, depth, Mode::Managed));
            return Ok(true);
        }
        if let Expression::BinaryExpression(binary) = node
            && binary.operator == BinaryOperator::In
            && matches!(
                mode,
                Mode::Predicate | Mode::General | Mode::Managed | Mode::Number | Mode::Discard
            )
        {
            self.schedule_in(node, binary, depth, mode, work)?;
            return Ok(true);
        }
        if let Expression::CallExpression(call) = node
            && object_intrinsics::is_array_test(call, self.semantic)
            && matches!(
                mode,
                Mode::Predicate | Mode::General | Mode::Managed | Mode::Number | Mode::Discard
            )
        {
            boolean_consumer(node, mode, "Array.isArray produces a Boolean", work)?;
            if call.arguments.len() > 1000 {
                return Err(unsupported(
                    node,
                    "at most 1000 Array.isArray arguments are admitted",
                ));
            }
            let first = invocation::offset(call);
            work.push(Work::ArrayIsArray {
                node,
                has_input: first < call.arguments.len(),
            });
            // Only the tested argument leaves a value on the stack. Evaluate
            // the ignored thisArg and surplus arguments for their source effects.
            for (index, argument) in call.arguments.iter().enumerate().rev() {
                let expr = argument
                    .as_expression()
                    .ok_or_else(|| unsupported(node, "spread call arguments are not admitted"))?;
                let mode = if index == first {
                    Mode::Managed
                } else {
                    Mode::Discard
                };
                work.push(Work::Eval(expr, depth + 1, mode));
            }
            return Ok(true);
        }
        if let Expression::CallExpression(call) = node
            && object_intrinsics::is_method(call, self.semantic, "hasOwn")
            && matches!(
                mode,
                Mode::Predicate | Mode::General | Mode::Managed | Mode::Number | Mode::Discard
            )
        {
            self.schedule_has_own(node, call, depth, mode, work)?;
            return Ok(true);
        }
        if let Expression::CallExpression(call) = node
            && let Some((receiver, method)) = crate::lower::string_methods::call(call)
            && self.stored.string(receiver.node_id())
            && (matches!(mode, Mode::Managed)
                || (method.result() != Type::ManagedValue
                    && matches!(mode, Mode::Number | Mode::General | Mode::Predicate))
                || (method.result() == Type::Number && matches!(mode, Mode::Discard)))
        {
            if method.result() == Type::Number {
                if matches!(mode, Mode::Discard) {
                    work.push(Work::Forget);
                } else {
                    number_consumer(work, node, mode)?;
                }
            } else if method.result() == Type::Predicate {
                match mode {
                    Mode::Predicate => (),
                    Mode::General => work.push(Work::Pack(node)),
                    Mode::Managed => {
                        work.push(Work::Managed(ManagedWork::Primitive(node)));
                        work.push(Work::Pack(node));
                    }
                    _ => {
                        return Err(unsupported(
                            node,
                            "String search returns a Boolean, not a Number",
                        ));
                    }
                }
            }
            work.push(Work::StringMethod(super::string_methods::Task {
                node,
                call,
                method,
                receiver: None,
                inputs: [None; 2],
                fragments: Vec::new(),
                argument: 0,
                pending: false,
                depth,
            }));
            work.push(Work::Eval(receiver, depth + 1, Mode::Managed));
            return Ok(true);
        }
        if let Expression::CallExpression(call) = node
            && let Some((receiver, method)) = crate::lower::string_methods::call(call)
            && let Some(kind) = method.array_search()
            && self.receiver_type(receiver, depth + 1) == Some(Type::ManagedArray)
            && matches!(
                mode,
                Mode::Predicate | Mode::Number | Mode::General | Mode::Managed | Mode::Discard
            )
        {
            if kind == ir::ArraySearch::Includes {
                boolean_consumer(node, mode, "array includes produces a Boolean", work)?;
            } else if matches!(mode, Mode::Discard) {
                work.push(Work::Forget);
            } else {
                number_consumer(work, node, mode)?;
            }
            self.schedule_array_search(node, call, receiver, kind, depth, work)?;
            return Ok(true);
        }
        if matches!(mode, Mode::Managed)
            && let Expression::CallExpression(call) = node
            && let Some(receiver) = crate::lower::array_at::receiver(call)
            && self.receiver_type(receiver, depth + 1) == Some(Type::ManagedArray)
        {
            // Admitted arrays cannot override `at`, mutate Array.prototype, or
            // originate at a foreign-reference boundary. Other method calls keep
            // their ordinary call path; this is not a generic receiver cast.
            work.push(Work::ArrayAt(super::array_at::Task {
                node,
                call,
                array: None,
                index: None,
                argument: 0,
                depth,
            }));
            work.push(Work::Eval(receiver, depth + 1, Mode::Array));
            return Ok(true);
        }
        if let Expression::CallExpression(call) = node
            && let Some((operation, receiver)) = crate::lower::array_mutations::operation(call)
            && matches!(
                (operation, mode),
                (
                    crate::lower::array_mutations::Operation::Pop
                        | crate::lower::array_mutations::Operation::Shift,
                    Mode::Managed
                ) | (
                    crate::lower::array_mutations::Operation::Reverse
                        | crate::lower::array_mutations::Operation::ToReversed,
                    Mode::Array
                ) | (
                    crate::lower::array_mutations::Operation::Push
                        | crate::lower::array_mutations::Operation::Unshift,
                    Mode::Number | Mode::General | Mode::Predicate | Mode::Managed | Mode::Discard
                )
            )
            && self.receiver_type(receiver, depth + 1) == Some(Type::ManagedArray)
        {
            if matches!(
                operation,
                crate::lower::array_mutations::Operation::Push
                    | crate::lower::array_mutations::Operation::Unshift
            ) {
                if matches!(mode, Mode::Discard) {
                    work.push(Work::Forget);
                } else {
                    number_consumer(work, node, mode)?;
                }
            }
            work.push(Work::ArrayMutation(super::array_mutation::Task {
                operation,
                node,
                call,
                array: None,
                argument: 0,
                depth,
                push_values: Vec::new(),
                awaiting_push: false,
                push_arguments: None,
            }));
            work.push(Work::Eval(receiver, depth + 1, Mode::Array));
            return Ok(true);
        }
        if self.member_array(node, depth)
            && let Expression::AssignmentExpression(a) = node
            && let Some((object, key)) = assignment_member(node, true, false, depth)?
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
        ) && let Some((receiver, ty)) = self.managed_length_receiver(node, depth)
        {
            number_consumer(work, node, mode)?;
            if ty == Type::ManagedArray {
                work.push(Work::Managed(ManagedWork::ArrayLength(node)));
                work.push(Work::Eval(receiver, depth + 1, Mode::Array));
            } else {
                work.push(Work::Managed(ManagedWork::StringLength(node)));
                work.push(Work::Eval(receiver, depth + 1, Mode::Managed));
            }
            return Ok(true);
        }
        if matches!(mode, Mode::Predicate)
            && let Expression::BinaryExpression(binary) = node
            && self.stored.string(binary.left.node_id())
            && self.stored.string(binary.right.node_id())
            && let Some(operator) = match binary.operator {
                BinaryOperator::LessThan => Some(Relational::Less),
                BinaryOperator::LessEqualThan => Some(Relational::LessEqual),
                BinaryOperator::GreaterThan => Some(Relational::Greater),
                BinaryOperator::GreaterEqualThan => Some(Relational::GreaterEqual),
                _ => None,
            }
        {
            work.push(Work::Managed(ManagedWork::StringCompareRight {
                node,
                right: &binary.right,
                depth: depth + 1,
                operator,
            }));
            work.push(Work::Eval(&binary.left, depth + 1, Mode::Managed));
            return Ok(true);
        }
        if matches!(mode, Mode::Predicate)
            && let Expression::BinaryExpression(binary) = node
            && matches!(
                binary.operator,
                BinaryOperator::Equality | BinaryOperator::Inequality
            )
            && self.stored.primitive(binary.left.node_id())
            && self.stored.primitive(binary.right.node_id())
            && (self.managed_source_type(&binary.left, depth + 1).is_some()
                || self.managed_source_type(&binary.right, depth + 1).is_some())
        {
            work.push(Work::PrimitiveCompareRight {
                node,
                right: &binary.right,
                depth: depth + 1,
                loose: true,
                negated: binary.operator == BinaryOperator::Inequality,
            });
            work.push(Work::Eval(&binary.left, depth + 1, Mode::General));
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
            let kind = if matches!(
                binary.operator,
                BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
            ) || (self.stored.string(binary.left.node_id())
                && self.stored.string(binary.right.node_id()))
                || (self.stored.number(binary.left.node_id())
                    && self.stored.number(binary.right.node_id()))
                || (self.stored.boolean(binary.left.node_id())
                    && self.stored.boolean(binary.right.node_id()))
                || (self.stored.reference(binary.left.node_id())
                    && self.stored.reference(binary.right.node_id()))
            {
                // Same-kind Number/String/Boolean pairs and language-object reference
                // pairs need no primitive coercion for abstract equality.
                EqualityKind::Exact
            } else if self.stored.nullish(binary.left.node_id()) {
                EqualityKind::LeftNullish
            } else if self.stored.nullish(binary.right.node_id()) {
                EqualityKind::RightNullish
            } else {
                return Err(unsupported(
                    node,
                    "expression is outside the numeric experiment: managed comparisons require strict equality",
                ));
            };
            work.push(Work::Managed(ManagedWork::EqualRight {
                node,
                right: &binary.right,
                depth: depth + 1,
                negated: matches!(
                    binary.operator,
                    BinaryOperator::Inequality | BinaryOperator::StrictInequality
                ),
                kind,
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
        if matches!(mode, Mode::Array)
            && let Expression::CallExpression(call) = node
            && let Some((receiver, crate::lower::string_methods::Method::Slice)) =
                crate::lower::string_methods::call(call)
            && self.receiver_type(receiver, depth + 1) == Some(Type::ManagedArray)
        {
            work.push(Work::ArraySlice(super::array_slice::Task {
                node,
                call,
                array: None,
                inputs: [None; 2],
                argument: 0,
                pending: false,
                depth,
            }));
            work.push(Work::Eval(receiver, depth + 1, Mode::Array));
            return Ok(true);
        }
        if matches!(mode, Mode::Array)
            && let Expression::CallExpression(call) = node
            && let Some(kind) = object_intrinsics::enumeration(call, self.semantic)
        {
            work.push(Work::OwnKeys(super::own_keys::Task {
                kind,
                node,
                call,
                arguments: invocation::all_arguments(call),
                index: 0,
                input: None,
                awaiting: false,
                known: false,
                depth,
            }));
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
        if matches!(mode, Mode::Object | Mode::Array | Mode::Managed)
            && let Expression::CallExpression(call) = node
            && managed_calls::function(call, self.semantic, self.functions).is_none()
            && (self.managed_kinds.calls.contains_key(&call.node_id.get())
                || self
                    .closure_target(invocation::callee(call), invocation::depth(call, depth + 1))
                    .is_some_and(|closure| {
                        closure.checked
                            && !closure.factory
                            && !self.arrow(closure.arrow).r#async
                            && closure.result.is_managed()
                    })
                || self
                    .runtime_closure_callee(
                        invocation::callee(call),
                        invocation::depth(call, depth + 1),
                    )
                    .and_then(|(_, id)| self.closure_signatures.borrow().result(id))
                    .is_some_and(Type::is_managed))
        {
            return Ok(false);
        }
        if matches!(mode, Mode::Array) {
            if schedule_selection(work, node, depth, mode)? {
                return Ok(true);
            }
            if let Expression::ArrayExpression(literal) = node {
                if let Some(source) = iteration::single_spread(literal) {
                    let mode =
                        self.intrinsic_iterable_mode(source, depth + 1)
                            .ok_or_else(|| {
                                unsupported(
                                    source,
                                    "array copy spread requires a source-proven Array or String",
                                )
                            })?;
                    work.push(Work::Managed(ManagedWork::SpreadCopy(node)));
                    work.push(Work::Eval(source, depth + 1, mode));
                    return Ok(true);
                }
                let elements = iteration::literal_elements(literal).ok_or_else(|| {
                    unsupported(
                        node,
                        "mixed array spread requires fixed Array-literal operands",
                    )
                })?;
                let length = u32::try_from(elements.len())
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
                    elements,
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
                    work.push(Work::Managed(ManagedWork::Literal(
                        objects::ObjectWork::new(node, literal, object, depth),
                    )));
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
            if let Expression::BinaryExpression(binary) = node
                && binary.operator == BinaryOperator::Addition
                && self.stored.string(binary.left.node_id())
                && self.stored.string(binary.right.node_id())
            {
                work.push(Work::Managed(ManagedWork::ConcatRight {
                    node,
                    right: &binary.right,
                    depth: depth + 1,
                }));
                work.push(Work::Eval(&binary.left, depth + 1, Mode::Managed));
                return Ok(true);
            }
            if let Expression::UnaryExpression(u) = node
                && u.operator == UnaryOperator::Typeof
            {
                if typeof_name::unresolvable(&u.argument, self.semantic, depth + 1) {
                    // Only a reference to an absent name skips GetValue. A
                    // sequence, member access or resolved TDZ binding does not.
                    work.push(Work::Managed(ManagedWork::ConstantTypeof(
                        node,
                        "undefined",
                    )));
                    return Ok(true);
                }
                let named = closures::unwrapped_at(&u.argument, depth + 1)
                    .and_then(|(argument, _)| match argument {
                        Expression::Identifier(i) => i.reference_id.get(),
                        _ => None,
                    })
                    .and_then(|reference| {
                        self.semantic.scoping().get_reference(reference).symbol_id()
                    })
                    .is_some_and(|symbol| self.functions.iter().any(|f| f.symbol == symbol));
                if named {
                    // A hoisted module declaration has no read/construction effects.
                    // Its complete source body is still validated independently.
                    work.push(Work::Managed(ManagedWork::ConstantTypeof(node, "function")));
                } else if self.function_arity(&u.argument, depth + 1).is_some() {
                    work.push(Work::Managed(ManagedWork::ConstantTypeof(node, "function")));
                    work.push(Work::Eval(&u.argument, depth + 1, Mode::Discard));
                } else {
                    work.push(Work::Managed(ManagedWork::Typeof(node)));
                    work.push(Work::Eval(&u.argument, depth + 1, Mode::Managed));
                }
                return Ok(true);
            }
            if let Expression::TemplateLiteral(template) = node {
                if template.expressions.len() > 1000 {
                    return Err(unsupported(
                        node,
                        "at most 1000 template substitutions are admitted",
                    ));
                }
                work.push(Work::StringTemplate(super::string_template::Task {
                    node,
                    template,
                    result: None,
                    index: 0,
                    depth,
                }));
                return Ok(true);
            }
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
            if !matches!(
                node,
                Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)
            ) && let Some(signature) = self.function_arity(node, depth)
            {
                if self.closure_signatures.borrow().is_async(signature) {
                    return Err(unsupported(
                        node,
                        "suspended async functions cannot be stored as managed values",
                    ));
                }
                work.push(Work::Managed(ManagedWork::BoxReference(node)));
                work.push(Work::Eval(node, depth, Mode::Function(signature)));
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
                && let Some((object, key)) = member(
                    node,
                    self.member_array(node, depth),
                    self.member_dictionary(node),
                    depth,
                )?
            {
                let receiver = self.receiver_type(object, depth + 1);
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
            if let Some((object, key)) = assignment_member(
                node,
                self.member_array(node, depth),
                self.member_dictionary(node),
                depth,
            )? {
                let Expression::AssignmentExpression(a) = node else {
                    unreachable!()
                };
                if a.operator != AssignmentOperator::Assign {
                    return Err(unsupported(
                        node,
                        "managed properties require plain assignment",
                    ));
                }
                let array = self.receiver_type(object, depth + 1) == Some(Type::ManagedArray);
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
        if !matches!(mode, Mode::Function(_)) && self.managed_source_type(node, depth).is_some() {
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
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        match task {
            ManagedWork::StringIndexReceiver { node, index, depth } => {
                let input = values.pop().expect("String index receiver").id();
                work.push(Work::Managed(ManagedWork::StringIndex { node, input }));
                work.push(Work::Eval(index, depth, Mode::Number));
            }
            ManagedWork::StringIndex { node, input } => {
                let index = values.pop().expect("String index").id();
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedStringGetIndex {
                        result,
                        input,
                        index,
                    },
                );
                values.push(Value::Managed(result));
            }
            ManagedWork::OptionalRead(task) => {
                self.complete_optional_member(task, regions, values, work)
            }
            ManagedWork::DictionaryNew(node) => {
                let result = self.value(Type::ManagedObject);
                Self::push(regions.current(), node, Op::ManagedObjectNew { result });
                values.push(Value::Object(result));
            }
            ManagedWork::ConcatRight { node, right, depth } => {
                let left = values.pop().expect("String addition left operand").id();
                work.push(Work::Managed(ManagedWork::Concat { node, left }));
                work.push(Work::Eval(right, depth, Mode::Managed));
            }
            ManagedWork::Concat { node, left } => {
                let right = values.pop().expect("String addition right operand").id();
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedStringConcat {
                        result,
                        left,
                        right,
                    },
                );
                values.push(Value::Managed(result));
            }
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
            ManagedWork::SpreadCopy(node) => {
                let input = values.pop().expect("spread iterable").id();
                let result = self.value(Type::ManagedArray);
                let op = if self.values[input.0] == Type::ManagedArray {
                    Op::ManagedArrayRest {
                        result,
                        array: input,
                        start: 0,
                    }
                } else {
                    Op::ManagedStringToArray { result, input }
                };
                Self::push(regions.current(), node, op);
                values.push(Value::Array(result));
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
            ManagedWork::StringLength(node) => {
                let input = values.pop().expect("String length receiver").id();
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedStringLength { result, input },
                );
                values.push(Value::Number(result));
            }
            ManagedWork::ConstantTypeof(node, name) => {
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedString {
                        result,
                        units: name.encode_utf16().collect(),
                    },
                );
                values.push(Value::Managed(result));
            }
            ManagedWork::Typeof(node) => {
                let input = values.pop().expect("typeof operand").id();
                let result = self.value(Type::ManagedValue);
                Self::push(regions.current(), node, Op::ManagedTypeof { result, input });
                values.push(Value::Managed(result));
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
            ManagedWork::Literal(task) => {
                self.complete_object(task, regions, values, work)?;
            }
            ManagedWork::FunctionProjection { node, signature } => {
                let input = values.pop().expect("stored function value").id();
                let result = self.value(Type::Closure(signature));
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedToClosure { result, input },
                );
                values.push(Value::Function(result));
            }
            ManagedWork::NumberProjection(node) => {
                let input = values.pop().expect("boxed Number").id();
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedToNumber { result, input },
                );
                values.push(Value::Number(result));
            }
            ManagedWork::ReferenceProjection { node, ty } => {
                let input = values.pop().expect("boxed receiver").id();
                let result = self.value(ty);
                let op = match ty {
                    Type::ManagedObject => Op::ManagedToObject { result, input },
                    Type::ManagedArray => Op::ManagedToArray { result, input },
                    _ => unreachable!("checked receiver projection"),
                };
                Self::push(regions.current(), node, op);
                values.push(if ty == Type::ManagedObject {
                    Value::Object(result)
                } else {
                    Value::Array(result)
                });
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
                    Value::Function(input) => Op::ManagedFromClosure { result, input },
                    Value::General(input) => Op::ManagedPrimitive { result, input },
                    _ => unreachable!("checked managed bridge"),
                };
                Self::push(regions.current(), node, kind);
                values.push(Value::Managed(result));
            }
            ManagedWork::PrimitiveProjection(node) => {
                let input = values.pop().expect("managed primitive input").id();
                let result = self.value(Type::General);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedToPrimitive { result, input },
                );
                values.push(Value::General(result));
            }
            ManagedWork::StringCompareRight {
                node,
                right,
                depth,
                operator,
            } => {
                let left = values.pop().expect("String comparison left").id();
                work.push(Work::Managed(ManagedWork::StringCompare {
                    node,
                    left,
                    operator,
                }));
                work.push(Work::Eval(right, depth, Mode::Managed));
            }
            ManagedWork::StringCompare {
                node,
                left,
                operator,
            } => {
                let right = values.pop().expect("String comparison right").id();
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedStringCompare {
                        result,
                        operator,
                        left,
                        right,
                    },
                );
                values.push(Value::Predicate(result));
            }
            ManagedWork::EqualRight {
                node,
                right,
                depth,
                negated,
                kind,
            } => {
                let left = values.pop().expect("managed equality left").id();
                work.push(Work::Managed(ManagedWork::Equal {
                    node,
                    left,
                    negated,
                    kind,
                }));
                work.push(Work::Eval(right, depth, Mode::Managed));
            }
            ManagedWork::Equal {
                node,
                left,
                negated,
                kind,
            } => {
                let right = values.pop().expect("managed equality right").id();
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    match kind {
                        EqualityKind::Exact => Op::ManagedEqual {
                            result,
                            left,
                            right,
                        },
                        // Abstract equality with a positively nullish operand is
                        // exactly a nullish test of the other value. Both operands
                        // still evaluate left-to-right, even when one is `void`.
                        EqualityKind::LeftNullish => Op::ManagedIsNullish {
                            result,
                            input: right,
                        },
                        EqualityKind::RightNullish => Op::ManagedIsNullish {
                            result,
                            input: left,
                        },
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
