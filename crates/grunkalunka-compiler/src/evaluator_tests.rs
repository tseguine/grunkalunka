//! Control-refactor observations use the real parser, semantic analysis and IR.
use std::cell::Cell;

thread_local! {
    static ACTIVE: Cell<usize> = const { Cell::new(0) };
    static MAXIMUM: Cell<usize> = const { Cell::new(0) };
    static ENTRIES: Cell<usize> = const { Cell::new(0) };
    static PENDING: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct Entry;
pub(crate) fn entry() -> Entry {
    ACTIVE.with(|active| {
        active.set(active.get() + 1);
        MAXIMUM.with(|maximum| maximum.set(maximum.get().max(active.get())));
    });
    ENTRIES.with(|entries| entries.set(entries.get() + 1));
    Entry
}
impl Drop for Entry {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(active.get().checked_sub(1).expect("balanced entry")));
    }
}

pub(crate) fn pending(count: usize) {
    PENDING.with(|pending| pending.set(pending.get().max(count)));
}

fn observe<T>(run: impl FnOnce() -> T) -> (T, usize, usize, usize) {
    ACTIVE.with(|active| assert_eq!(active.get(), 0));
    MAXIMUM.with(|maximum| maximum.set(0));
    ENTRIES.with(|entries| entries.set(0));
    PENDING.with(|pending| pending.set(0));
    let result = run();
    ACTIVE.with(|active| assert_eq!(active.get(), 0, "entry balanced after success or error"));
    (
        result,
        MAXIMUM.with(Cell::get),
        ENTRIES.with(Cell::get),
        PENDING.with(Cell::get),
    )
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    for byte in bytes {
        write!(text, "{byte:02x}").unwrap();
    }
    text
}

