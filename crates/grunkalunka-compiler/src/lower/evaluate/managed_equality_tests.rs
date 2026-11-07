#[test]
fn managed_loose_null_checks_require_a_positive_nullish_operand() {
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
        ("const o={x:'text'};return o.x=='text'?1:0;", false),
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
}
