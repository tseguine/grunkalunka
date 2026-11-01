use super::{
    managed_value_tests::{effect, node, value},
    *,
};

fn retain(wat: &str, name: &str, native: Option<&str>) -> Vec<u8> {
    let wasm = wat::parse_str(wat).unwrap();
    if let Some(directory) = std::env::var_os("GRUNKALUNKA_IR_TEST_ARTIFACTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("reference-{name}"));
        std::fs::write(path.with_extension("wat"), wat).unwrap();
        std::fs::write(path.with_extension("wasm"), &wasm).unwrap();
        std::fs::write(path.with_extension("expect"), native.unwrap_or("TRAP")).unwrap();
    }
    wasm
}

fn compare(m: &Module, name: &str, native: &str) {
    verify_module(m).unwrap();
    node(
        &retain(&crate::emit::emit(m).unwrap(), name, Some(native)),
        native,
    );
}

fn trap_wat(wat: &str, name: &str) {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let wasm = retain(wat, name, None);
    let script = format!(
        "const assert=require('node:assert/strict');const run=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports.run;assert.throws(()=>run(17,9),WebAssembly.RuntimeError);"
    );
    let mut child = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "reference trap timeout: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn module() -> Module {
    let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    m.functions[0].values.clear();
    m.functions[0].body.operations.clear();
    m
}

#[test]
fn projected_object_keeps_its_original_identity() {
    let mut m = module();
    let f = &mut m.functions[0];
    let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    let input = value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
        result,
        input: object,
    });
    let object = value(f, Type::ManagedObject, |result| Op::ManagedToObject {
        result,
        input,
    });
    let boxed = value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
        result,
        input: object,
    });
    let same = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: input,
        right: boxed,
    });
    let result = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: same,
    });
    f.body.exit = Exit::Return(result);
    compare(&m, "object-identity", "1");
}

#[test]
fn object_projection_checks_language_kind_as_well_as_carrier() {
    let mut m = module();
    let f = &mut m.functions[0];
    let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    let input = value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
        result,
        input: object,
    });
    value(f, Type::ManagedObject, |result| Op::ManagedToObject {
        result,
        input,
    });
    let result = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    f.body.exit = Exit::Return(result);
    verify_module(&m).unwrap();
    let wat = crate::emit::emit(&m).unwrap();
    // Deliberately malformed language value in an otherwise valid Wasm module:
    // the canonical object carrier cannot excuse an incorrect language tag.
    assert_eq!(wat.matches("i32.const 4\nf64.const 0\n").count(), 1);
    trap_wat(
        &wat.replacen(
            "i32.const 4\nf64.const 0\n",
            "i32.const 6\nf64.const 0\n",
            1,
        ),
        "object-wrong-tag",
    );
}

