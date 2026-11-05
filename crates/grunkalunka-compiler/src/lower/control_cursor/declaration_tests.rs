//! Declarations schedule one initializer at a time while preserving publication.
use super::SCHEDULED_DECLARATION_VALUES;

#[test]
fn nested_object_bindings_preserve_admission_and_small_stack_depth_boundaries() {
    for (depth, admitted) in [(32, true), (128, true), (129, false)] {
        let mut pattern = String::from("value");
        let mut initializer = String::from("a");
        for _ in 0..depth {
            pattern = format!("{{next:{pattern}}}");
            initializer = format!("{{next:{initializer}}}");
        }
        let source = format!(
            "export function run(a,b){{const {pattern}={initializer};return value===a?1:0;}}"
        );
        let (result, _) = on_small_stack(&source, false, false);
        if admitted {
            assert!(result.is_ok(), "{depth}: {result:?}");
        } else {
            let error = result.unwrap_err();
            assert!(
                error.contains("binding pattern nesting exceeds 128"),
                "{error}"
            );
        }
    }
    for body in [
        "const {child:{}}={};return a;",
        "const {child:{}}={child:null};return a;",
        "const {child:{}}={child:[]};return a;",
        "const {child:{x}}={child:a};return a;",
        "const {child:{x}={x:a}}={child:{x:b}};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let (result, _) = on_small_stack(&source, false, false);
        let error = result.err().unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn for_headers_schedule_all_initializers_before_the_test() {
    for (source, asynchronous, expected) in [
        (
            "export function run(a,b){const step=x=>{return x+a;};let n=0;for(let x=step(a),y=x+b;x<b;x++){n+=y;}return n;}",
            false,
            3,
        ),
        (
            "export function run(a,b){for(let f=()=>a,n=0;n<1;n++){return f();}return b;}",
            false,
            2,
        ),
        (
            "export function run(a,b){let n=0;for(const x=a;n<1;n++){n+=x;}return n;}",
            false,
            2,
        ),
        (
            "export function run(a,b){let f=()=>a;for(let x=0;x<2;x++){f=()=>x;}return f();}",
            false,
            2,
        ),
        (
            "export async function run(a,b){let n=0;for(let x=await Promise.resolve(a),y=x+b;n<1;n++){return y;}return b;}",
            true,
            3,
        ),
    ] {
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(source, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{source}, typescript={typescript}: {result:?}"
            );
            assert_eq!(
                scheduled, expected,
                "for header bypassed ordered initializer scheduling: {source}"
            );
        }
    }
}

#[test]
fn for_header_initializers_do_not_publish_their_own_binding_early() {
    let source = "export function run(a,b){for(let x=x;x<b;x++){}return a;}";
    for typescript in [false, true] {
        let (result, scheduled) = on_small_stack(source, false, typescript);
        assert_eq!(
            result.unwrap_err(),
            "unsupported at bytes 35..36: reference is not an initialized numeric parameter or local"
        );
        assert_eq!(scheduled, 1);
    }
}

fn on_small_stack(
    source: &str,
    asynchronous: bool,
    typescript: bool,
) -> (Result<(), String>, usize) {
    let source = source.to_string();
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            SCHEDULED_DECLARATION_VALUES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            (
                result,
                SCHEDULED_DECLARATION_VALUES.with(std::cell::Cell::get),
            )
        })
        .expect("2 MiB declaration thread")
        .join()
        .expect("declaration initializers must not overflow the native stack")
}

