//! Binding plans borrow the original source and select each own field or array
//! position in order. Origins prove receiver kinds; annotations never do.
use super::*;
use crate::lower::evaluate::KeyAccess;
use oxc_ast::ast::{
    ArrayPattern, AssignmentPattern, BindingIdentifier, BindingProperty, BindingRestElement,
    ObjectPattern,
};
use oxc_syntax::node::NodeId;

pub(super) fn string_index(key: &[u16]) -> Option<u32> {
    let text = String::from_utf16(key).ok()?;
    let index = text.parse::<u32>().ok()?;
    (index.to_string() == text).then_some(index)
}

pub(super) fn string_field_read(result: ValueId, input: ValueId, key: ir::ManagedKey) -> Op {
    match key {
        ir::ManagedKey::Uint32(index) => Op::ManagedStringGetIndex {
            result,
            input,
            index,
        },
        ir::ManagedKey::String(key) => Op::ManagedStringGetKey { result, input, key },
        _ => unreachable!("planned computed String property"),
    }
}

/// Original-source pattern leaves for representation planning. Plain identifier
/// declarations have their own planners. This walk does not admit a pattern.
pub(super) fn leaves<'n, 'a>(
    pattern: &'n BindingPattern<'a>,
) -> impl Iterator<Item = &'n BindingIdentifier<'a>> {
    let mut pending = if matches!(pattern, BindingPattern::BindingIdentifier(_)) {
        Vec::new()
    } else {
        vec![pattern]
    };
    std::iter::from_fn(move || {
        while let Some(pattern) = pending.pop() {
            match pattern {
                BindingPattern::BindingIdentifier(identifier) => return Some(identifier.as_ref()),
                BindingPattern::AssignmentPattern(assignment) => pending.push(&assignment.left),
                BindingPattern::ObjectPattern(object) => {
                    if let Some(rest) = &object.rest {
                        pending.push(&rest.argument);
                    }
                    pending.extend(
                        object
                            .properties
                            .iter()
                            .rev()
                            .map(|property| &property.value),
                    );
                }
                BindingPattern::ArrayPattern(array) => {
                    if let Some(rest) = &array.rest {
                        pending.push(&rest.argument);
                    }
                    pending.extend(array.elements.iter().rev().flatten());
                }
            }
        }
        None
    })
}

pub(super) enum BindingStep<'n, 'a> {
    Object {
        source: &'n ObjectPattern<'a>,
        receiver: Type,
    },
    Array {
        source: &'n ArrayPattern<'a>,
        live: bool,
        string: bool,
    },
    Field {
        source: &'n BindingProperty<'a>,
        key: Vec<u16>,
    },
    ComputedField {
        kind: evaluate::RuntimeKey,
        source: &'n BindingProperty<'a>,
        key: &'n Expression<'a>,
        depth: usize,
    },
    Element {
        source: &'n BindingPattern<'a>,
        index: u32,
    },
    ObjectRest(&'n BindingRestElement<'a>),
    ArrayRest {
        source: &'n BindingRestElement<'a>,
        start: u32,
    },
    Default(&'n AssignmentPattern<'a>),
    Bind {
        identifier: &'n BindingIdentifier<'a>,
    },
    Leave,
}

// A default or computed field can change any ancestor array before its next
// step. Each such iterator must observe growth only until its own exhaustion.
fn mark_live_arrays(containers: &[usize], steps: &mut [BindingStep<'_, '_>]) {
    for &index in containers {
        if let BindingStep::Array { live, .. } = &mut steps[index] {
            *live = true;
        }
    }
}

pub(super) struct BindingPlan<'n, 'a> {
    pub(super) steps: std::vec::IntoIter<BindingStep<'n, 'a>>,
    pub(super) mutable: bool,
}

