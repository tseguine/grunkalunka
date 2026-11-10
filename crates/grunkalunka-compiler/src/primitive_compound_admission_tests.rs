//! Fast source-admission guards for primitive compound assignments.
//! Keep these in compiler unit tests so the PR profile catches stale rejection
//! expectations before full master runtime verification.
use crate::compile;

#[test]
fn string_compound_addition_is_admitted() {
    for body in [
        "let value=null; value+=\"x\"; return a;",
        "let value=null; value+=\"x\"; return value===\"nullx\"?a:b;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        for ts in [false, true] {
            compile(&source, ts).unwrap_or_else(|error| panic!("{source}: {error}"));
        }
    }
}

#[test]
fn primitive_compound_boundaries_still_reject_invalid_sources() {
    for body in [
        "let value=null; value**=a; return a;",
        "const value=null; value+=a; return a;",
        "value+=a; let value=null; return a;",
        "let value=(value+=a); return a;",
        "let value=null; value.x+=a; return a;",
        "let value=null; value+=1n; return a;",
        "let value=null; value+=({}); return a;",
        "a+=(\"x\" as number); return a;",
        "if(false){a+=Math.random();} return a;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, true)
            .err()
            .unwrap_or_else(|| panic!("previously rejected source now compiles: {source}"));
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}
