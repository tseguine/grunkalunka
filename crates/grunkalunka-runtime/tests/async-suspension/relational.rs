use super::*;

const CYCLE: &str =
    "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";

#[test]
fn general_relational_errors_compare_their_primitive_strings() {
    let bodies = ["<", "<=", ">", ">="]
        .map(|op| format!("{CYCLE}try{{await p;}}catch(e){{return e{op}e;}}"));
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let plain = source_for(&bodies[1]);
    let typed = plain.replace(
        "return e<=e;",
        "return (e as unknown as number)<=(e as never);",
    );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
}

#[test]
fn general_relational_errors_keep_string_equality_separate_from_object_identity() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();let q=f();const g=async()=>{await a;return p;};const h=async()=>{await b;return q;};p=g();q=h();let first=null;try{await p;}catch(e){first=e;}";
    let bodies = ["<", "<=", ">", ">="]
        .map(|op| format!("{cycle}try{{await q;}}catch(e){{return e!==first&&(e{op}first);}}"));
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn general_relational_error_primitive_comparisons_preserve_numeric_nan() {
    let mut bodies = Vec::new();
    for op in ["<", "<=", ">", ">="] {
        for primitive in ["a", "null", "undefined", "false", "true", "+e"] {
            for expression in [format!("e{op}{primitive}"), format!("{primitive}{op}e")] {
                bodies.push(format!(
                    "{CYCLE}try{{await p;}}catch(e){{return {expression};}}"
                ));
            }
        }
        bodies.push(format!(
            "{CYCLE}try{{await p;}}catch(e){{return +e{op}+e;}}"
        ));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn general_relational_keeps_operand_snapshots_calls_and_suspension() {
    let tails = [
        "const compare=(x,y)=>x<=y;return compare(e,e);",
        "const compare=a?(x,y)=>x<=y:(x,y)=>x>=y;return compare(e,e);",
        "const compare=(x,y)=>x<=y;return compare(e,await a);",
        "let left=e;const change=()=>{left=a;return e;};return left<=change();",
        "let count=a;const take=x=>{count=count*10+x;return e;};const answer=take(1)<=take(2);return answer?count:-count;",
        "return e<=(await e);",
        "return (await e)<=e;",
        "return (b?e:0)<=e;",
        "return (e<=e)&&(e>=e)&&!(e<e)&&!(e>e);",
    ]
    .map(|tail| format!("{CYCLE}try{{await p;}}catch(e){{{tail}}}"));
    compare_promises(
        &tails.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}