#[test]
fn projected_arrays_retain_their_sparse_logical_length() {
    for length in [0, 3, u32::MAX] {
        let mut m = module();
        let f = &mut m.functions[0];
        let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
            result,
            length,
        });
        let input = value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
            result,
            input: array,
        });
        let array = value(f, Type::ManagedArray, |result| Op::ManagedToArray {
            result,
            input,
        });
        let result = value(f, Type::Number, |result| Op::ManagedArrayLength {
            result,
            array,
        });
        f.body.exit = Exit::Return(result);
        compare(
            &m,
            &format!("array-length-{length}"),
            &format!("new Array({length}).length"),
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum Container {
    Object,
    Array,
}
impl Container {
    const ALL: [Self; 2] = [Self::Object, Self::Array];
    fn ty(self) -> Type {
        match self {
            Self::Object => Type::ManagedObject,
            Self::Array => Type::ManagedArray,
        }
    }
    fn literal(self) -> &'static str {
        match self {
            Self::Object => "{}",
            Self::Array => "[]",
        }
    }
    fn create(self, f: &mut Function) -> (ValueId, ValueId) {
        let reference = value(f, self.ty(), |result| match self {
            Self::Object => Op::ManagedObjectNew { result },
            Self::Array => Op::ManagedArrayNew { result, length: 0 },
        });
        let boxed = self.boxed(f, reference);
        (reference, boxed)
    }
    fn boxed(self, f: &mut Function, input: ValueId) -> ValueId {
        value(f, Type::ManagedValue, |result| match self {
            Self::Object => Op::ManagedFromObject { result, input },
            Self::Array => Op::ManagedFromArray { result, input },
        })
    }
    fn project(self, f: &mut Function, input: ValueId) -> ValueId {
        value(f, self.ty(), |result| match self {
            Self::Object => Op::ManagedToObject { result, input },
            Self::Array => Op::ManagedToArray { result, input },
        })
    }
    fn get(self, f: &mut Function, input: ValueId) -> ValueId {
        value(f, Type::ManagedValue, |result| match self {
            Self::Object => Op::ManagedGetOwn {
                result,
                object: input,
                key: vec![48],
            },
            Self::Array => Op::ManagedArrayGetOwn {
                result,
                array: input,
                key: vec![48],
            },
        })
    }
    fn set(self, f: &mut Function, input: ValueId, stored: ValueId) {
        effect(
            f,
            match self {
                Self::Object => Op::ManagedSetOwn {
                    object: input,
                    key: vec![48],
                    value: stored,
                },
                Self::Array => Op::ManagedArraySetOwn {
                    array: input,
                    key: vec![48],
                    value: stored,
                },
            },
        );
    }
}
fn primitive(f: &mut Function, slot: usize) -> ValueId {
    let n = value(f, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(slot),
    });
    let g = value(f, Type::General, |result| Op::Pack { result, input: n });
    value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
        result,
        input: g,
    })
}
fn equal(f: &mut Function, left: ValueId, right: ValueId) -> ValueId {
    let predicate = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left,
        right,
    });
    value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: predicate,
    })
}
fn sum(f: &mut Function, left: ValueId, right: ValueId) -> ValueId {
    value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left,
        right,
    })
}
fn finish_zero(f: &mut Function) {
    let result = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    f.body.exit = Exit::Return(result);
}
fn traps(m: &Module, name: &str) {
    verify_module(m).unwrap();
    trap_wat(&crate::emit::emit(m).unwrap(), name);
}

#[test]
fn projected_containers_mutate_originals_and_keep_saved_values() {
    for kind in Container::ALL {
        let mut m = module();
        let f = &mut m.functions[0];
        let (original, boxed) = kind.create(f);
        let a = primitive(f, 0);
        let b = primitive(f, 1);
        kind.set(f, original, a);
        let projected = kind.project(f, boxed);
        let saved = kind.get(f, projected);
        kind.set(f, projected, b);
        let updated = kind.get(f, original);
        let alias = kind.boxed(f, projected);
        let old = equal(f, saved, a);
        let new = equal(f, updated, b);
        let identity = equal(f, boxed, alias);
        let values = sum(f, old, new);
        let result = sum(f, values, identity);
        f.body.exit = Exit::Return(result);
        compare(
            &m,
            &format!("mutation-{kind:?}"),
            &format!(
                "(()=>{{const o={};o[0]=a;const p=o,old=p[0];p[0]=b;return Number(old===a)+Number(o[0]===b)+Number(p===o)}})()",
                kind.literal()
            ),
        );
    }
}

#[test]
fn nested_property_projections_keep_children_and_replaced_snapshots() {
    for parent in Container::ALL {
        for child in Container::ALL {
            let mut m = module();
            let f = &mut m.functions[0];
            let (holder, _) = parent.create(f);
            let (original, boxed) = child.create(f);
            let (_, replacement) = child.create(f);
            parent.set(f, holder, boxed);
            let saved = parent.get(f, holder);
            parent.set(f, holder, replacement);
            let projected = child.project(f, saved);
            let b = primitive(f, 1);
            child.set(f, projected, b);
            let got = child.get(f, original);
            let updated = equal(f, got, b);
            let latest = parent.get(f, holder);
            let untouched = equal(f, latest, replacement);
            let identity = child.boxed(f, projected);
            let identical = equal(f, identity, boxed);
            let values = sum(f, updated, untouched);
            let result = sum(f, values, identical);
            f.body.exit = Exit::Return(result);
            compare(
                &m,
                &format!("nested-{parent:?}-{child:?}"),
                &format!(
                    "(()=>{{const o={},x={},y={};o[0]=x;const old=o[0];o[0]=y;old[0]=b;return Number(x[0]===b)+Number(o[0]===y)+Number(old===x)}})()",
                    parent.literal(),
                    child.literal(),
                    child.literal()
                ),
            );
        }
    }
}