pub(super) struct PatternBinding<'n, 'a> {
    pub(super) source: &'n Expression<'a>,
    pub(super) mode: evaluate::Mode,
    pub(super) plan: BindingPlan<'n, 'a>,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn prepare_local_pattern<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        mutable: bool,
    ) -> Result<PatternBinding<'n, 'a>, String> {
        let source = local
            .init
            .as_ref()
            .ok_or_else(|| unsupported(local, "binding patterns require an initializer"))?;
        let receiver = match self.managed_source_type(source, 0) {
            Some(Type::ManagedValue) if self.stored.string(source.node_id()) => {
                Some(Type::ManagedValue)
            }
            Some(Type::ManagedValue) => self.stored.container(source.node_id()),
            ty => ty.or_else(|| {
                (self.stored.object_primitive(source.node_id())
                    || self.numeric_result_shape(source, 0)
                    || self.operand_boolean_shape(source, 0))
                .then_some(Type::General)
            }),
        };
        let (root_type, mode) = match (&local.id, receiver) {
            (_, Some(Type::ManagedValue)) => (Type::ManagedValue, evaluate::Mode::Managed),
            (_, Some(Type::ManagedArray)) => (Type::ManagedArray, evaluate::Mode::Array),
            (BindingPattern::ObjectPattern(_), Some(Type::ManagedObject)) => {
                (Type::ManagedObject, evaluate::Mode::Object)
            }
            (BindingPattern::ObjectPattern(_), Some(Type::General)) => {
                (Type::General, evaluate::Mode::General)
            }
            (BindingPattern::ArrayPattern(_), _) => {
                return Err(unsupported(
                    source,
                    "array bindings require a source-proven managed array",
                ));
            }
            _ => {
                return Err(unsupported(
                    source,
                    "object bindings require a source-proven managed object or array",
                ));
            }
        };
        let plan = self.prepare_binding_pattern(&local.id, source.node_id(), root_type, mutable)?;
        Ok(PatternBinding { source, mode, plan })
    }

    /// Prepare a pattern independently of evaluating its original source. The
    /// caller proves the root kind; every nested selection has its own source
    /// proof. The resulting plan can consume an already evaluated value.
    pub(super) fn prepare_binding_pattern<'n, 'a>(
        &mut self,
        pattern: &'n BindingPattern<'a>,
        origin: NodeId,
        root_type: Type,
        mutable: bool,
    ) -> Result<BindingPlan<'n, 'a>, String> {
        enum Work<'n, 'a> {
            Pattern(&'n BindingPattern<'a>, NodeId, usize),
            Field(&'n BindingProperty<'a>, Type, bool, usize),
            Element(&'n BindingPattern<'a>, u32, usize),
            ObjectRest(&'n BindingRestElement<'a>, usize),
            ArrayRest(&'n BindingRestElement<'a>, u32, usize),
            Leave,
        }
        let root = pattern.node_id();
        let mut pending = vec![Work::Pattern(pattern, origin, 0)];
        let mut steps = Vec::new();
        let mut containers = Vec::new();
        let mut count = 0;
        while let Some(work) = pending.pop() {
            match work {
                Work::Pattern(pattern, origin, parent_depth) => {
                    match pattern {
                        BindingPattern::BindingIdentifier(identifier) => {
                            count += 1;
                            steps.push(BindingStep::Bind { identifier });
                            continue;
                        }
                        BindingPattern::AssignmentPattern(assignment) => {
                            mark_live_arrays(&containers, &mut steps);
                            steps.push(BindingStep::Default(assignment));
                            // The left subtree consumes the selected/default
                            // result and uses its post-default source origins.
                            // A default wrapper adds no container nesting.
                            pending.push(Work::Pattern(
                                &assignment.left,
                                assignment.left.node_id(),
                                parent_depth,
                            ));
                            continue;
                        }
                        _ => (),
                    }
                    let depth = parent_depth + 1;
                    if depth > 128 {
                        return Err(unsupported(pattern, "binding pattern nesting exceeds 128"));
                    }
                    let proven = if pattern.node_id() == root {
                        Some(root_type)
                    } else if self.stored.string(origin) {
                        Some(Type::ManagedValue)
                    } else if self.stored.object_primitive(origin) {
                        Some(Type::General)
                    } else {
                        self.stored.container(origin)
                    };
                    containers.push(steps.len());
                    pending.push(Work::Leave);
                    match pattern {
                        BindingPattern::ObjectPattern(object) => {
                            let Some(
                                receiver @ (Type::ManagedObject
                                | Type::ManagedArray
                                | Type::ManagedValue
                                | Type::General),
                            ) = proven
                            else {
                                return Err(unsupported(
                                    object.as_ref(),
                                    "nested object bindings require a source-proven managed object or array",
                                ));
                            };
                            if receiver == Type::ManagedValue && !self.stored.string(origin) {
                                return Err(unsupported(
                                    object.as_ref(),
                                    "String object bindings require a proven String",
                                ));
                            }
                            steps.push(BindingStep::Object {
                                source: object,
                                receiver,
                            });
                            if let Some(rest) = &object.rest {
                                pending.push(Work::ObjectRest(rest, depth));
                            }
                            let dictionary = self.stored.dictionary(origin);
                            pending.extend(object.properties.iter().rev().map(|property| {
                                Work::Field(property, receiver, dictionary, depth)
                            }));
                        }
                        BindingPattern::ArrayPattern(array) => {
                            let string =
                                proven == Some(Type::ManagedValue) && self.stored.string(origin);
                            if proven != Some(Type::ManagedArray) && !string {
                                return Err(unsupported(
                                    array.as_ref(),
                                    "nested array bindings require a source-proven managed array",
                                ));
                            }
                            let fixed = u32::try_from(array.elements.len()).map_err(|_| {
                                unsupported(array.as_ref(), "array binding length exceeds u32")
                            })?;
                            steps.push(BindingStep::Array {
                                source: array,
                                live: false,
                                string,
                            });
                            if let Some(rest) = &array.rest {
                                // Rest follows every fixed position, including
                                // trailing elisions, and precedes the Leave.
                                pending.push(Work::ArrayRest(rest, fixed, depth));
                            }
                            pending.extend(array.elements.iter().enumerate().rev().filter_map(
                                |(index, element)| {
                                    element
                                        .as_ref()
                                        .map(|element| Work::Element(element, index as u32, depth))
                                },
                            ));
                        }
                        _ => unreachable!("identifiers and defaults were planned above"),
                    }
                }
                Work::Field(property, receiver, dictionary, depth) => {
                    if let Some(key) =
                        static_keys::property(&property.key, property.computed, depth)
                    {
                        let key = evaluate::managed_member_key(
                            property,
                            key,
                            receiver == Type::ManagedArray,
                            dictionary,
                        )?;
                        if receiver == Type::General && string_index(&key).is_none() {
                            return Err(unsupported(
                                property,
                                "Number and Boolean object binding keys require canonical numeric indices",
                            ));
                        }
                        if receiver == Type::ManagedValue
                            && key.as_slice() != [108, 101, 110, 103, 116, 104]
                            && string_index(&key).is_none()
                        {
                            return Err(unsupported(
                                property,
                                "String object binding keys require length or canonical numeric indices",
                            ));
                        }
                        steps.push(BindingStep::Field {
                            source: property,
                            key,
                        });
                    } else if property.computed
                        && let Some(key) = property.key.as_expression()
                        && let Some(kind) =
                            if matches!(receiver, Type::ManagedObject | Type::ManagedArray) {
                                let known = self.binding_key_kind(
                                    receiver,
                                    dictionary,
                                    key,
                                    depth,
                                    KeyAccess::Read,
                                );
                                known.or_else(|| {
                                    // An Array's inherited intrinsic methods are
                                    // not represented by the own-data getter.
                                    if receiver == Type::ManagedArray
                                        && !self
                                            .stored
                                            .record_names
                                            .domain(key, depth)
                                            .array_own_names()
                                    {
                                        None
                                    } else {
                                        self.boxed_runtime_key_fallback(
                                            key,
                                            depth,
                                            receiver,
                                            dictionary,
                                            KeyAccess::Read,
                                        )
                                    }
                                })
                            } else if receiver == Type::ManagedValue
                                && self.string_own_key(key, depth)
                            {
                                Some(evaluate::RuntimeKey::String)
                            } else {
                                self.stored
                                    .uint32_keys
                                    .contains(key, depth)
                                    .then_some(evaluate::RuntimeKey::Uint32)
                            }
                    {
                        mark_live_arrays(&containers, &mut steps);
                        steps.push(BindingStep::ComputedField {
                            kind,
                            source: property,
                            key,
                            depth,
                        });
                    } else {
                        return Err(unsupported(
                            property,
                            "computed binding keys require an unsigned index or a proven record name",
                        ));
                    }
                    pending.push(Work::Pattern(
                        &property.value,
                        property.value.node_id(),
                        depth,
                    ));
                }
                Work::Element(source, index, depth) => {
                    steps.push(BindingStep::Element { source, index });
                    pending.push(Work::Pattern(source, source.node_id(), depth));
                }
                Work::ObjectRest(source, depth) => {
                    steps.push(BindingStep::ObjectRest(source));
                    pending.push(Work::Pattern(
                        &source.argument,
                        source.argument.node_id(),
                        depth,
                    ));
                }
                Work::ArrayRest(source, start, depth) => {
                    steps.push(BindingStep::ArrayRest { source, start });
                    pending.push(Work::Pattern(
                        &source.argument,
                        source.argument.node_id(),
                        depth,
                    ));
                }
                Work::Leave => {
                    containers.pop().expect("balanced binding plan");
                    steps.push(BindingStep::Leave);
                }
            }
        }
        self.charge_declaration_bindings(pattern, count)?;
        Ok(BindingPlan {
            steps: steps.into_iter(),
            mutable,
        })
    }
}

