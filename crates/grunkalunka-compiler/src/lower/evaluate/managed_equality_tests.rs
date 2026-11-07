#[test]
fn boolean_logical_writes_preserve_only_boolean_cells_and_results() {
    for (body, admitted) in [
        ("let [flag]=[true];flag&&=false;return +flag;", true),
        ("let {flag}={flag:false};return +(flag||=true);", true),
        (
            "let [flag]=[false];flag??=true;return flag==false?1:0;",
            true,
        ),
        ("let [flag]=[true];flag&&=a;return flag==true?1:0;", false),
        ("let [flag]=[true];flag++;return flag==true?1:0;", false),
        ("let [flag]=[true];flag-=1;return flag==true?1:0;", false),
        (
            "let [flag]=[a?true:b];flag||=false;return flag==true?1:0;",
            false,
        ),
        ("const [flag]=[false];flag||=true;return +flag;", false),
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
    assert!(crate::compile("export function run(a,b){let [flag]=[true];flag&&=(a as unknown as boolean);return flag==true?1:0;}",true).is_err());
}

#[test]
fn boolean_projection_and_await_require_positive_boolean_origins() {
    for (body, admitted) in [
        ("const o={flag:true};return +o.flag;", true),
        ("const xs=[a<b];return +xs[0];", true),
        ("const o={flag:a?true:b};return +o.flag;", false),
        ("const o={flag:true};delete o.flag;return +o.flag;", false),
        (
            "const get=()=>{if(a)return true;};const o={flag:get()};return +o.flag;",
            false,
        ),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_eq!(crate::compile(&source, typed).is_ok(), admitted, "{source}");
        }
    }
    for (body, admitted) in [
        ("const o={flag:await true};return o.flag==true?1:0;", true),
        ("const o={flag:await(await (a<b))};return +o.flag;", true),
        ("const o={flag:true};return +(await o.flag);", true),
        (
            "const o={flag:await (a?true:b)};return o.flag==true?1:0;",
            false,
        ),
        (
            "const o={flag:await {then:resolve=>resolve(true)}};return o.flag==true?1:0;",
            false,
        ),
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile_async(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }
    assert!(
        crate::compile(
            "export function run(a,b){const o={flag:a};return +(o.flag as boolean);}",
            true
        )
        .is_ok()
    );
    assert!(
        crate::compile(
            "export function run(a,b){const o={flag:'text'};return +(o.flag as boolean);}",
            true
        )
        .is_err()
    );
}

#[test]
fn boolean_loose_equality_requires_positive_boolean_pairs() {
    for (body, admitted) in [
        ("const o={flag:true};return o.flag==true?1:0;", true),
        ("const xs=[a<b,false];return xs[0]!=xs[1]?1:0;", true),
        ("const o={flag:!a};return o.flag==!!b?1:0;", true),
        ("const o={flag:a?true:false};return o.flag==true?1:0;", true),
        ("const o={flag:true??a};return o.flag==true?1:0;", true),
        // Boolean kind proves non-nullishness, not which truthy branch runs.
        ("const o={flag:true||a};return o.flag==true?1:0;", false),
        ("const o={flag:false&&a};return o.flag==false?1:0;", false),
        ("const o={flag:true};return o.flag==1?1:0;", false),
        ("const o={flag:a?true:b};return o.flag==true?1:0;", false),
        (
            "const o={flag:true};delete o.flag;return o.flag==true?1:0;",
            false,
        ),
        (
            "const o={flag:true};o.flag='text';return o.flag==true?1:0;",
            false,
        ),
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
    assert!(crate::compile("export function run(a,b){const o={flag:true};return o.flag==(a as unknown as boolean)?1:0;}",true).is_err());
}

#[test]
fn function_loose_equality_requires_reference_proofs_and_supported_functions() {
    for (body, admitted) in [
        ("const f=x=>x;const alias=f;return f==alias?1:0;", true),
        ("const f=x=>x;return f!=(()=>a)?1:0;", true),
        ("const f=x=>x;return f!={}&&f!=[]?1:0;", true),
        ("const f=x=>x;const value=a?f:{};return value==f?1:0;", true),
        ("const o={f:x=>x};return o.f==o.f?1:0;", true),
        ("const f=x=>x;return f==a?1:0;", false),
        ("const f=x=>x;return f=='function'?1:0;", false),
        ("const o={f:x=>x};delete o.f;return o.f==(x=>x)?1:0;", false),
        (
            "const f=x=>x;const value=a?f:false;return value==f?1:0;",
            false,
        ),
        (
            "const f=async()=>{await a;return b;};return f==f?1:0;",
            false,
        ),
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
            "export function run(a,b){const f=x=>x;return f==(a as unknown as Function)?1:0;}",
            true
        )
        .is_err()
    );
}

#[test]
fn boxed_number_loose_equality_requires_positive_number_pairs() {
    for (body, admitted) in [
        ("const o={n:a};return o.n==b?1:0;", true),
        ("const xs=[a,b];return xs[0]!=xs[1]?1:0;", true),
        ("const o={n:NaN};return o.n==o.n?1:0;", true),
        ("const o={n:a};return o.n=='0'?1:0;", false),
        ("const o={n:a?b:false};return o.n==b?1:0;", false),
        ("const o={n:a};delete o.n;return o.n==b?1:0;", false),
        ("const o={};return o.n==b?1:0;", true),
        ("const o={n:a};o.n='text';return o.n==b?1:0;", false),
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
            "export function run(a,b){const o={n:a};return o.n==('0' as unknown as number)?1:0;}",
            true
        )
        .is_err()
    );
}

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
#[test]
fn string_relational_operators_require_positive_string_pairs() {
    for operator in ["<", "<=", ">", ">="] {
        for (prefix, left, right, admitted) in [
            ("", "'a'", "'b'", true),
            ("const o={s:'a'};", "o.s", "'b'", true),
            ("const [s]=['a'];", "s", "'b'", true),
            ("const f=x=>x;", "f('a')", "'b'", true),
            ("", "a", "'b'", false),
            ("", "'a'", "null", false),
            ("const o={s:a?'a':b};", "o.s", "'b'", false),
            ("const o={s:'a'};delete o.s;", "o.s", "'b'", false),
            ("const o={s:'a'};o.s=b;", "o.s", "'b'", false),
            ("const f=()=>{if(a)return 'a';};", "f()", "'b'", false),
        ] {
            let source =
                format!("export function run(a,b){{{prefix}return {left}{operator}{right}?1:0;}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
                if let Err(error) = result {
                    assert!(!error.contains("internal IR"), "{error}");
                }
            }
        }
        let source = format!(
            "export function run(a,b){{return (a as unknown as string){operator}'b'?1:0;}}"
        );
        assert!(crate::compile(&source, true).is_err());
        let source = format!(
            "export async function run(a,b){{return (await 'a'){operator}(await 'b')?1:0;}}"
        );
        assert!(crate::compile_async(&source, false).is_ok());
    }
}