#[test]
fn object_and_array_carriers_cannot_be_interchanged() {
    for kind in Container::ALL {
        let mut m = module();
        let f = &mut m.functions[0];
        let (_, boxed) = kind.create(f);
        let requested = match kind {
            Container::Object => Container::Array,
            Container::Array => Container::Object,
        };
        requested.project(f, boxed);
        finish_zero(f);
        traps(&m, &format!("wrong-carrier-{kind:?}"));
    }
}

#[test]
fn non_container_values_trap_in_both_projections() {
    for requested in Container::ALL {
        for kind in 0..5 {
            let mut m = module();
            let f = &mut m.functions[0];
            let input = match kind {
                0 | 1 => {
                    let g = value(f, Type::General, |result| Op::Nullish {
                        result,
                        value: if kind == 0 {
                            Nullish::Undefined
                        } else {
                            Nullish::Null
                        },
                    });
                    value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
                        result,
                        input: g,
                    })
                }
                2 => {
                    let p = value(f, Type::Predicate, |result| Op::Boolean {
                        result,
                        value: true,
                    });
                    let g = value(f, Type::General, |result| Op::Pack { result, input: p });
                    value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
                        result,
                        input: g,
                    })
                }
                3 => primitive(f, 0),
                _ => value(f, Type::ManagedValue, |result| Op::ManagedString {
                    result,
                    units: vec![120],
                }),
            };
            requested.project(f, input);
            finish_zero(f);
            traps(&m, &format!("wrong-kind-{requested:?}-{kind}"));
        }
    }
}

#[test]
fn array_projection_also_checks_language_kind_and_nonnull_payload() {
    for kind in Container::ALL {
        let mut m = module();
        let f = &mut m.functions[0];
        let (_, boxed) = kind.create(f);
        kind.project(f, boxed);
        finish_zero(f);
        verify_module(&m).unwrap();
        let wat = crate::emit::emit(&m).unwrap();
        assert_eq!(wat.matches("i32.const 4\nf64.const 0\n").count(), 1);
        trap_wat(
            &wat.replacen(
                "i32.const 4\nf64.const 0\n",
                "i32.const 5\nf64.const 0\n",
                1,
            ),
            &format!("wrong-language-kind-{kind:?}"),
        );
        let header = "i32.const 4\nf64.const 0\n";
        let start = wat.find(header).unwrap() + header.len();
        let end = start + wat[start..].find('\n').unwrap();
        assert!(wat[start..end].starts_with("local.get "));
        let mut malformed = wat.clone();
        malformed.replace_range(start..end, "ref.null eq");
        trap_wat(&malformed, &format!("null-payload-{kind:?}"));
    }
}

#[test]
fn projection_verifier_requires_visible_managed_inputs_and_exact_results() {
    for kind in Container::ALL {
        for change in 0..3 {
            let mut m = module();
            let f = &mut m.functions[0];
            let (reference, boxed) = kind.create(f);
            let input = match change {
                0 => reference,
                1 => ValueId(f.values.len() + 1),
                _ => boxed,
            };
            let projected = kind.project(f, input);
            if change == 1 {
                value(f, Type::ManagedValue, |result| Op::ManagedString {
                    result,
                    units: vec![],
                });
            }
            if change == 2 {
                f.values[projected.0] = Type::ManagedValue;
            }
            finish_zero(f);
            let error = verify_module(&m).unwrap_err();
            if change == 1 {
                assert!(error.contains("not visible"), "{error}");
            } else {
                assert!(error.contains("type"), "{error}");
            }
        }
    }
}

#[test]
fn projections_use_reference_globals_in_single_coroutine_frames() {
    for kind in Container::ALL {
        let mut m = module();
        let f = &mut m.functions[0];
        f.return_type = Type::General;
        let (_, boxed) = kind.create(f);
        kind.project(f, boxed);
        let n = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        let result = value(f, Type::General, |result| Op::Pack { result, input: n });
        f.body.exit = Exit::Return(result);
        verify_async_module(&m).unwrap();
        let wat = crate::emit::emit_async(&m).unwrap();
        wat::parse_str(wat).unwrap();
    }
}

