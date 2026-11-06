#[test]
fn awaited_numbers_retain_proofs_without_assuming_thenable_results() {
    for (body, admitted) in [
        ("const o={x:await a};return o.x+b;", true),
        ("let [x]=[a];return x+=await b;", true),
        ("let [x]=[a];const result=x&&=await b;return +result;", true),
        (
            "const o={x:a};const result=o.x||=await b;return +result;",
            true,
        ),
        ("const o={x:await (await a)};return o.x+b;", true),
        ("const o={x:await 'text'};return o.x+b;", false),
        ("const o={x:await (a?b:'text')};return o.x+b;", false),
        (
            "const o={x:await {then:resolve=>resolve(a)}};return o.x+b;",
            false,
        ),
        ("const o={x:await {n:a}};return o.x+b;", false),
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile_async(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }
    let source = "export async function run(a,b){const o={x:await ('text' as unknown as number)};return o.x+b;}";
    assert!(crate::compile_async(source, true).is_err());
}

#[test]
fn numeric_fields_and_patterns_use_positive_source_proofs() {
    for (body, admitted) in [
        ("return ({x:a}).x+b;", true),
        ("const xs=[a,b];return xs[1];", true),
        ("const {x}={x:a};return x+b;", true),
        ("let x=a;[x]=[b];return x;", true),
        ("const {x=b}={x:a};return x+b;", true),
        ("const {x=b}={};return x+b;", true),
        ("const o={x:a};o.x=b;return o.x+b;", true),
        ("const get=()=>a+b;const o={x:get()};return o.x+b;", true),
        ("const o={x:a};delete o.x;return o.x+b;", false),
        (
            "const o={x:a};const alias=o;alias.x='bad';return o.x+b;",
            false,
        ),
        (
            "const o={x:a};const change=t=>{t.x=null;return 0;};change(o);return o.x+b;",
            false,
        ),
        (
            "const o={x:a};const key='x';delete o[key];return o.x+b;",
            false,
        ),
        ("const o=a?{x:a}:{x:'bad'};return o.x+b;", false),
        ("const xs=[a,,b];return xs[1]+b;", false),
        ("const xs=[a,b];xs.length=0;return xs[0]+b;", false),
        ("const xs=[a,b];xs.reverse();return xs[0]+b;", false),
        ("const o={};return o.x+b;", false),
        ("const o={x:{}};return o.x+b;", false),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn type_annotations_cannot_supply_a_boxed_number_proof() {
    for body in [
        "const o={x:('bad' as unknown as number)};return o.x+b;",
        "const o={x:a};const alias=o;alias.x=('bad' as unknown as number);return o.x+b;",
        "const o={x:a};delete o.x;return (o.x as number)+b;",
    ] {
        let source = format!("export function run(a:number,b:number){{{body}}}");
        assert!(crate::compile(&source, true).is_err(), "{source}");
    }
}