#[cfg(test)]
mod boxed_computed_binding_key_tests {
    #[test]
    fn computed_bindings_read_boxed_runtime_keys_from_data_collections() {
        for body in [
            "const d=Object.create(null);d.x=b;const k=a?'x':b;const {[k]:v=7}=d;return v===(a?b:7)?1:0;",
            "const xs=[a,b];const k=a?'length':'0';const {[k]:v=7}=xs;return v===(a?2:a)?1:0;",
            "const xs=[a,b];const k=a?'0':'length';const {[k]:v}=xs;return v===(a?a:2)?1:0;",
            "const d=Object.create(null);d.x=b;d.y=a;const k=a?'x':b;const {[k]:v,...rest}=d;return Object.keys(rest).length;",
            "const d=Object.create(null);d.x=a;let n=0;const {[(n++,a?'x':b)]:v=9}=d;return n===1&&v===(a?a:9)?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let result = crate::compile(&source, typescript);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn boxed_binding_keys_keep_reference_and_prototype_guards() {
        for body in [
            "const d=Object.create(null);const k={};const {[k]:v}=d;return a;",
            "const xs=[b];const k={};const {[k]:v}=xs;return a;",
            "const o={x:a};const k=b?'constructor':'x';const {[k]:v}=o;return a;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
    }
}

#[cfg(test)]
mod string_rest_binding_tests {
    #[test]
    fn string_rest_bindings_keep_presence_and_source_guards() {
        for (body, ok) in [
            (
                "const {...rest}='abc';return Object.keys(rest).length;",
                true,
            ),
            (
                "const {0:x,...rest}='abc';return Object.keys(rest).length;",
                true,
            ),
            ("const {...rest}='abc';return rest.length;", false),
            ("const {...rest}=(a?'abc':null);return a;", false),
            // Number entry parameters now use the primitive object-binding path.
            ("const {...rest}=a;return a;", true),
            ("const {0:x=(b=7)}=a;return b;", true),
            ("const {[b>>>0]:x=(b=7),...rest}=a;return b;", true),
            ("const {valueOf:x}=a;return a;", false),
            ("const {[b]:x}=a;return a;", false),
            ("const k='0';const {[k]:x,...rest}='abc';return a;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&source, ts);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){const {valueOf:x,...rest}=(a as string);return a;}",
                true
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod unsigned_binding_tests {
    #[test]
    fn unsigned_binding_keys_require_real_syntax_and_receiver_proofs() {
        for (body, ok) in [
            ("const {[a>>>0]:x=b}=[a];return x;", true),
            ("const {[a>>>0]:x='?'}='abc';return x.length;", true),
            ("const {[a]:x}=[b];return a;", true),
            ("const {[a>>>0]:x}=b;return a;", true),
            (
                "const k=a?'k':'__proto__';const {[k]:x}={k:b};return a;",
                false,
            ),
            ("const k='0';const {[k]:x}=[b];return a;", true),
            ("const {[a>>>0]:x}=[a?'text':b];return x.length;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&source, ts);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){const {[{} as unknown as number]:x}=[b];return a;}",
                true
            )
            .is_err()
        );
    }
    #[test]
    fn unsigned_binding_keys_keep_small_native_stack_boundaries() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (n, ok) in [(126, true), (127, false)] {
                    let source = format!(
                        "export function run(a,b){{const {{[{}a>>>0{}]:x=b}}=[a];return x;}}",
                        "(".repeat(n),
                        ")".repeat(n)
                    );
                    for ts in [false, true] {
                        let r = crate::compile(&source, ts);
                        assert_eq!(r.is_ok(), ok, "{n}: {:?}", r.err());
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod primitive_object_binding_tests {
    #[test]
    fn number_and_boolean_bindings_admit_empty_and_rest_patterns_only() {
        for (body, ok) in [
            ("const {}=a;return b;", true),
            (
                "const {...rest}=false;return Object.keys(rest).length;",
                true,
            ),
            (
                "const {...rest}=(a?b:false);return Object.keys(rest).length;",
                true,
            ),
            (
                "const {value:{...rest}}={value:a};return Object.keys(rest).length;",
                true,
            ),
            (
                "let n=0;for(const {...rest} of [a,false])n+=Object.keys(rest).length;return n;",
                true,
            ),
            ("const {x}=a;return x;", false),
            ("const {...rest}=(a?b:null);return a;", false),
            ("const {}=null;return a;", false),
            ("const {}=undefined;return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&source, ts);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){const value=(a?{}:null) as object;const {}=value;return b;}",
                true
            )
            .is_err()
        );
    }
}
