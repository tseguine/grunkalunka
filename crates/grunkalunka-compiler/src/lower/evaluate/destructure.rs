//! Assignment patterns consume one saved RHS and update existing lexical cells.
use super::*;
use oxc_ast::ast::{AssignmentTarget, AssignmentTargetProperty};

pub(super) enum Selection<'e, 'a> {
    Index(u32),
    Field(Vec<u16>),
    Computed(&'e Expression<'a>, bool),
    SavedField(ir::ManagedKey),
    ArrayRest(u32),
    ObjectRest,
}

pub(super) enum Destination<'e, 'a> {
    Slot(SlotId),
    Member {
        object: &'e Expression<'a>,
        key: iteration::PropertyKey<'e, 'a>,
        ty: Type,
    },
}

pub(super) enum SavedDestination {
    Slot(SlotId),
    Member {
        object: ValueId,
        key: ir::ManagedKey,
    },
}

struct Write<'e, 'a> {
    selection: Selection<'e, 'a>,
    destination: Destination<'e, 'a>,
    default: Option<&'e Expression<'a>>,
}

pub(super) struct Plan<'e, 'a> {
    node: &'e Expression<'a>,
    writes: Vec<Write<'e, 'a>>,
    mode: Mode,
    depth: usize,
    string_array: bool,
}

pub(super) struct State<'e, 'a> {
    node: &'e Expression<'a>,
    writes: std::vec::IntoIter<Write<'e, 'a>>,
    ready: Option<Write<'e, 'a>>,
    input: Value,
    receiver: ValueId,
    mode: Mode,
    depth: usize,
    iterator: Option<control_cursor::array_bindings::IteratorState>,
    next_index: u32,
    excluded: Vec<ir::ManagedKey>,
    pending: Option<(
        Selection<'e, 'a>,
        SavedDestination,
        Option<&'e Expression<'a>>,
    )>,
}

pub(super) enum Task<'e, 'a> {
    Begin(Plan<'e, 'a>),
    Receiver {
        state: Box<State<'e, 'a>>,
        selection: Selection<'e, 'a>,
        key: iteration::PropertyKey<'e, 'a>,
        default: Option<&'e Expression<'a>>,
    },
    Key {
        state: Box<State<'e, 'a>>,
        selection: Selection<'e, 'a>,
        object: ValueId,
        uint32: bool,
        default: Option<&'e Expression<'a>>,
    },
    SourceKey {
        state: Box<State<'e, 'a>>,
        destination: Destination<'e, 'a>,
        uint32: bool,
        default: Option<&'e Expression<'a>>,
    },
    Default {
        state: Box<State<'e, 'a>>,
        destination: SavedDestination,
        selected: ValueId,
        condition: ValueId,
    },
}

impl Lowering<'_, '_> {
    fn assignment_pattern_destination<'e, 'a>(
        &self,
        target: &'e AssignmentTarget<'a>,
        depth: usize,
    ) -> Result<Destination<'e, 'a>, String> {
        if let Some((object, key)) = assignment_patterns::property_target(target, depth) {
            let ty = self
                .receiver_type(object, depth)
                .filter(|ty| matches!(ty, Type::ManagedObject | Type::ManagedArray))
                .ok_or_else(|| {
                    unsupported(
                        target,
                        "assignment member requires a proven object or Array",
                    )
                })?;
            let key = match key {
                iteration::PropertyKey::Literal(key) => {
                    if ty == Type::ManagedArray && key.as_slice() == [108, 101, 110, 103, 116, 104]
                    {
                        return Err(unsupported(
                            target,
                            "assignment patterns cannot set unchecked Array length",
                        ));
                    }
                    iteration::PropertyKey::Literal(managed::member_key(
                        target,
                        key,
                        ty == Type::ManagedArray,
                        self.stored.dictionary(object.node_id()),
                    )?)
                }
                iteration::PropertyKey::Runtime(key) => {
                    if !self.stored.uint32_keys.contains(key, depth)
                        && !(self.stored.dictionary(object.node_id())
                            && self.stored.string(key.node_id()))
                    {
                        return Err(unsupported(
                            target,
                            "assignment member runtime keys require unsigned syntax or a dictionary String",
                        ));
                    }
                    iteration::PropertyKey::Runtime(key)
                }
            };
            return Ok(Destination::Member { object, key, ty });
        }
        let identifier = target
            .as_simple_assignment_target()
            .map(|target| target_identifier(target, 0))
            .transpose()?
            .flatten()
            .ok_or_else(|| unsupported(target, "assignment patterns require identifier targets"))?;
        self.assignment_identifier_slot(identifier)
            .map(Destination::Slot)
    }

    fn assignment_identifier_slot(
        &self,
        identifier: &IdentifierReference<'_>,
    ) -> Result<SlotId, String> {
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        let compatible = match self.slots[binding.slot.0].ty {
            Type::ManagedValue => true,
            Type::Number => self.stored.number(identifier.node_id()),
            Type::General => self.stored.primitive(identifier.node_id()),
            _ => false,
        };
        if !binding.mutable || !compatible {
            return Err(unsupported(
                identifier,
                "assignment target must be an initialized identifier with mutable managed-value storage",
            ));
        }
        Ok(binding.slot)
    }

    fn store_assignment_selection(
        &mut self,
        node: &Expression<'_>,
        slot: SlotId,
        value: ValueId,
        region: &mut Region,
    ) {
        let value = match self.slots[slot.0].ty {
            Type::ManagedValue => value,
            ty @ (Type::Number | Type::General) => {
                let primitive = self.value(Type::General);
                Self::push(
                    region,
                    node,
                    Op::ManagedToPrimitive {
                        result: primitive,
                        input: value,
                    },
                );
                if ty == Type::Number {
                    let number = self.value(Type::Number);
                    Self::push(
                        region,
                        node,
                        Op::NumberFromGeneral {
                            result: number,
                            input: primitive,
                        },
                    );
                    number
                } else {
                    primitive
                }
            }
            _ => unreachable!("validated assignment cell"),
        };
        Self::push(region, node, Op::Store { slot, value });
    }

