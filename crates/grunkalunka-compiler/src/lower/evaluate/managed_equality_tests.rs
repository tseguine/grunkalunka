#[test]
fn reference_loose_equality_requires_two_known_reference_operands() {
    for (body, admitted) in [
        ("const o={};const alias=o;return o==alias?1:0;", true),
        ("const xs=[];return xs!=[]?1:0;", true),
        ("return {}==[]?1:0;", true),
        ("const d=Object.create(null);return d=={}?1:0;", true),
        ("const o={x:{}};return o.x==o.x?1:0;", true),
        ("const o={x:{}};delete o.x;return o.x=={}?1:0;", false),
        ("return {}==a?1:0;", false),
        ("return []==''?1:0;", false),
        ("const o={x:a?{}:false};return o.x=={}?1:0;", false),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
    assert!(
        crate::compile(
            "export function run(a,b){return {}==(a as unknown as object)?1:0;}",
            true
        )
        .is_err()
    );
}

#[test]
fn managed_loose_equality_requires_nullish_or_string_pair_proofs() {
    for (body, admitted) in [
        ("const o={x:null};return o.x==null?1:0;", true),
        ("const o={};return undefined==o.x?1:0;", true),
        ("const o={x:a?'text':null};return o.x!=null?1:0;", true),
        ("const o={x:a};return o.x==void b?1:0;", true),
        (
            "const o={x:'text'};const sentinel=a?null:undefined;return sentinel!=o.x?1:0;",
            true,
        ),
        ("const o={x:'text'};return o.x==a?1:0;", false),
        ("const o={x:'text'};return o.x=='text'?1:0;", true),
        ("return 'x'!='y'?1:0;", true),
        ("let x='old';return x==(x='new','old')?1:0;", true),
        ("const o={x:a?'text':false};return o.x=='text'?1:0;", false),
        (
            "const o={x:'text'};delete o.x;return o.x=='text'?1:0;",
            false,
        ),
        (
            "const o={x:a?'text':null};const undefined=b;return o.x==undefined?1:0;",
            false,
        ),
        (
            "const o={x:null};const undefined=b;return o.x==undefined?1:0;",
            true,
        ),
        (
            "const o={x:'text'};const sentinel=a?null:b;return o.x==sentinel?1:0;",
            false,
        ),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{source}: {error}");
            }
        }
    }
    assert!(
        crate::compile(
            "export function run(a,b){const o={x:'text'};return o.x==(a as unknown as null)?1:0;}",
            true
        )
        .is_err()
    );
    assert!(
        crate::compile(
            "export function run(a,b){return 'text'==(a as unknown as string)?1:0;}",
            true
        )
        .is_err()
    );
}
