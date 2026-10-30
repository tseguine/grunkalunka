use super::*;

#[test]
fn discarded_logical_keeps_function_assignments_and_constructor_effects() {
    compare_function_programs(
        &[
            "let f=x=>x+b;a&&(f=x=>x-b);return f(a);",
            "let f=x=>x+b;a||(f=x=>x-b);return f(a);",
            "let f=x=>x+b;const left=a?null:false;left??(f=x=>x-b);return f(a);",
            "let f=x=>x+b;(a++,a)&&(f=x=>x-b);return f(a);",
            "let n=a;const make=x=>(n+=x,y=>y+x);a&&make(b);return n;",
            "let f=x=>x+b;const left=x=>x;left??(f=x=>x-b);return f(a);",
            "let f=x=>x+b;a&&(b||(f=x=>x-b));return f(a);",
            "const f=x=>x;const g=(x,y)=>x+y;f&&g;return f(a);",
        ],
        false,
    );
}

#[test]
fn discarded_logical_preserves_wrappers_type_erasure_and_source_guards() {
    let js = "export function run(a,b){let f=x=>x+b;((a&&(f=x=>x-b)));return f(a);}";
    let ts = "export function run(a:never,b:Wrong):unknown{let f=(x:Missing):unknown=>x+b;(((a&&(f=(x:Wrong):never=>x-b as never)) as Missing));return f(a);}";
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    for count in [128, 129] {
        let source = format!(
            "export function run(a,b){{let f=x=>x+b;{}(f=x=>x-b);return f(a);}}",
            "a&&".repeat(count)
        );
        if count == 128 {
            assert!(compile(&source, false).is_ok());
        } else {
            assert!(compile(&source, false).is_err());
        }
    }
    for body in [
        "false&&missing();return a;",
        "let f=x=>x;false&&(f=(x,y)=>x+y);return f(a);",
        "const f=x=>x;false&&(f=x=>x+b);return f(a);",
    ] {
        assert!(
            compile(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
}