    fn store_pattern_destination(
        &mut self,
        node: &Expression<'_>,
        destination: SavedDestination,
        value: ValueId,
        region: &mut Region,
    ) {
        match destination {
            SavedDestination::Slot(slot) => {
                self.store_assignment_selection(node, slot, value, region)
            }
            SavedDestination::Member { object, key } => Self::push(
                region,
                node,
                match key {
                    ir::ManagedKey::Literal(key) if self.values[object.0] == Type::ManagedArray => {
                        Op::ManagedArraySetOwn {
                            array: object,
                            key,
                            value,
                        }
                    }
                    ir::ManagedKey::Literal(key) => Op::ManagedSetOwn { object, key, value },
                    key => Op::ManagedSetOwnDynamic { object, key, value },
                },
            ),
        }
    }

    pub(super) fn schedule_destructure<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::AssignmentExpression(assignment) = node else {
            return Ok(false);
        };
        if assignment.left.as_assignment_target_pattern().is_none() {
            return Ok(false);
        }
        if depth >= 128 {
            return Err(unsupported(node, "assignment pattern nesting exceeds 128"));
        }
        if assignment.operator != AssignmentOperator::Assign {
            return Err(unsupported(
                node,
                "assignment patterns require plain assignment",
            ));
        }
        let string = self.stored.string(assignment.right.node_id());
        let object_primitive = self.stored.object_primitive(assignment.right.node_id())
            || self.numeric_result_shape(&assignment.right, depth + 1)
            || self.operand_boolean_shape(&assignment.right, depth + 1);
        let receiver = if string {
            Some(Type::ManagedValue)
        } else if object_primitive {
            Some(Type::General)
        } else {
            self.receiver_type(&assignment.right, depth + 1)
        };
        let input_mode = match receiver {
            Some(Type::ManagedValue) if string => Mode::Managed,
            Some(Type::ManagedArray) => Mode::Array,
            Some(Type::ManagedObject) => Mode::Object,
            Some(Type::General) => Mode::General,
            _ => {
                return Err(unsupported(
                    node,
                    "assignment patterns require a source-proven Array, object, Number or Boolean",
                ));
            }
        };
        let compatible_result = if receiver == Some(Type::General) {
            matches!(
                mode,
                Mode::General | Mode::Managed | Mode::Predicate | Mode::Discard
            ) || (matches!(mode, Mode::Number)
                && (self.stored.number(assignment.right.node_id())
                    || self.numeric_result_shape(&assignment.right, depth + 1)))
        } else {
            matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard)
                || matches!(
                    (mode, receiver),
                    (Mode::Array, Some(Type::ManagedArray))
                        | (Mode::Object, Some(Type::ManagedObject))
                )
        };
        if !compatible_result {
            return Err(unsupported(
                node,
                "assignment pattern results require a compatible managed position",
            ));
        }
        let mut writes = Vec::new();
        match &assignment.left {
            AssignmentTarget::ArrayAssignmentTarget(pattern) => {
                if receiver != Some(Type::ManagedArray) && !string {
                    return Err(unsupported(
                        node,
                        "array assignments require a source-proven Array",
                    ));
                }
                for (index, element) in pattern.elements.iter().enumerate() {
                    if let Some(element) = element {
                        let (target, default) = assignment_patterns::target_and_default(element);
                        let index = u32::try_from(index).map_err(|_| {
                            unsupported(node, "array assignment exceeds index limit")
                        })?;
                        writes.push(Write {
                            selection: Selection::Index(index),
                            destination: self.assignment_pattern_destination(target, depth + 1)?,
                            default,
                        });
                    }
                }
                if let Some(rest) = &pattern.rest {
                    let start = u32::try_from(pattern.elements.len())
                        .map_err(|_| unsupported(node, "array assignment exceeds index limit"))?;
                    writes.push(Write {
                        selection: Selection::ArrayRest(start),
                        destination: self
                            .assignment_pattern_destination(&rest.target, depth + 1)?,
                        default: None,
                    });
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(pattern) => {
                let dictionary = self.stored.dictionary(assignment.right.node_id());
                for property in &pattern.properties {
                    let (key, destination, default) = match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(p) => (
                            iteration::PropertyKey::Literal(
                                p.binding.name.encode_utf16().collect(),
                            ),
                            Destination::Slot(self.assignment_identifier_slot(&p.binding)?),
                            p.init.as_ref(),
                        ),
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => {
                            let key = static_keys::property(&p.name, p.computed, depth + 1)
                                .map(iteration::PropertyKey::Literal)
                                .or_else(|| {
                                    p.name.as_expression().map(iteration::PropertyKey::Runtime)
                                })
                                .ok_or_else(|| {
                                    unsupported(
                                        p.as_ref(),
                                        "object assignment requires an admitted property key",
                                    )
                                })?;
                            let (target, default) =
                                assignment_patterns::target_and_default(&p.binding);
                            (
                                key,
                                self.assignment_pattern_destination(target, depth + 1)?,
                                default,
                            )
                        }
                    };
                    let selection = match key {
                        iteration::PropertyKey::Literal(key) => {
                            let key = managed::member_key(
                                property,
                                key,
                                receiver == Some(Type::ManagedArray),
                                dictionary,
                            )?;
                            if receiver == Some(Type::General)
                                && binding_patterns::string_index(&key).is_none()
                            {
                                return Err(unsupported(
                                    node,
                                    "Number and Boolean object assignment keys require canonical numeric indices",
                                ));
                            }
                            if string
                                && key.as_slice() != [108, 101, 110, 103, 116, 104]
                                && binding_patterns::string_index(&key).is_none()
                            {
                                return Err(unsupported(
                                    node,
                                    "String object assignment keys require length or canonical numeric indices",
                                ));
                            }
                            Selection::Field(key)
                        }
                        iteration::PropertyKey::Runtime(key) => {
                            let uint32 = self.stored.uint32_keys.contains(key, depth + 1);
                            let string_key = self.stored.string(key.node_id())
                                && (dictionary
                                    || (receiver == Some(Type::ManagedObject)
                                        && static_keys::own_string_name(
                                            key,
                                            self.semantic,
                                            depth + 1,
                                        )));
                            if !uint32 && !string_key {
                                return Err(unsupported(
                                    node,
                                    "computed assignment source keys require an unsigned index or a proven record name",
                                ));
                            }
                            Selection::Computed(key, uint32)
                        }
                    };
                    writes.push(Write {
                        selection,
                        destination,
                        default,
                    });
                }
                if let Some(rest) = &pattern.rest {
                    writes.push(Write {
                        selection: Selection::ObjectRest,
                        destination: self
                            .assignment_pattern_destination(&rest.target, depth + 1)?,
                        default: None,
                    });
                }
            }
            _ => unreachable!("checked assignment pattern"),
        }
        work.push(Work::Destructure(Task::Begin(Plan {
            node,
            writes,
            mode,
            depth: depth + 1,
            string_array: string
                && matches!(assignment.left, AssignmentTarget::ArrayAssignmentTarget(_)),
        })));
        work.push(Work::Eval(&assignment.right, depth + 1, input_mode));
        Ok(true)
    }

    pub(super) fn resume_destructure<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let mut state = match task {
            Task::Begin(plan) => {
                let input = values.pop().expect("assignment RHS");
                let receiver = if plan.string_array {
                    // Select code points from a private immutable-String snapshot,
                    // but keep the original String as the assignment result.
                    let array = self.value(Type::ManagedArray);
                    Self::push(
                        regions.current(),
                        plan.node,
                        Op::ManagedStringToArray {
                            result: array,
                            input: input.id(),
                        },
                    );
                    array
                } else {
                    input.id()
                };
                let iterator = (self.values[receiver.0] == Type::ManagedArray
                    && plan.writes.iter().any(|write| {
                        write.default.is_some()
                            || matches!(write.destination, Destination::Member { .. })
                    }))
                .then(|| self.begin_array_binding_iterator(plan.node, regions.current()));
                Box::new(State {
                    node: plan.node,
                    writes: plan.writes.into_iter(),
                    ready: None,
                    input,
                    receiver,
                    mode: plan.mode,
                    depth: plan.depth,
                    iterator,
                    next_index: 0,
                    excluded: Vec::new(),
                    pending: None,
                })
            }
            Task::Receiver {
                mut state,
                selection,
                key,
                default,
            } => {
                let object = values.pop().expect("assignment member receiver").id();
                match key {
                    iteration::PropertyKey::Literal(key) => {
                        state.pending = Some((
                            selection,
                            SavedDestination::Member {
                                object,
                                key: ir::ManagedKey::Literal(key),
                            },
                            default,
                        ));
                        state
                    }
                    iteration::PropertyKey::Runtime(key) => {
                        let depth = state.depth;
                        let uint32 = self.stored.uint32_keys.contains(key, depth);
                        work.push(Work::Destructure(Task::Key {
                            state,
                            selection,
                            object,
                            uint32,
                            default,
                        }));
                        work.push(Work::Eval(
                            key,
                            depth,
                            if uint32 { Mode::Number } else { Mode::Managed },
                        ));
                        return Ok(());
                    }
                }
            }
            Task::Key {
                mut state,
                selection,
                object,
                uint32,
                default,
            } => {
                let key = values.pop().expect("assignment member key").id();
                let key = if uint32 {
                    ir::ManagedKey::Uint32(key)
                } else {
                    ir::ManagedKey::String(key)
                };
                state.pending =
                    Some((selection, SavedDestination::Member { object, key }, default));
                state
            }
            Task::SourceKey {
                mut state,
                destination,
                uint32,
                default,
            } => {
                let key = values.pop().expect("assignment source property key").id();
                let key = if uint32 {
                    ir::ManagedKey::Uint32(key)
                } else {
                    ir::ManagedKey::String(key)
                };
                state.ready = Some(Write {
                    selection: Selection::SavedField(key),
                    destination,
                    default,
                });
                state
            }
            Task::Default {
                state,
                destination,
                selected,
                condition,
            } => {
                let default = values.pop().expect("assignment default").id();
                let consequent = regions.finish(default);
                let mut alternate = self.region(state.node);
                alternate.exit = Exit::Yield(selected);
                let value = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    state.node,
                    Op::If {
                        result: Some(value),
                        condition,
                        consequent,
                        alternate,
                    },
                );
                self.store_pattern_destination(state.node, destination, value, regions.current());
                state
            }
        };
        let node = state.node;
        let receiver = state.receiver;
        loop {
            let (selection, destination, default) = if let Some(pending) = state.pending.take() {
                pending
            } else {
                let Some(Write {
                    selection,
                    destination,
                    default,
                }) = state.ready.take().or_else(|| state.writes.next())
                else {
                    break;
                };
                if let Selection::Computed(key, uint32) = selection {
                    let depth = state.depth;
                    work.push(Work::Destructure(Task::SourceKey {
                        state,
                        destination,
                        uint32,
                        default,
                    }));
                    work.push(Work::Eval(
                        key,
                        depth,
                        if uint32 { Mode::Number } else { Mode::Managed },
                    ));
                    return Ok(());
                }
                match &selection {
                    Selection::Field(key) => {
                        state.excluded.push(ir::ManagedKey::Literal(key.clone()))
                    }
                    Selection::SavedField(key) => state.excluded.push(key.clone()),
                    _ => (),
                }
                match destination {
                    Destination::Slot(slot) => (selection, SavedDestination::Slot(slot), default),
                    Destination::Member { object, key, ty } => {
                        // Elisions precede evaluation of the next original reference.
                        // They can exhaust the iterator before that reference grows it.
                        if let Selection::Index(index) | Selection::ArrayRest(index) = &selection
                            && *index > state.next_index
                            && let Some(iterator) = &mut state.iterator
                        {
                            self.select_array_binding_step(
                                node,
                                regions.current(),
                                receiver,
                                iterator,
                                index - 1,
                            );
                            state.next_index = *index;
                        }
                        let depth = state.depth;
                        work.push(Work::Destructure(Task::Receiver {
                            state,
                            selection,
                            key,
                            default,
                        }));
                        work.push(Work::Eval(object, depth, Mode::managed(ty)));
                        return Ok(());
                    }
                }
            };
            let value = match selection {
                Selection::Index(index) => {
                    state.next_index = index + 1;
                    if let Some(iterator) = &mut state.iterator {
                        self.select_array_binding_step(
                            node,
                            regions.current(),
                            receiver,
                            iterator,
                            index,
                        )
                    } else {
                        self.read_array_binding_element(node, regions.current(), receiver, index)
                    }
                }
                Selection::Field(key) => {
                    if self.values[receiver.0] == Type::General {
                        self.binding_undefined(node, regions.current())
                    } else {
                        let result = self.value(Type::ManagedValue);
                        let op = if self.values[receiver.0] == Type::ManagedValue {
                            if key.as_slice() == [108, 101, 110, 103, 116, 104] {
                                let number = self.value(Type::Number);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::ManagedStringLength {
                                        result: number,
                                        input: receiver,
                                    },
                                );
                                let packed = self.value(Type::General);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::Pack {
                                        result: packed,
                                        input: number,
                                    },
                                );
                                Op::ManagedPrimitive {
                                    result,
                                    input: packed,
                                }
                            } else {
                                let index = self.value(Type::Number);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::Constant {
                                        result: index,
                                        value: f64::from(
                                            binding_patterns::string_index(&key)
                                                .expect("planned String assignment index"),
                                        ),
                                    },
                                );
                                Op::ManagedStringGetIndex {
                                    result,
                                    input: receiver,
                                    index,
                                }
                            }
                        } else if self.values[receiver.0] == Type::ManagedArray {
                            Op::ManagedArrayGetOwn {
                                result,
                                array: receiver,
                                key,
                            }
                        } else {
                            Op::ManagedGetOwn {
                                result,
                                object: receiver,
                                key,
                            }
                        };
                        Self::push(regions.current(), node, op);
                        result
                    }
                }
                Selection::Computed(..) => unreachable!("source key resolved before destination"),
                Selection::SavedField(key) => {
                    if self.values[receiver.0] == Type::General {
                        self.binding_undefined(node, regions.current())
                    } else {
                        let result = self.value(Type::ManagedValue);
                        Self::push(
                            regions.current(),
                            node,
                            if self.values[receiver.0] == Type::ManagedValue {
                                let ir::ManagedKey::Uint32(index) = key else {
                                    unreachable!("validated String assignment index")
                                };
                                Op::ManagedStringGetIndex {
                                    result,
                                    input: receiver,
                                    index,
                                }
                            } else {
                                Op::ManagedGetOwnDynamic {
                                    result,
                                    object: receiver,
                                    key,
                                }
                            },
                        );
                        result
                    }
                }
                Selection::ArrayRest(start) => {
                    let rest = self.collect_array_binding_rest(
                        node,
                        regions.current(),
                        receiver,
                        state.iterator.as_ref(),
                        start,
                    );
                    let result = self.value(Type::ManagedValue);
                    Self::push(
                        regions.current(),
                        node,
                        Op::ManagedFromArray {
                            result,
                            input: rest,
                        },
                    );
                    result
                }
                Selection::ObjectRest => {
                    // String own indices are enumerable UTF-16 data properties.
                    // Materialize them only after the saved destination executes.
                    let object = if matches!(
                        self.values[receiver.0],
                        Type::ManagedValue | Type::General | Type::Number | Type::Predicate
                    ) {
                        let primitive = match self.values[receiver.0] {
                            Type::ManagedValue => receiver,
                            Type::General => {
                                let boxed = self.value(Type::ManagedValue);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::ManagedPrimitive {
                                        result: boxed,
                                        input: receiver,
                                    },
                                );
                                boxed
                            }
                            Type::Number | Type::Predicate => {
                                let packed = self.value(Type::General);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::Pack {
                                        result: packed,
                                        input: receiver,
                                    },
                                );
                                let boxed = self.value(Type::ManagedValue);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::ManagedPrimitive {
                                        result: boxed,
                                        input: packed,
                                    },
                                );
                                boxed
                            }
                            _ => unreachable!("validated primitive object assignment"),
                        };
                        let object = self.value(Type::ManagedObject);
                        Self::push(
                            regions.current(),
                            node,
                            Op::ManagedObjectNew { result: object },
                        );
                        Self::push(
                            regions.current(),
                            node,
                            Op::ManagedObjectSpread {
                                object,
                                input: primitive,
                            },
                        );
                        object
                    } else {
                        receiver
                    };
                    let rest = self.value(Type::ManagedObject);
                    Self::push(
                        regions.current(),
                        node,
                        Op::ManagedObjectRest {
                            result: rest,
                            object,
                            excluded: std::mem::take(&mut state.excluded),
                        },
                    );
                    let result = self.value(Type::ManagedValue);
                    Self::push(
                        regions.current(),
                        node,
                        Op::ManagedFromObject {
                            result,
                            input: rest,
                        },
                    );
                    result
                }
            };
            if let Some(default) = default {
                let undefined = self.binding_undefined(node, regions.current());
                let condition = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedEqual {
                        result: condition,
                        left: value,
                        right: undefined,
                    },
                );
                let child = self.region(default);
                regions.children.push(child);
                let depth = state.depth;
                work.push(Work::Destructure(Task::Default {
                    state,
                    destination,
                    selected: value,
                    condition,
                }));
                work.push(Work::Eval(default, depth, Mode::Managed));
                return Ok(());
            }
            self.store_pattern_destination(node, destination, value, regions.current());
        }
        match state.mode {
            Mode::Array | Mode::Object => values.push(state.input),
            Mode::General => values.push(state.input),
            Mode::Number => {
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::NumberFromGeneral {
                        result,
                        input: state.input.id(),
                    },
                );
                values.push(Value::Number(result));
            }
            Mode::Managed if self.values[state.input.id().0] == Type::ManagedValue => {
                values.push(state.input)
            }
            Mode::Managed => {
                let result = self.value(Type::ManagedValue);
                let op = match self.values[state.input.id().0] {
                    Type::General => Op::ManagedPrimitive {
                        result,
                        input: state.input.id(),
                    },
                    Type::ManagedArray => Op::ManagedFromArray {
                        result,
                        input: state.input.id(),
                    },
                    Type::ManagedObject => Op::ManagedFromObject {
                        result,
                        input: state.input.id(),
                    },
                    _ => unreachable!("validated managed assignment result"),
                };
                Self::push(regions.current(), node, op);
                values.push(Value::Managed(result));
            }
            Mode::Predicate => {
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    match self.values[state.input.id().0] {
                        Type::ManagedValue => Op::ManagedTruthy {
                            result,
                            input: state.input.id(),
                        },
                        Type::General => Op::GeneralTruthy {
                            result,
                            input: state.input.id(),
                        },
                        _ => Op::Boolean {
                            result,
                            value: true,
                        },
                    },
                );
                values.push(Value::Predicate(result));
            }
            Mode::Discard => (),
            _ => unreachable!("validated array assignment result"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn string_object_assignments_keep_property_result_and_kind_contracts() {
        for body in [
            "let n=0;({length:n}='abc');return n;",
            "let x='';({0:x='?'}='abc');return x.charCodeAt(0);",
            "let x;const result=({0:x}='abc');return result.length;",
            "let x;return ({}='')?1:0;",
            "let x;({length:x}='abc');return a;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let result = crate::compile(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
        for body in [
            "let x;({length:x}=(a as string));return a;",
            "let x;({length:x}=(a?'abc':null));return a;",
            "let x;({slice:x}='abc');return a;",
            "let x;({'01':x}='abc');return a;",
            "let x;const key='length';({[key]:x}='abc');return a;",
            "const x=0;({length:x}='abc');return a;",
            "let x=0;({0:x}='abc');return x;",
        ] {
            assert!(
                crate::compile(&format!("export function run(a,b){{{body}}}"), true).is_err(),
                "{body}"
            );
        }
        let js = "export function run(a,b){let n=0;({length:n}='abc');return n;}";
        let ts = "export function run(a:boolean,b:string):never{let n:string=0;({length:n}=('abc' as never));return n as boolean;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
    #[test]
    fn string_array_assignments_preserve_result_kinds_and_source_guards() {
        for body in [
            "let x;[x]='abc';return x==='a'?1:0;",
            "let x;return ([x]='')?1:0;",
            "let x;return ([x]='abc')?1:0;",
            "let x;const result=([x]='abc');return result.length;",
            "let x,rest=[];[x,...rest]='abc';return rest.length;",
            "let x;const read=()=>x;[x]='abc';return read()==='a'?1:0;",
            "var x;[x]='abc';return x==='a'?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let result = crate::compile(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
        for body in [
            "let x;[x]=(a as string);return a;",
            "let x;[x]=(a?'abc':null);return a;",
            "const x='';[x]='abc';return a;",
            "let x;[[x]]='abc';return a;",
            "[missing]='abc';return a;",
            "let x=3;[x]='abc';return x;",
            "let x=3;const read=()=>x;[x]='abc';return read();",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, true).is_err(), "{source}");
        }
        let js = "export function run(a,b){let x;const result=([x]='abc');return result.length;}";
        let ts = "export function run(a:boolean,b:string):never{let x:number;const result:boolean=([x]=('abc' as never));return (result as number).length;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }

    #[test]
    fn assignment_defaults_keep_lazy_selection_and_source_kind_guards() {
        for (body, admitted) in [
            ("let x='';[x='text']=[];return x.length;", true),
            ("let x='';({x='text'}={});return x.length;", true),
            ("let x='';({name:x='text'}={});return x.length;", true),
            ("let x=0,y=0;[x=a,y=x]=[];return y===a?1:0;", true),
            ("let x='';[x=1]=[];return x.length;", false),
            ("let x='';({x=1}={});return x.length;", false),
            ("let x='';[x='text']=[null];return x.length;", false),
            ("let x='';({x='text'}={x:null});return x.length;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn nested_assignment_defaults_keep_controlled_depth_limits() {
        for (depth, root) in [6, 140]
            .into_iter()
            .flat_map(|depth| ["[]", "''"].into_iter().map(move |root| (depth, root)))
        {
            let mut expression = String::from(root);
            for _ in 0..depth {
                expression = format!("[x=({expression})]={root}");
            }
            let source = format!("export function run(a,b){{let x=0;{expression};return a;}}");
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || crate::compile(&source, false))
                .unwrap()
                .join()
                .expect("assignment defaults must not exhaust the native stack");
            if depth == 6 {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                let error = result.err().expect("over-depth defaults must reject");
                assert!(
                    error.contains("nesting") || error.contains("limit"),
                    "{error}"
                );
            }
        }
    }

    #[test]
    fn assignment_patterns_plan_boxed_local_cells_without_annotation_proofs() {
        for (body, admitted) in [
            ("let x=0,y=0;[x,y]=[a,b];return x===a&&y===b?1:0;", true),
            ("let x,y;({x,y}={x:a,y:b});return x===a&&y===b?1:0;", true),
            ("let tail=[];[,...tail]=[a,b];return tail.length;", true),
            (
                "let rest={};({...rest}={name:'text'});return rest.name==='text'?1:0;",
                true,
            ),
            (
                "let x=0;const read=()=>x;[x]=['text'];return read()==='text'?1:0;",
                true,
            ),
            (
                "let x;const before=x===undefined;[x]=[a];return before&&x===a?1:0;",
                true,
            ),
            ("let x=0;[x]=['text'];return x.length;", false),
            ("let x=0;[x,y]=[a,b];let y=0;return a;", false),
            ("const x=0;[x]=[a];return a;", false),
            ("[a]=[b];return a;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        assert!(crate::compile("export function run(a:number,b:number):number{let x:number=0;[x]=['text'];return x==='text'?1:0;}", true).is_ok());
    }

    #[test]
    fn object_assignment_admission_and_provenance() {
        for (body, admitted) in [
            ("let x='';({x}={x:'text'});return x.length;", true),
            ("let x='';({name:x}={name:'text'});return x.length;", true),
            (
                "let x='';const d=Object.create(null);d.constructor='text';({constructor:x}=d);return x==='text'?1:0;",
                true,
            ),
            (
                "let x='';const d=Object.create(null);d.constructor='text';({constructor:x}=d);return x.length;",
                false,
            ),
            ("let [x]=[];({0:x}=[a]);return x===a?1:0;", true),
            (
                "let {...rest}={};({...rest}={name:'text'});return rest.name==='text'?1:0;",
                true,
            ),
            ("let x='';({x}={x:a});return x.length;", false),
            ("const x='';({x}={x:'text'});return a;", false),
            ("let x='';({x}=null);return a;", false),
            ("let x='';({constructor:x}={});return a;", false),
            ("let x='';({x='text'}={});return a;", true),
            (
                "let x='';const key='x';({[key]:x}={x:'text'});return a;",
                true,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn primitive_object_assignments_preserve_rhs_results_and_guard_fields() {
        for (body, admitted) in [
            ("({}=a);return b;", true),
            (
                "let rest={};({...rest}=a);return Object.keys(rest).length;",
                true,
            ),
            ("return ({}=a);", true),
            ("return ({}=a)?1:0;", true),
            ("const box={value:({}=a)};return +box.value;", true),
            ("return ({}=false)===false?1:0;", true),
            (
                "const box={};({...box.rest}=a);return Object.hasOwn(box,'rest')?1:0;",
                true,
            ),
            ("let n=0;let rest={};({...rest}=(n++,a));return n;", true),
            ("let x;({x}=a);return x;", false),
            ("let rest={};({...rest}=null);return a;", false),
            ("let rest={};({...rest}=(a?b:null));return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let result = crate::compile(&source, ts);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){let x;({x}=(a as object));return b;}",
                true
            )
            .is_err()
        );
    }

    #[test]
    fn array_assignment_admission_and_provenance() {
        for (body, admitted) in [
            ("let x='';[x]=['text'];return x.length;", true),
            ("let [x,y]=[];[x,y]=[a,b];return x===a&&y===b?1:0;", true),
            ("let [tail]=[[]];[,...tail]=[a,b];return tail.length;", true),
            (
                "let x='';const source=['text'];return ([x]=source)===source?1:0;",
                true,
            ),
            ("let x='';if([x]=['x'])return x.length;return 0;", true),
            ("let x='';[x]=[a];return x.length;", false),
            ("const x='';[x]=['text'];return a;", false),
            ("let x='';[x]=a;return a;", false),
            ("[missing]=['text'];return a;", false),
            ("let x='';[x='default']=[];return a;", true),
            ("let x='';[[x]]=[['text']];return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}

#[cfg(test)]
mod scalar_parameter_tests {
    #[test]
    fn scalar_pattern_parameters_require_source_and_storage_proofs() {
        for (body, ok) in [
            ("[a]=[b];return a;", true),
            ("({x:a}={x:b});return a;", true),
            ("[a=b]=[];return a;", true),
            ("({x:a=b}={});return a;", true),
            ("[a]=['text'];return a;", false),
            ("[a]=[{}];return a;", false),
            ("[...a]=[b];return a;", false),
            ("[a]=[];return a;", false),
            ("const x=0;[x]=[a];return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a:number,b:number){[a]=(['x'] as number[]);return a;}",
                true
            )
            .is_err()
        );
    }
    #[test]
    fn scalar_pattern_parameter_rhs_depth_is_bounded_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (n, ok) in [(126, true), (127, false)] {
                    let source = format!(
                        "export function run(a,b){{[a]={}[b]{};return a;}}",
                        "(".repeat(n),
                        ")".repeat(n)
                    );
                    let r = crate::compile(&source, false);
                    assert_eq!(r.is_ok(), ok, "{n}: {:?}", r.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod member_tests {
    #[test]
    fn member_pattern_destinations_require_owned_receiver_and_literal_keys() {
        for (body, ok) in [
            ("const o={x:a};[o.x]=[b];return o.x;", true),
            ("const xs=[];({x:xs[0]}={x:a});return xs.length;", true),
            ("const o={x:a};[o.x=b]=[];return o.x;", true),
            ("[a.x]=[b];return a;", false),
            ("const o=a?{}:null;[o.x]=[b];return a;", false),
            ("const o={};[o.__proto__]=[b];return a;", false),
            ("const xs=[];[xs.length]=[b];return a;", false),
            ("const o={};const k='x';[o[k]]=[b];return a;", false),
            ("const o={};[...o.x]=[b];return a;", true),
            ("const o={};({...o.x}={x:b});return a;", true),
            (
                "const o={x:'safe'},alias=o;[alias.x]=[a];return o.x.length;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
    }
}

#[cfg(test)]
mod member_depth_tests {
    #[test]
    fn member_pattern_receiver_depth_stays_bounded_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (n, ok) in [(127, true), (128, false)] {
                    let source = format!(
                        "export function run(a,b){{const o={{x:0}};[{}o{}.x]=[a];return o.x;}}",
                        "(".repeat(n),
                        ")".repeat(n)
                    );
                    let r = crate::compile(&source, false);
                    assert_eq!(r.is_ok(), ok, "{n}: {:?}", r.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod computed_member_tests {
    #[test]
    fn computed_pattern_targets_require_key_proofs_and_widen_alias_facts() {
        for (body, admitted) in [
            ("const xs=[];[xs[a>>>0]]=[b];return xs.length;", true),
            ("const o={};({x:o[a>>>0]}={x:b});return a;", true),
            (
                "const d=Object.create(null);const k='x';[d[k]]=[b];return +(d[k]);",
                true,
            ),
            (
                "const d=Object.create(null);const k='__proto__';({x:d[k]}={x:b});return +(d[k]);",
                true,
            ),
            ("const xs=[];[xs[a]]=[b];return a;", false),
            ("const xs=[];const k='0';[xs[k]]=[b];return a;", false),
            ("const d=Object.create(null);[d[a]]=[b];return a;", false),
            ("const o={x:a};const k='x';[o[k]]=[b];return a;", false),
            (
                "const xs=['safe'];const alias=xs;[alias[a>>>0]]=[b];return xs[0].length;",
                false,
            ),
            (
                "const d=Object.create(null);d.x='safe';const k='x';[d[k]]=[b];return d.x.length;",
                false,
            ),
            ("const xs=[[]];[xs[a>>>0]]=[b];return xs[0].length;", false),
            ("const xs=[];[...xs[a>>>0]]=[b];return a;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        for source in [
            "export function run(a,b){const xs=[];[xs[a as number]]=[b];return a;}",
            "export function run(a,b){const d=Object.create(null);[d[a as string]]=[b];return a;}",
        ] {
            assert!(crate::compile(source, true).is_err(), "{source}");
        }
    }
    #[test]
    fn computed_pattern_key_depth_is_bounded_on_a_small_native_stack() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
            for (wraps,admitted) in [(126,true),(127,false)] {
                let source=format!("export function run(a,b){{const xs=[];[xs[{}a>>>0{}]]=[b];return xs.length;}}","(".repeat(wraps),")".repeat(wraps));
                for typed in [false,true] {
                    let r=crate::compile(&source,typed);
                    assert_eq!(r.is_ok(),admitted,"{wraps}: {:?}",r.err());
                }
            }
        }).unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod member_rest_tests {
    #[test]
    fn member_rest_targets_preserve_receiver_key_and_alias_guards() {
        for (body, admitted) in [
            ("const o={x:[]};[...o.x]=[a,b];return o.x.length;", true),
            (
                "const o={x:{}};({...o.x}={a,b});return 'a' in o.x?1:0;",
                true,
            ),
            (
                "const d=Object.create(null);const k='tail';[...d[k]]=[a,b];return k in d?1:0;",
                true,
            ),
            ("[...a.x]=[b];return a;", false),
            ("const o=a?{}:null;({...o.x}={x:b});return a;", false),
            ("const xs=[];[...xs.length]=[b];return a;", false),
            ("const o={};({...o.__proto__}={x:b});return a;", false),
            ("const xs=[];[...xs[a]]=[b];return a;", false),
            (
                "const o={x:'safe'};const alias=o;[...alias.x]=[a];return o.x.length;",
                false,
            ),
            (
                "const o={x:'safe'};const alias=o;({...alias.x}={a});return o.x.length;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){[...(a as object).x]=[b];return a;}",
                true
            )
            .is_err()
        );
    }
    #[test]
    fn member_rest_reference_depth_is_bounded_on_a_small_native_stack() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
            for (wraps,admitted) in [(126,true),(127,false)] {
                let source=format!("export function run(a,b){{const o={{x:[]}};[...({}o{}).x]=[a,b];return o.x.length;}}","(".repeat(wraps),")".repeat(wraps));
                for typed in [false,true] {
                    let r=crate::compile(&source,typed);
                    assert_eq!(r.is_ok(),admitted,"{wraps}: {:?}",r.err());
                }
            }
        }).unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod computed_selection_tests {
    #[test]
    fn computed_source_keys_require_proven_reads_and_keep_alias_guards() {
        for (body, admitted) in [
            ("let x;({[a>>>0]:x}=[b]);return +x;", true),
            (
                "const d=Object.create(null);d.k=a;const k='k';let x;({[k]:x}=d);return +x;",
                true,
            ),
            (
                "const d=Object.create(null);d.k='text';const k='k';let x='';({[k]:x='fallback'}=d);return x.length;",
                true,
            ),
            ("let x;({[a]:x}=[b]);return a;", false),
            ("const k='x';let x;({[k]:x}={x:b});return a;", true),
            (
                "const d=Object.create(null);let x;({[a]:x}=d);return a;",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';let x,rest;({[k]:x,...rest}=d);return a;",
                true,
            ),
            ("let x;({[a>>>0]:x}='text');return a;", true),
            (
                "const o={x:'safe'};const alias=o;({[a>>>0]:alias.x}=[b]);return o.x.length;",
                false,
            ),
            ("({[a>>>0]:b}=[undefined]);return b;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        assert!(crate::compile("export function run(a,b){const d=Object.create(null);let x;({[a as string]:x}=d);return a;}",true).is_err());
    }
    #[test]
    fn computed_source_key_depth_is_bounded_on_a_small_native_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (wraps, admitted) in [(125, true), (126, false)] {
                    let source = format!(
                        "export function run(a,b){{let x;({{[{}a>>>0{}]:x}}=[b]);return +x;}}",
                        "(".repeat(wraps),
                        ")".repeat(wraps)
                    );
                    for typed in [false, true] {
                        let r = crate::compile(&source, typed);
                        assert_eq!(r.is_ok(), admitted, "{wraps}: {:?}", r.err());
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod computed_rest_tests {
    #[test]
    fn computed_exclusions_keep_presence_and_alias_kind_guards() {
        for (body, admitted) in [
            (
                "let rest={};let x;({[a>>>0]:x,...rest}=[b]);return Object.keys(rest).length;",
                true,
            ),
            (
                "const d=Object.create(null);d.k=a;const k='k';let rest={};let x;({[k]:x,...rest}=d);return Object.keys(rest).length;",
                true,
            ),
            (
                "let {...rest}={k:'seed'};const d=Object.create(null);d.k='text';const k='k';let x;({[k]:x,...rest}=d);return rest.k.length;",
                false,
            ),
            (
                "let {...rest}={0:'seed'};let x;({[a>>>0]:x,...rest}={0:'text'});return rest[0].length;",
                false,
            ),
            (
                "const d=Object.create(null);d.k=a;const k='k';const o={rest:'safe'};let x;({[k]:x,...o.rest}=d);return o.rest.length;",
                false,
            ),
            (
                "const k='k';let rest={};let x;({[k]:x,...rest}={k:b});return a;",
                true,
            ),
            (
                "const k=a?'k':'constructor';let x;({[k]:x}={k:b});return a;",
                false,
            ),
            (
                "let rest={};let x;({[a>>>0]:x,...rest}='text');return a;",
                true,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
    }
    #[test]
    fn computed_rest_key_depth_is_bounded_on_a_small_native_stack() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
            for (wraps,admitted) in [(125,true),(126,false)] {
                let source=format!("export function run(a,b){{let rest={{}};let x;({{[{}a>>>0{}]:x,...rest}}=[b]);return Object.keys(rest).length;}}","(".repeat(wraps),")".repeat(wraps));
                for typed in [false,true] {
                    let r=crate::compile(&source,typed);
                    assert_eq!(r.is_ok(),admitted,"{wraps}: {:?}",r.err());
                }
            }
        }).unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod string_computed_selection_tests {
    #[test]
    fn string_computed_selections_keep_source_and_key_proof_boundaries() {
        for (body, admitted) in [
            (
                "let x='';({[a>>>0]:x='?'}='123');return x.charCodeAt(0);",
                true,
            ),
            ("let x='';({[a>>>0]:x='?'}='text');return x.length;", true),
            (
                "const o={x:''};({[a>>>0]:o.x='?'}='text');return o.x.length;",
                true,
            ),
            ("let x;({[a>>>0]:x}='123');return +x;", false),
            ("let x='';({[a>>>0]:x}='text');return x.length;", false),
            ("let x;({[a]:x}='text');return a;", false),
            ("const k='0';let x;({[k]:x}='text');return a;", false),
            ("let x;({[a>>>0]:x}=(a?'text':null));return a;", false),
            ("let x;({[a>>>0]:x}=a);return a;", true),
            ("const x='';({[a>>>0]:x}='text');return a;", false),
            ("({[a>>>0]:b}='text');return b;", false),
            ("let x,rest;({[a>>>0]:x,...rest}='text');return a;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){let x;({length:x}=(a as string));return a;}",
                true
            )
            .is_err()
        );
    }
    #[test]
    fn string_computed_source_depth_is_bounded_on_a_small_native_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (wraps, admitted) in [(125, true), (126, false)] {
                    let source = format!(
                        "export function run(a,b){{let x='';({{[{}a>>>0{}]:x='?'}}='123');return x.length;}}",
                        "(".repeat(wraps),
                        ")".repeat(wraps)
                    );
                    for typed in [false, true] {
                        let r = crate::compile(&source, typed);
                        assert_eq!(r.is_ok(), admitted, "{wraps}: {:?}", r.err());
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod string_rest_tests {
    #[test]
    fn string_object_rest_keeps_own_enumerable_and_source_proof_guards() {
        for (body, ok) in [
            (
                "let rest={};({...rest}='abc');return Object.keys(rest).length;",
                true,
            ),
            (
                "let rest={};let x;({[a>>>0]:x,...rest}='abc');return Object.keys(rest).length;",
                true,
            ),
            (
                "let {...rest}={length:1};({...rest}='abc');return rest.length;",
                false,
            ),
            ("let rest={};({...rest}=(a?'abc':null));return a;", false),
            // Number entry parameters now use the primitive object-assignment path.
            ("let rest={};({...rest}=a);return a;", true),
            ("let x={};({0:x=(b=7)}=a);return b;", true),
            (
                "let x={},rest={};({[b>>>0]:x=(b=7),...rest}=a);return b;",
                true,
            ),
            ("let x;({valueOf:x}=a);return a;", false),
            ("let x;({[b]:x}=a);return a;", false),
            ("const rest={};({...rest}='abc');return a;", false),
            (
                "let rest={};const k='0';let x;({[k]:x,...rest}='abc');return a;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){let x,rest={};({valueOf:x,...rest}=(a as string));return a;}",
                true
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod string_rest_depth_tests {
    #[test]
    fn string_rest_source_keys_keep_small_stack_depth_boundaries() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
for (n,ok) in [(125,true),(126,false)] {
let source=format!("export function run(a,b){{let x,rest={{}};({{[{}a>>>0{}]:x,...rest}}='abc');return Object.keys(rest).length;}}","(".repeat(n),")".repeat(n));
for ts in [false,true] {let r=crate::compile(&source,ts);assert_eq!(r.is_ok(),ok,"{n}: {:?}",r.err());}
}
}).unwrap().join().unwrap();
    }
}