#[test]
fn projections_traverse_deep_regions_on_two_megabyte_stacks() {
    for kind in Container::ALL {
        let mut m = module();
        let f = &mut m.functions[0];
        let (_, boxed) = kind.create(f);
        kind.project(f, boxed);
        finish_zero(f);
        let span = f.body.span;
        let mut body = std::mem::replace(
            &mut f.body,
            Region {
                id: RegionId(0),
                span,
                operations: vec![],
                exit: Exit::Fallthrough,
            },
        );
        body.id = RegionId(256);
        for id in (0..256).rev() {
            body = Region {
                id: RegionId(id),
                span,
                operations: vec![Operation {
                    span,
                    kind: Op::Block(body),
                }],
                exit: Exit::Fallthrough,
            };
        }
        f.body = body;
        let wat = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                verify_module(&m).unwrap();
                crate::emit::emit(&m).unwrap()
            })
            .unwrap()
            .join()
            .unwrap();
        node(&retain(&wat, &format!("deep-{kind:?}"), Some("0")), "0");
    }
}

// Exercise the new operations at actual generated receiver boundaries while
// leaving source evaluation, argument frames and environments intact. This is
// backend composition evidence, not admission of nested source property reads.
#[test]
fn projected_receivers_compose_with_generated_calls_and_captures() {
    for (name, helpers, body) in [
        (
            "helper-object",
            "function id(x){return x;}",
            "const o={x:a};const p=id(o);return p.x===a?1:0;",
        ),
        (
            "helper-array",
            "function id(x){return x;}",
            "const o=[a,b];const p=id(o);return p[0]===a?1:0;",
        ),
        (
            "captured-object",
            "",
            "const o={x:a};const f=()=>o.x===a?1:0;const holder={f};return f();",
        ),
        (
            "captured-array",
            "",
            "const o=[a,b];const f=()=>o[0]===a?1:0;const holder={f};return f();",
        ),
        (
            "returned-object",
            "",
            "const make=x=>()=>x;const o={x:a};const f=make(o);const p=f();return p.x===a&&p===o?1:0;",
        ),
        (
            "returned-array",
            "",
            "const make=x=>()=>x;const o=[a,b];const f=make(o);const p=f();return p[0]===a&&p===o?1:0;",
        ),
    ] {
        let source = format!("{helpers}export function run(a,b){{{body}}}");
        let mut m = crate::lower_source(&source, false).unwrap();
        let mut count = 0;
        for f in &mut m.functions {
            let indices = (0..f.body.operations.len())
                .filter(|index| {
                    matches!(
                        f.body.operations[*index].kind,
                        Op::ManagedGetOwn { .. } | Op::ManagedArrayGetOwn { .. }
                    )
                })
                .collect::<Vec<_>>();
            for index in indices.into_iter().rev() {
                let (receiver, kind) = match &mut f.body.operations[index].kind {
                    Op::ManagedGetOwn { object, .. } => (object, Container::Object),
                    Op::ManagedArrayGetOwn { array, .. } => (array, Container::Array),
                    _ => unreachable!(),
                };
                let original = *receiver;
                let input = ValueId(f.values.len());
                f.values.push(Type::ManagedValue);
                let result = ValueId(f.values.len());
                f.values.push(kind.ty());
                *receiver = result;
                let span = f.body.operations[index].span;
                let boxing = match kind {
                    Container::Object => Op::ManagedFromObject {
                        result: input,
                        input: original,
                    },
                    Container::Array => Op::ManagedFromArray {
                        result: input,
                        input: original,
                    },
                };
                let projection = match kind {
                    Container::Object => Op::ManagedToObject { result, input },
                    Container::Array => Op::ManagedToArray { result, input },
                };
                f.body.operations.splice(
                    index..index,
                    [
                        Operation { span, kind: boxing },
                        Operation {
                            span,
                            kind: projection,
                        },
                    ],
                );
                count += 1;
            }
        }
        assert!(count > 0, "{name}");
        compare(&m, name, &format!("(()=>{{{helpers}{body}}})()"));
    }
}

#[test]
fn actual_function_carriers_are_not_object_or_array_receivers() {
    for kind in Container::ALL {
        let mut m = crate::lower_source(
            "export function run(a,b){const f=()=>a;const o={f};return a;}",
            false,
        )
        .unwrap();
        let f = &mut m.functions[0];
        let input = f
            .body
            .operations
            .iter()
            .find_map(|op| match op.kind {
                Op::ManagedFromClosure { result, .. } => Some(result),
                _ => None,
            })
            .unwrap();
        kind.project(f, input);
        traps(&m, &format!("function-carrier-{kind:?}"));
    }
}
