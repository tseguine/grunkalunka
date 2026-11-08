#[test]
fn var_identifiers_are_hoisted_and_redeclarations_write_existing_cells() {
    for body in [
        "var x=a;return x+b;",
        "var x=a;return x;",
        "const saved=x;var x=3;return saved;",
        "const before=x===undefined?1:0;var x=a;return before;",
        "var x=1;var x;return x;",
        "var x=1;var x=2;return x;",
        "x=a;var x;return x;",
        "var x; x++;return x;",
        "{var x=a;}return x;",
        "if(a){var x=b;}return x;",
        "for(var i=0;i<3;i++){}return i;",
        "var x='abc';return x.length;",
        "var x={a:a};return x.a;",
        "var x=[a,b];return x[0];",
        "const f=()=>x;var x=3;return f();",
        "const f=()=>{var x=3;return x;};return f();",
        "const f=()=>{const saved=x;var x=3;return saved;};return f();",
        "var a;return a;",
        "var a=null;return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = crate::compile_value(&source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}
#[test]
fn var_helpers_and_default_parameter_body_environments_are_distinct() {
    for source in [
        "function f(x){var y=x;return y;}export function run(a,b){return f(a);}",
        "function f(x){const saved=y;var y=x;return saved;}export function run(a,b){return f(a);}",
        "function f(x){var x;return x;}export function run(a,b){return f(a);}",
        "function f(x=3){var x;return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=3)=>{var x;return x;};return f();}",
    ] {
        let result = crate::compile_value(source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn var_number_proofs_follow_initialization_and_account_for_all_writes() {
    for body in [
        "var x=a;return x;",
        "x=a;var x;return x;",
        "var x=a; x=b;return x;",
        "var x; x++;return x;",
        "for(var i=0;i<3;i++){}return i;",
        "var x=1;var x=2;return x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let result = crate::compile(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    for body in [
        "var x;return x;",
        "if(a){var x=b;}return x;",
        "const saved=x;var x=3;return saved;",
        "const change=()=>{x=null;};var x=3;change();return x;",
        "var x=3;({x}={});return x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::compile(&source, false).is_err(), "{source}");
    }
}

#[test]
fn var_keeps_unreachable_iteration_async_callable_and_lexical_guards() {
    for body in [
        "return x;var x=3;",
        "var f=x=>x;return f(a);",
        "try{throw a;}catch(e){var e=3;}return 0;",
        "var x=1;let x=2;return x;",
        "var x=1;{const saved=x;let x=2;}return x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::compile_value(&source, false).is_err(), "{source}");
    }
    assert!(
        crate::compile_async("export async function run(a,b){var x=a;return x;}", false).is_err()
    );
}

#[test]
fn var_annotations_erase_without_changing_hoisting_or_parameter_cells() {
    for (js, ts) in [
        (
            "export function run(a,b){let n=0;for(var key in {x:a,y:b})n+=key==='x'?1:2;return n;}",
            "export function run(a:string,b:boolean):number{let n:boolean=0;for(var key in ({x:a,y:b} as never))n+=(key as number)==='x'?1:2;return n;}",
        ),
        (
            "export function run(a,b){let n=0;for(var item of [a,b])n+=+item;return n;}",
            "export function run(a:string,b:boolean):number{let n:boolean=0;for(var item of ([a,b] as string[]))n+=+(item as number);return n;}",
        ),
        (
            "export function run(a,b){const before=x;var x=a;return before;}",
            "export function run(a:number,b:number):unknown{const before=x;var x:string=a;return before;}",
        ),
        (
            "function f(x=3){var x;return x;}export function run(a,b){return f();}",
            "function f(x:boolean=3):unknown{var x:string;return x;}export function run(a:number,b:number):unknown{return f();}",
        ),
    ] {
        assert_eq!(
            crate::compile_value(js, false).unwrap().wasm,
            crate::compile_value(ts, true).unwrap().wasm
        );
    }
}

#[test]
fn var_source_binding_limits_are_controlled_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [128, 129] {
                let names = (0..count)
                    .map(|i| format!("x{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let source = format!("export function run(a,b){{var {names};return a;}}");
                let loop_names = (0..count - 1)
                    .map(|i| format!("x{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let loop_source = format!(
                    "export function run(a,b){{var {loop_names};for(var item of [a]){{}}return a;}}"
                );
                let in_source=format!("export function run(a,b){{var {loop_names};for(var item in {{x:a}}){{}}return a;}}");
                let pattern_source=format!("export function run(a,b){{var [{names}]=[];return a;}}");
                let pattern_loop_source=format!("export function run(a,b){{var {loop_names};for(var [item] of [[a]]){{}}return a;}}");
                let fields = (0..count).map(|i|format!("length:x{i}")).collect::<Vec<_>>().join(",");
                let in_pattern_source=format!("export function run(a,b){{for(var {{{fields}}} in {{a:1}}){{}}return a;}}");
                for source in [source, loop_source, in_source, pattern_source, pattern_loop_source, in_pattern_source] {
                    for ts in [false, true] {
                        let result = crate::compile_value(&source, ts);
                        if count == 128 {
                            result.unwrap();
                        } else {
                            assert!(result.err().unwrap().contains("at most 128 source locals"));
                        }
                    }
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn var_for_of_reuses_function_cells_and_preserves_kind_boundaries() {
    for body in [
        "for(var item of [a]){}return a;",
        "for(var item of [a,b]){}return +item;",
        "const before=item;for(var item of []){}return before;",
        "var item=3;for(var item of []){}return item;",
        "var a;for(var a of [null,b]){}return a;",
        "let n=0;for(var item of [a,b])n+=+item;return n;",
        "let n=0;for(var item of 'abc')n+=item==='a'?1:2;return n;",
        "const f=()=>item;for(var item of [a,b]){}return f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = crate::compile_value(&source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
    // A loop's writes must widen earlier Number origins, even for captures.
    for body in [
        "var item=3;for(var item of [null]){}return item;",
        "const f=()=>item;var item=3;for(var item of [null]){}return f();",
        "var item;for(var item of []){}return item;",
        "for(var a of [null]){}return a;",
    ] {
        assert!(
            crate::compile(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    assert!(
        crate::compile_value(
            "export function run(a,b){for(var x of a){}return a;}",
            false
        )
        .is_err()
    );
}

#[test]
fn var_for_in_reuses_hoisted_cells_and_accounts_for_key_writes() {
    for body in [
        "for(var key in {x:a}){}return key==='x'?1:0;",
        "const before=key;for(var key in {}){}return before===undefined?1:0;",
        "var key=3;for(var key in {}){}return key===3?1:0;",
        "const get=()=>a;for(var a in {x:b}){}return get()==='x'?1:0;",
        "let n=0;for(var key in [a,b])n+=key==='0'?1:2;return n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = crate::compile_value(&source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
    for body in [
        "var key=3;for(var key in {x:a}){}return key;",
        "const get=()=>key;var key=3;for(var key in {x:a}){}return get();",
        "for(var a in {x:b}){}return a;",
    ] {
        assert!(
            crate::compile(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    for body in [
        "for(var {x} in {x:a}){}return a;",
        "try{throw a;}catch(e){for(var e in {x:b}){}}return a;",
        "const source=a?{x:b}:null;for(var key in source){}return a;",
    ] {
        assert!(
            crate::compile_value(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
}

#[test]
fn var_patterns_preserve_function_cells_and_selected_leaf_origins() {
    for body in [
        "var {x}={x:a};return +x;",
        "var [x,y]=[a,b];return +x+(+y);",
        "const get=()=>x;var {x}={x:a};var {x}={x:b};return get();",
        "const before=x;var [x]=[a];return before;",
        "var [x=x]=[];return x;",
        "var [x,x]=[3,null];return x;",
        "var [x=3,y=x]=[];return +y;",
        "var {x,...rest}={x:a,y:b};return rest.y;",
        "var [x,...rest]=[a,b];return rest[0];",
        "var [a]=[null];return a;",
        "for(var [x]=[a];0;){}return +x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let r = crate::compile_value(&source, ts);
            assert!(r.is_ok(), "{source}: {:?}", r.err());
        }
    }
    for body in ["var {x}=a;return a;", "var {x}={x:a};let x=b;return x;"] {
        assert!(
            crate::compile_value(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    assert!(
        crate::compile(
            "export function run(a,b){const get=()=>x;var [x]=[3];var [x]=[null];return get();}",
            false
        )
        .is_err()
    );
    assert!(
        crate::compile(
            "export function run(a,b){var [x,x]=[3,null];return x;}",
            false
        )
        .is_err()
    );
}

#[test]
fn var_pattern_types_erase_and_lexical_bindings_keep_tdz() {
    let loop_js = "export function run(a,b){let n=0;for(var [x] of [[a],[b]])n+=+x;return n;}";
    let loop_ts = "export function run(a:string,b:boolean):never{let n:boolean=0;for(var [x] of ([[a],[b]] as never))n+=+(x as string);return n;}";
    assert_eq!(
        crate::compile(loop_js, false).unwrap().wasm,
        crate::compile(loop_ts, true).unwrap().wasm
    );
    let js = "export function run(a,b){var [x,y]=[a,b];return +x+(+y);}";
    let ts = "export function run(a:string,b:boolean):void{var [x,y]:never=([a,b] as unknown);return +(x as string)+(+(y as boolean));}";
    assert_eq!(
        crate::compile(js, false).unwrap().wasm,
        crate::compile(ts, true).unwrap().wasm
    );
    assert!(
        crate::compile_value("export function run(a,b){let [x=x]=[];return x;}", false).is_err()
    );
    assert!(
        crate::compile_value(
            "export function run(a,b){try{throw a;}catch(e){var [e]=[b];}return a;}",
            false
        )
        .is_err()
    );
}

#[test]
fn var_for_of_patterns_share_cells_and_preserve_iteration_origins() {
    for body in [
        "let n=0;for(var [x] of [[a],[b]])n+=+x;return n;",
        "for(var {x} of [{x:a},{x:b}]){}return +x;",
        "const before=x;for(var [x] of (a?[]:[[b]])){}return before;",
        "var x=3;for(var [x] of (a?[]:[[b]])){}return +x;",
        "const get=()=>x;for(var [x] of [[a],[b]]){}return get();",
        "for(var [x=3,y=x] of [[]]){}return +y;",
        "for(var {row:[x]} of [{row:[a]}]){}return +x;",
        "for(var [a] of [[null]]){}return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let r = crate::compile_value(&source, ts);
            assert!(r.is_ok(), "{source}: {:?}", r.err());
        }
    }
    for body in [
        "var x=3;for(var [x] of [[null]]){}return x;",
        "const get=()=>x;var x=3;for(var [x] of [[null]]){}return get();",
    ] {
        assert!(
            crate::compile(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    for body in [
        "for(var [x] of 'ab'){}return a;",
        "for(var [x] of [a]){}return a;",
        "for(var [x] in {x:a}){}return a;",
    ] {
        assert!(
            crate::compile_value(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
}

#[test]
fn string_object_bindings_use_own_utf16_properties_and_keep_guards() {
    for body in [
        "const {length:n}='abc';return n;",
        "let {0:x='?'}='abc';return x.charCodeAt(0);",
        "var {length:n}='abc';return +n;",
        "const {s:{length:n}}={s:'abc'};return n;",
        "const {'99':x='z'}='abc';return x.charCodeAt(0);",
        "let n=0;const {0:x=(n++, '?'),length:l}='ab';return n+l;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let r = crate::compile(&source, ts);
            assert!(r.is_ok(), "{source}: {:?}", r.err());
        }
    }
    for body in [
        "const {...rest}='abc';return a;",
        "const {slice}='abc';return a;",
        "const {'01':x}='abc';return a;",
        "const key='length';const {[key]:n}='abc';return a;",
        "const {length:n}=(a as string);return a;",
        "const source=a?'abc':null;const {length:n}=source;return a;",
    ] {
        assert!(
            crate::compile_value(&format!("export function run(a,b){{{body}}}"), true).is_err(),
            "{body}"
        );
    }
    let js = "export function run(a,b){const {length:n}='abc';return n;}";
    let ts = "export function run(a:string,b:boolean):never{const {length:n}:never=('abc' as unknown);return n as string;}";
    assert_eq!(
        crate::compile(js, false).unwrap().wasm,
        crate::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn for_in_object_patterns_use_string_keys_and_preserve_source_guards() {
    for body in [
        "let n=0;for(const {length:l} in {a:1,abc:2})n+=l;return n;",
        "let n=0;for(let {0:x='?'} in {'':1,abc:2})n+=x.charCodeAt(0);return n;",
        "const get=()=>n;for(var {length:n} in {a:1,abc:2}){}return +get();",
        "let n=0;for(const {0:{length:l}='z'} in {'':1,abc:2})n+=l;return n;",
        "for(var {} in 17){}return a;",
        "var n=3;for(var {length:n} in {}){}return n===3?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = crate::compile(&source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
    for body in [
        "for(var [x] in {a:1}){}return a;",
        "for(const {...rest} in {a:1}){}return a;",
        "for(let {slice:x} in {a:1}){}return a;",
        "const key='length';for(const {[key]:x} in {a:1}){}return a;",
        "for(let {'01':x} in {a:1}){}return a;",
        "for(const {length:n} in ((a?'abc':null) as object)){}return a;",
        "let n=3;for(const {length:n} in {a:n}){}return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::compile_value(&source, true).is_err(), "{source}");
    }
    for body in [
        "var x=3;for(var {0:x='?'} in {a:1}){}return x;",
        "const get=()=>x;var x=3;for(var {0:x='?'} in {a:1}){}return get();",
    ] {
        assert!(
            crate::compile(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    let js = "export function run(a,b){let n=0;for(let {length:l} in {abc:a})n+=l;return n;}";
    let ts = "export function run(a:string,b:boolean):never{let n:boolean=0;for(let {length:l} in ({abc:a} as never))n+=(l as string);return n as boolean;}";
    assert_eq!(
        crate::compile(js, false).unwrap().wasm,
        crate::compile(ts, true).unwrap().wasm
    );
}
