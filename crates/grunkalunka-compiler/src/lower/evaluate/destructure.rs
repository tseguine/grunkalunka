//! Assignment patterns consume one saved RHS and update existing lexical cells.
use super::*;
use oxc_ast::ast::{AssignmentTarget, AssignmentTargetProperty};

enum Selection {
    Index(u32),
    Field(Vec<u16>),
    ArrayRest(u32),
    ObjectRest(Vec<ir::ManagedKey>),
}

struct Write<'e, 'a> {
    selection: Selection,
    slot: SlotId,
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
    input: Value,
    receiver: ValueId,
    mode: Mode,
    depth: usize,
    iterator: Option<control_cursor::array_bindings::IteratorState>,
}

pub(super) enum Task<'e, 'a> {
    Begin(Plan<'e, 'a>),
    Default {
        state: Box<State<'e, 'a>>,
        slot: SlotId,
        selected: ValueId,
        condition: ValueId,
    },
}

impl Lowering<'_, '_> {
    fn assignment_pattern_slot(&self, target: &AssignmentTarget<'_>) -> Result<SlotId, String> {
        let identifier = target
            .as_simple_assignment_target()
            .map(|target| target_identifier(target, 0))
            .transpose()?
            .flatten()
            .ok_or_else(|| unsupported(target, "assignment patterns require identifier targets"))?;
        self.assignment_identifier_slot(identifier)
    }

    fn assignment_identifier_slot(
        &self,
        identifier: &IdentifierReference<'_>,
    ) -> Result<SlotId, String> {
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        if !binding.mutable || self.slots[binding.slot.0].ty != Type::ManagedValue {
            return Err(unsupported(
                identifier,
                "assignment target must be an initialized identifier with mutable managed-value storage",
            ));
        }
        Ok(binding.slot)
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
        let receiver = if string {
            Some(Type::ManagedValue)
        } else {
            self.receiver_type(&assignment.right, depth + 1)
        };
        let input_mode = match receiver {
            Some(Type::ManagedValue) if string => Mode::Managed,
            Some(Type::ManagedArray) => Mode::Array,
            Some(Type::ManagedObject) => Mode::Object,
            _ => {
                return Err(unsupported(
                    node,
                    "assignment patterns require a source-proven Array or object",
                ));
            }
        };
        if !matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard)
            && !matches!(
                (mode, receiver),
                (Mode::Array, Some(Type::ManagedArray)) | (Mode::Object, Some(Type::ManagedObject))
            )
        {
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
                            slot: self.assignment_pattern_slot(target)?,
                            default,
                        });
                    }
                }
                if let Some(rest) = &pattern.rest {
                    let start = u32::try_from(pattern.elements.len())
                        .map_err(|_| unsupported(node, "array assignment exceeds index limit"))?;
                    writes.push(Write {
                        selection: Selection::ArrayRest(start),
                        slot: self.assignment_pattern_slot(&rest.target)?,
                        default: None,
                    });
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(pattern) => {
                if string && pattern.rest.is_some() {
                    return Err(unsupported(
                        node,
                        "String object assignment rest is not yet admitted",
                    ));
                }
                let dictionary = self.stored.dictionary(assignment.right.node_id());
                let mut excluded = Vec::new();
                for property in &pattern.properties {
                    let (key, slot, default) = match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(p) => (
                            p.binding.name.encode_utf16().collect(),
                            self.assignment_identifier_slot(&p.binding)?,
                            p.init.as_ref(),
                        ),
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => {
                            let key = static_keys::property(&p.name, p.computed, depth + 1)
                                .ok_or_else(|| {
                                    unsupported(
                                        p.as_ref(),
                                        "object assignment keys require static property names",
                                    )
                                })?;
                            let (target, default) =
                                assignment_patterns::target_and_default(&p.binding);
                            (key, self.assignment_pattern_slot(target)?, default)
                        }
                    };
                    let key = managed::member_key(
                        property,
                        key,
                        receiver == Some(Type::ManagedArray),
                        dictionary,
                    )?;
                    if string
                        && key.as_slice() != [108, 101, 110, 103, 116, 104]
                        && binding_patterns::string_index(&key).is_none()
                    {
                        return Err(unsupported(
                            node,
                            "String object assignment keys require length or canonical numeric indices",
                        ));
                    }
                    excluded.push(ir::ManagedKey::Literal(key.clone()));
                    writes.push(Write {
                        selection: Selection::Field(key),
                        slot,
                        default,
                    });
                }
                if let Some(rest) = &pattern.rest {
                    writes.push(Write {
                        selection: Selection::ObjectRest(excluded),
                        slot: self.assignment_pattern_slot(&rest.target)?,
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
                    && plan.writes.iter().any(|write| write.default.is_some()))
                .then(|| self.begin_array_binding_iterator(plan.node, regions.current()));
                Box::new(State {
                    node: plan.node,
                    writes: plan.writes.into_iter(),
                    input,
                    receiver,
                    mode: plan.mode,
                    depth: plan.depth,
                    iterator,
                })
            }
            Task::Default {
                state,
                slot,
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
                Self::push(regions.current(), state.node, Op::Store { slot, value });
                state
            }
        };
        let node = state.node;
        let receiver = state.receiver;
        while let Some(Write {
            selection,
            slot,
            default,
        }) = state.writes.next()
        {
            let value = match selection {
                Selection::Index(index) => {
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
                Selection::ObjectRest(excluded) => {
                    let rest = self.value(Type::ManagedObject);
                    Self::push(
                        regions.current(),
                        node,
                        Op::ManagedObjectRest {
                            result: rest,
                            object: receiver,
                            excluded,
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
                    slot,
                    selected: value,
                    condition,
                }));
                work.push(Work::Eval(default, depth, Mode::Managed));
                return Ok(());
            }
            Self::push(regions.current(), node, Op::Store { slot, value });
        }
        match state.mode {
            Mode::Array | Mode::Object => values.push(state.input),
            Mode::Managed if self.values[state.input.id().0] == Type::ManagedValue => {
                values.push(state.input)
            }
            Mode::Managed => {
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    if self.values[receiver.0] == Type::ManagedArray {
                        Op::ManagedFromArray {
                            result,
                            input: receiver,
                        }
                    } else {
                        Op::ManagedFromObject {
                            result,
                            input: receiver,
                        }
                    },
                );
                values.push(Value::Managed(result));
            }
            Mode::Predicate => {
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    if self.values[state.input.id().0] == Type::ManagedValue {
                        Op::ManagedTruthy {
                            result,
                            input: state.input.id(),
                        }
                    } else {
                        Op::Boolean {
                            result,
                            value: true,
                        }
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
            "let x;({...x}='abc');return a;",
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
            ("[a]=[b];return a;", false),
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
                false,
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