// Finite observations cover every admitted family and every evaluator-mode edge.
// The immutable pre-refactor capture supplies whole IR/WAT/Wasm/error comparison.
const CASES: &[(&str, &str, bool, bool)] = &[
    (
        "leaf",
        "export function run(a,b) { return a+1; }",
        false,
        true,
    ),
    (
        "identifiers",
        "export function run(a,b) { return Infinity+NaN+a+b; }",
        false,
        true,
    ),
    (
        "numbers",
        "export function run(a,b) { return ((a-b)*b)/(a%3); }",
        false,
        true,
    ),
    (
        "number-bits",
        "export function run(a,b) { return ((a&b)|(a^b))+(a<<b)+(a>>b)+(a>>>b); }",
        false,
        true,
    ),
    (
        "unary",
        "export function run(a,b) { return +a+-b+~b; }",
        false,
        true,
    ),
    (
        "unary-bridge",
        "export function run(a,b) { const flag=a<b; return +flag+-!a+~(a!==b); }",
        false,
        true,
    ),
    (
        "arithmetic-bridge",
        "export function run(a,b) { const flag=a<b; return flag+true-!a*(a===b)/false%true; }",
        false,
        true,
    ),
    (
        "number-predicate",
        "export function run(a,b) { if(a) return a; return b; }",
        false,
        true,
    ),
    (
        "boolean-predicate",
        "export function run(a,b) { const flag=true; if(!flag) return a; return b; }",
        false,
        true,
    ),
    (
        "comparisons",
        "export function run(a,b) { if(a<b||a<=b||a>b||a>=b||a==b||a!=b||a===b||a!==b) return a; return b; }",
        false,
        true,
    ),
    (
        "strict-bridge",
        "export function run(a,b) { const flag=a<b; return flag===!b?1:2; }",
        false,
        true,
    ),
    (
        "strict-effect",
        "export function run(a,b) { return !(a=1)===!(a=2)?a:9; }",
        false,
        true,
    ),
    (
        "assignment",
        "export function run(a,b) { let x=a; x=(a=b); return x+(a=x); }",
        false,
        true,
    ),
    (
        "compound",
        "export function run(a,b) { a+=(a=b); a-=b; a*=b; a/=b; a%=b; a&=b; a|=b; a^=b; a<<=b; a>>=b; a>>>=b; return a; }",
        false,
        true,
    ),
    (
        "updates",
        "export function run(a,b) { return a+++ ++b+a--+--b; }",
        false,
        true,
    ),
    (
        "number-logical",
        "export function run(a,b) { return (a&&(b=3))+(a||(b=4)); }",
        false,
        true,
    ),
    (
        "predicate-logical",
        "export function run(a,b) { return !a&&(!b||a<b)?1:2; }",
        false,
        true,
    ),
    (
        "number-regions",
        "export function run(a,b) { return a?(b?(a=3):(a=4)):(a=5); }",
        false,
        true,
    ),
    (
        "predicate-regions",
        "export function run(a,b) { if(a?(b?true:false):!a) return a; return b; }",
        false,
        true,
    ),
    (
        "number-comma",
        "export function run(a,b) { return (a=1,!b,a)+(b=2,b); }",
        false,
        true,
    ),
    (
        "predicate-comma",
        "export function run(a,b) { if((a=1,!b,a<b)) return a; return b; }",
        false,
        true,
    ),
    (
        "discard",
        "export function run(a,b) { const flag=true; flag; a; !a; (a=1,!b); a?true:!b; true&&true; return a; }",
        false,
        true,
    ),
    (
        "calls",
        "function f(x,y,z) { return x+y+z; } export function run(a,b) { return f(a,a=b,a++)+f(b,a,b); }",
        false,
        true,
    ),
    (
        "math",
        "export function run(a,b) { return Math.abs(a)+Math.sqrt(b)+Math.floor(a)+Math.ceil(b)+Math.trunc(a); }",
        false,
        true,
    ),
    (
        "wrappers",
        "export function run(a:number,b:number):number { const flag=((a<b) as boolean)!; return (+((flag satisfies boolean) as boolean))+((<number>a)!); }",
        true,
        true,
    ),
    (
        "loops",
        "export function run(a,b) { for(let x=0;x<b;x++) { if(x===a) continue; a+=x; } while(a<b) a++; do { b--; } while(b>a); return a+b; }",
        false,
        true,
    ),
    (
        "tdz",
        "export function run(a,b) { const flag=flag===true; return a; }",
        false,
        false,
    ),
    (
        "shadow",
        "export function run(a,b) { const flag=true; { const copy=flag; const flag=false; } return a; }",
        false,
        false,
    ),
    (
        "strict-mixed",
        "export function run(a,b) { return true===a?1:2; }",
        false,
        false,
    ),
    (
        "loose-boolean",
        "export function run(a,b) { return true==false?1:2; }",
        false,
        false,
    ),
    (
        "return-boolean",
        "export function run(a,b) { return true===false; }",
        false,
        false,
    ),
    (
        "operator-before-child",
        "export function run(a,b) { return Math.random() ** a; }",
        false,
        false,
    ),
    (
        "target-before-rhs",
        "export function run(a,b) { const x=1; x=Math.random(); return a; }",
        false,
        false,
    ),
    (
        "invalid-call-before-child",
        "export function run(a,b) { return Math.random(a ** b); }",
        false,
        false,
    ),
    (
        "arity-before-child",
        "function f(x) { return x; } export function run(a,b) { return f(a ** b,a); }",
        false,
        false,
    ),
    (
        "argument-before-spread",
        "function f(x,y) { return x+y; } export function run(a,b) { return f(a ** b,...a); }",
        false,
        false,
    ),
    (
        "spread-before-next",
        "function f(x,y) { return x+y; } export function run(a,b) { return f(...a,a ** b); }",
        false,
        false,
    ),
    (
        "math-arity",
        "export function run(a,b) { return Math.abs(a ** b,b); }",
        false,
        false,
    ),
    (
        "math-spread",
        "export function run(a,b) { return Math.abs(...a); }",
        false,
        false,
    ),
    (
        "skipped-invalid",
        "export function run(a,b) { return true?a:Math.random(); }",
        false,
        false,
    ),
    (
        "number-nullish",
        "export function run(a,b) { return a??b; }",
        false,
        false,
    ),
    (
        "predicate-nullish",
        "export function run(a,b) { return (true??false)?a:b; }",
        false,
        false,
    ),
];