#[test]
fn scalar_declaration_values_use_the_source_scheduler() {
    for (source, asynchronous) in [
        ("export function run(a,b){const x=a+b;return x;}", false),
        (
            "export function run(a,b){const step=x=>{return x+a;};const x=step(b),y=step(x);return x+y;}",
            false,
        ),
        (
            "export function run(a,b){const step=x=>{return x+a;};let x=step(b),y=step(x);return x+y;}",
            false,
        ),
        ("export function run(a,b){const x=a>b;return x?a:b;}", false),
        (
            "export function run(a,b){const x=a>b,y=x;return y?a:b;}",
            false,
        ),
        (
            "export function run(a,b){const x=a?null:b;return +x;}",
            false,
        ),
        (
            "export function run(a,b){const x=a,y=x+b,z=y+x;return z;}",
            false,
        ),
        (
            "export function run(a,b){let x=a,y=(x=b,x),z=(x=a,y);return x+y+z;}",
            false,
        ),
        (
            "export function run(a,b){let n=0;const x=(n=n*10+1,a),y=(n=n*10+2,x+b);return x+y+n;}",
            false,
        ),
    ] {
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(source, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{source}, typescript={typescript}: {result:?}"
            );
            assert!(
                scheduled > 0,
                "{source}: initializer bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn function_declaration_values_use_the_source_scheduler() {
    for (source, asynchronous) in [
        ("export function run(a,b){let f=x=>x+a;return f(b);}", false),
        (
            "export function run(a,b){let f=x=>x+a,g=f;f=x=>x-b;return g(a)+f(b);}",
            false,
        ),
        (
            "export function run(a,b){let f=a?(x=>x+a):(x=>x-b);return f(b);}",
            false,
        ),
        (
            "export function run(a,b){const f=a?(x=>x+a):(x=>x-b);return f(b);}",
            false,
        ),
        (
            "export function run(a,b){let n=0;const f=(n=n*10+1,x=>x+n);return f(a)+n;}",
            false,
        ),
        (
            "export function run(a,b){const step=x=>{let f=y=>x+y;return f(b);};const value=step(a);return value;}",
            false,
        ),
    ] {
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(source, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{source}, typescript={typescript}: {result:?}"
            );
            assert!(
                scheduled > 0,
                "{source}: initializer bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn managed_declaration_values_use_the_source_scheduler() {
    for (source, asynchronous) in [
        (
            "export function run(a,b){const o={x:a},v=o.x;return v===a?a:b;}",
            false,
        ),
        (
            "export function run(a,b){const xs=[a,b],v=xs[0];return v===a?a:b;}",
            false,
        ),
        (
            "export function run(a,b){const o={x:a},f=()=>o.x===a?a:b;o.x=b;return f();}",
            false,
        ),
        (
            "export function run(a,b){let o={x:a},p=o;o={x:b};return p.x===a?a:b;}",
            false,
        ),
        (
            "export function run(a,b){const x=a?{v:b}:[b];return x===x?a:b;}",
            false,
        ),
        (
            "export function run(a,b){let n=0;const o={x:(n=n*10+1,a)},p={x:(n=n*10+2,b)};return (o===p?a:b)+n;}",
            false,
        ),
    ] {
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(source, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{source}, typescript={typescript}: {result:?}"
            );
            assert!(
                scheduled > 0,
                "{source}: initializer bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn async_declaration_values_use_the_source_scheduler() {
    for (source, asynchronous) in [
        (
            "export async function run(a,b){const f=async x=>{await x;return x+b;};const p=f(a),q=f(b);return(await p)+(await q);}",
            true,
        ),
        (
            "export async function run(a,b){let n=0;const f=async x=>{n=n*10+x;await x;return n;};const p=f(1),q=(n=n*10+2,f(3));return(await p)+(await q)+n;}",
            true,
        ),
        (
            "export async function run(a,b){let x=a,y=await Promise.resolve(x+b);return x+y;}",
            true,
        ),
        (
            "export async function run(a,b){const p=Promise.resolve(a),f=async()=>await p;return await f();}",
            true,
        ),
        (
            "export async function run(a,b){const seed=async x=>{await x;return x+a;};let f=async y=>await seed(y);return await f(b);}",
            true,
        ),
        (
            "export async function run(a,b){const o={x:a},f=async()=>o.x===a?a:b;return await f();}",
            true,
        ),
    ] {
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(source, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{source}, typescript={typescript}: {result:?}"
            );
            assert!(
                scheduled > 0,
                "{source}: initializer bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn declaration_publication_preserves_first_diagnostics() {
    for (source, asynchronous, expected) in [
        (
            "export function run(a,b){let x=x;return x;}",
            false,
            "unsupported at bytes 31..32: reference is not an initialized numeric parameter or local",
        ),
        (
            "export function run(a,b){const x=y,y=a;return x;}",
            false,
            "unsupported at bytes 33..34: reference is not an initialized numeric parameter or local",
        ),
        (
            "export function run(a,b){const x=a;{const x=x;return x;}}",
            false,
            "unsupported at bytes 44..45: reference is not an initialized numeric parameter or local",
        ),
        (
            "export function run(a,b){const f=(x,y)=>x+y;const x=f(a),y=Math.noSuch(b);return x+y;}",
            false,
            "unsupported at bytes 52..56: local call is missing declared arguments",
        ),
        (
            "export async function run(a,b){const p=q,q=Promise.resolve(a);return await p;}",
            true,
            "unsupported at bytes 39..40: reference is not an initialized numeric parameter or local",
        ),
    ] {
        for typescript in [false, true] {
            assert_eq!(
                on_small_stack(source, asynchronous, typescript)
                    .0
                    .unwrap_err(),
                expected,
                "{source}, typescript={typescript}"
            );
        }
    }
}

#[test]
fn declaration_initializers_keep_original_depth_boundaries() {
    for (prefix, expression, suffix, admitted, rejected, asynchronous, expected) in [
        (
            "export function run(a,b){let f=",
            "x=>x+a",
            ";return f(b);}",
            128,
            129,
            false,
            "unsupported at bytes 160..166: expression nesting exceeds this experiment's limit",
        ),
        (
            "export async function run(a,b){const p=",
            "Promise.resolve(a)",
            ";return await p;}",
            127,
            128,
            true,
            "unsupported at bytes 183..184: expression nesting exceeds this experiment's limit",
        ),
        (
            "export function run(a,b){const xs=",
            "[a,b]",
            ";return xs[0]===a?a:b;}",
            127,
            128,
            false,
            "unsupported at bytes 163..164: expression nesting exceeds this experiment's limit",
        ),
    ] {
        for depth in [admitted, rejected] {
            let source = [
                prefix,
                &"(".repeat(depth),
                expression,
                &")".repeat(depth),
                suffix,
            ]
            .concat();
            for typescript in [false, true] {
                let (result, scheduled) = on_small_stack(&source, asynchronous, typescript);
                if depth == admitted {
                    assert!(result.is_ok(), "{source}: {result:?}");
                    assert!(scheduled > 0, "wrapped initializer bypassed the scheduler");
                } else {
                    assert_eq!(
                        result.unwrap_err(),
                        expected,
                        "{source}, typescript={typescript}"
                    );
                }
            }
        }
    }
}

#[test]
fn declaration_locals_keep_their_source_charge_and_error_order() {
    for count in [128, 129] {
        let locals = (0..count)
            .map(|index| {
                if index == 0 {
                    "x0=a".to_owned()
                } else {
                    format!("x{index}=x{}", index - 1)
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {locals};return x{};}}",
            count - 1
        );
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(&source, false, typescript);
            if count == 128 {
                assert!(result.is_ok(), "{result:?}");
                assert_eq!(scheduled, 128);
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 1088..1097: at most 128 source locals are admitted"
                );
            }
        }
    }
    let locals = (0..128)
        .map(|index| format!("x{index}=a"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("export function run(a,b){{const {locals},bad=Math.noSuch(a);return b;}}");
    for typescript in [false, true] {
        assert_eq!(
            on_small_stack(&source, false, typescript).0.unwrap_err(),
            "unsupported at bytes 817..835: at most 128 source locals are admitted"
        );
    }
}