#[test]
fn evaluator_control_characterization() {
    for &(id, source, typescript, admitted) in CASES {
        let (result, maximum, entries, peak) = observe(|| crate::lower_source(source, typescript));
        assert_eq!(result.is_ok(), admitted, "{id}: {result:?}");
        assert!(entries > 0, "real evaluator entered: {id}");
        assert_eq!(maximum, 1, "one active evaluator implementation: {id}");
        match result {
            Ok(module) => {
                let debug = format!("{module:?}");
                let wat = crate::emit::emit(&module).unwrap();
                let wasm = wat::parse_str(&wat).unwrap();
                println!("CONTROL|{id}|IR|{}", hex(debug.as_bytes()));
                println!("CONTROL|{id}|WAT|{}", hex(wat.as_bytes()));
                println!("CONTROL|{id}|WASM|{}", hex(&wasm));
            }
            Err(error) => {
                let expected = match id {
                    "tdz" => {
                        "unsupported at bytes 38..42: reference is not an initialized numeric parameter or local"
                    }
                    "shadow" => {
                        "unsupported at bytes 57..61: reference is not an initialized numeric parameter or local"
                    }
                    "strict-mixed" => {
                        "unsupported at bytes 34..38: expression is outside the numeric experiment"
                    }
                    "loose-boolean" => {
                        "unsupported at bytes 34..38: expression is outside the numeric experiment"
                    }
                    "return-boolean" => {
                        "unsupported at bytes 34..46: only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted"
                    }
                    "operator-before-child" => {
                        "unsupported at bytes 34..52: only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted"
                    }
                    "target-before-rhs" => {
                        "unsupported at bytes 38..53: assignment to a const binding is not admitted"
                    }
                    "invalid-call-before-child" => {
                        "unsupported at bytes 34..53: expression is outside the numeric experiment"
                    }
                    "arity-before-child" => {
                        "unsupported at bytes 62..73: call requires exactly the declared number of arguments"
                    }
                    "argument-before-spread" => {
                        "unsupported at bytes 68..74: only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted"
                    }
                    "spread-before-next" => {
                        "unsupported at bytes 66..80: spread call arguments are not admitted"
                    }
                    "math-arity" => {
                        "unsupported at bytes 34..52: Math.abs requires exactly one numeric argument"
                    }
                    "math-spread" => {
                        "unsupported at bytes 34..48: spread call arguments are not admitted"
                    }
                    "skipped-invalid" => {
                        "unsupported at bytes 41..54: expression is outside the numeric experiment"
                    }
                    "number-nullish" => {
                        "unsupported at bytes 34..38: expression is outside the numeric experiment"
                    }
                    "predicate-nullish" => {
                        "unsupported at bytes 35..46: expression is outside the numeric experiment"
                    }
                    _ => unreachable!("characterized diagnostic"),
                };
                assert_eq!(error, expected, "first complete diagnostic: {id}");
                println!("CONTROL|{id}|ERROR|{}", hex(error.as_bytes()));
            }
        }
        println!("MEASURE|{id}|{maximum}|{entries}|{peak}");
    }
}

#[test]
fn evaluator_entries_do_not_nest() {
    for body in [
        "return a+1;",
        "return (a);",
        "return +true;",
        "!a; return a;",
        "if(a) return a; return b;",
    ] {
        let source = format!("export function run(a,b) {{ {body} }}");
        let (result, maximum, _, _) = observe(|| crate::lower_source(&source, false));
        result.unwrap();
        assert_eq!(maximum, 1, "one active evaluator implementation: {body}");
    }
}
#[test]
fn nested_conditional_region_and_value_allocation_stays_ordered() {
    use crate::ir::{Exit, Op, Region};
    fn ids(region: &Region, found: &mut Vec<usize>) {
        found.push(region.id.0);
        for operation in &region.operations {
            if let Op::If {
                consequent,
                alternate,
                ..
            } = &operation.kind
            {
                ids(consequent, found);
                ids(alternate, found);
            }
        }
    }
    for id in ["number-regions", "predicate-regions"] {
        let (_, source, typescript, _) = CASES.iter().find(|case| case.0 == id).unwrap();
        let module = crate::lower_source(source, *typescript).unwrap();
        let body = &module.functions[0].body;
        let mut found = Vec::new();
        ids(body, &mut found);
        assert_eq!(
            found,
            if id == "number-regions" {
                vec![0, 1, 2, 3, 4]
            } else {
                vec![0, 1, 2, 3, 4, 5, 6]
            }
        );
        if id == "number-regions" {
            assert!(matches!(body.exit, Exit::Return(crate::ir::ValueId(8))));
            assert!(matches!(
                &body.operations[2].kind,
                Op::If {
                    result: Some(crate::ir::ValueId(8)),
                    condition: crate::ir::ValueId(1),
                    ..
                }
            ));
        }
    }
}

#[test]
fn predicate_fallback_keeps_the_original_depth_boundary() {
    for depth in [128, 129] {
        let source = format!(
            "export function run(a,b) {{ if({}a{}) return a; return b; }}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let (result, maximum, _, _) = observe(|| crate::lower_source(&source, false));
        assert_eq!(maximum, 1);
        if depth == 128 {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err(),
                "unsupported at bytes 159..160: predicate nesting exceeds this experiment's limit"
            );
        }
    }
}

#[test]
fn left_snapshot_precedes_right_assignment() {
    use crate::ir::{Op, SlotId, ValueId};
    let module =
        crate::lower_source("export function run(a,b) { return a+(a=b); }", false).unwrap();
    let operations = &module.functions[0].body.operations;
    assert!(matches!(
        operations[0].kind,
        Op::Load {
            result: ValueId(0),
            slot: SlotId(0)
        }
    ));
    assert!(matches!(
        operations[1].kind,
        Op::Load {
            result: ValueId(1),
            slot: SlotId(1)
        }
    ));
    assert!(matches!(
        operations[2].kind,
        Op::Store {
            slot: SlotId(0),
            value: ValueId(1)
        }
    ));
    assert!(matches!(
        operations[3].kind,
        Op::Binary {
            left: ValueId(0),
            right: ValueId(1),
            ..
        }
    ));
}

#[test]
fn characterization_setup_frontiers_keep_their_original_diagnostics() {
    for (source, expected) in [
        (
            "export function run(a,b) { const flag=a?(b?true:false):!a; return +flag; }",
            "unsupported at bytes 43..47: expression is outside the numeric experiment",
        ),
        (
            "export function run(a,b) { const flag=true; flag; a; !a; (a=1,flag); a?flag:!b; flag&&true; return a; }",
            "unsupported at bytes 62..66: Boolean bindings are not admitted in numeric value positions",
        ),
    ] {
        let (result, maximum, _, _) = observe(|| crate::lower_source(source, false));
        assert_eq!(result.unwrap_err(), expected);
        assert_eq!(maximum, 1);
    }
}

#[test]
fn numeric_logical_assignment_keeps_store_and_rhs_in_selected_region() {
    use crate::ir::{Exit, Op, SlotId, ValueId};
    for operator in ["&&=", "||="] {
        let source = format!("export function run(a,b) {{ return a {operator} b; }}");
        let (module, maximum, _, _) = observe(|| crate::lower_source(&source, false));
        let module = module.unwrap();
        assert_eq!(maximum, 1);
        let operations = &module.functions[0].body.operations;
        assert_eq!(
            operations.len(),
            3,
            "only captured Load/Truthy/If in parent"
        );
        assert!(matches!(
            operations[0].kind,
            Op::Load {
                result: ValueId(0),
                slot: SlotId(0)
            }
        ));
        assert!(matches!(
            operations[1].kind,
            Op::Truthy {
                input: ValueId(0),
                ..
            }
        ));
        let Op::If {
            consequent,
            alternate,
            result: Some(result),
            ..
        } = &operations[2].kind
        else {
            panic!("logical assignment If")
        };
        let (selected, skipped) = if operator == "&&=" {
            (consequent, alternate)
        } else {
            (alternate, consequent)
        };
        assert_eq!(selected.operations.len(), 2, "RHS Load and selected Store");
        assert!(matches!(
            selected.operations[0].kind,
            Op::Load {
                result: ValueId(2),
                slot: SlotId(1)
            }
        ));
        assert!(matches!(
            selected.operations[1].kind,
            Op::Store {
                slot: SlotId(0),
                value: ValueId(2)
            }
        ));
        assert!(
            matches!(selected.exit, Exit::Yield(ValueId(2))),
            "selected result is RHS snapshot"
        );
        assert!(skipped.operations.is_empty(), "no skipped RHS/Store");
        assert!(matches!(skipped.exit, Exit::Yield(ValueId(0))));
        assert!(matches!(module.functions[0].body.exit, Exit::Return(value) if value == *result));
    }
}

#[test]
fn numeric_logical_assignment_selected_result_is_the_stored_rhs() {
    use crate::ir::{Exit, Op};
    for operator in ["&&=", "||="] {
        let source = format!("export function run(a,b) {{ return a {operator} (a=b,a+1); }}");
        let module = crate::lower_source(&source, false).unwrap();
        let Op::If {
            consequent,
            alternate,
            ..
        } = &module.functions[0].body.operations[2].kind
        else {
            panic!("logical assignment If")
        };
        let selected = if operator == "&&=" {
            consequent
        } else {
            alternate
        };
        let Op::Store { value, .. } = &selected.operations.last().unwrap().kind else {
            panic!("selected final Store")
        };
        assert!(matches!(selected.exit, Exit::Yield(result) if result == *value));
    }
}
