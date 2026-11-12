#[test]
fn private_primitive_defaults_admit_omitted_and_supplied_values() {
    for source in [
        "function f(x=3){return x;}export function run(a,b){return f();}",
        "function f(x=3){return x+1;}export function run(a,b){return f(a);}",
        "function f(x=-0){return x;}export function run(a,b){return f();}",
        "function f(x=true){return x;}export function run(a,b){return f();}",
        "function f(x=null){return x;}export function run(a,b){return f();}",
        "function f(x=undefined){return x;}export function run(a,b){return f();}",
        "function f(x='abc'){return x.length;}export function run(a,b){return f();}",
        "function f(x=3){return x;}export function run(a,b){const g=f;return g();}",
        "function f(x=3,y=false){return x;}export function run(a,b){return f(a);}",
        "function f(x=3){return x;}export function run(a,b){return a;}",
        "function f(x=3){return x;}export function run(a,b){return f.length;}",
        "function f(x=3){return x;}export function run(a,b){const g=f;return g.length;}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn helper_default_scope_and_source_guards_remain_explicit() {
    for source in [
        "export function run(a=1,b){return a;}",
        "function f(x=3){return x;}function g(x){return x;}export function run(a,b){const h=a?f:g;return h.length;}",
        "function f(...x){return 1;}export function run(a,b){return f();}",
        "function f({x}=null){return 1;}export function run(a,b){return f();}",
        "function f(x=a){return x;}export function run(a,b){return f();}",
        "function f(x=Math.random()){return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=a)=>x;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn helper_default_wrappers_respect_source_depth_and_erased_annotations() {
    let js = "function f(x=3){return x;}export function run(a,b){return f(a);}";
    let ts = "function f<T>(x:never=3):Missing{return x as never;}export function run(a:unknown,b:unknown):unknown{return f<Wrong>(a);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
    for depth in [16, 129] {
        let source = format!(
            "function f(x={}1){{return x;}}export function run(a,b){{return f();}}",
            "+ ".repeat(depth)
        );
        assert_eq!(crate::compile_value(&source, false).is_ok(), depth == 16);
    }
}

#[test]
fn synchronous_arrow_defaults_share_the_checked_primitive_selection() {
    for source in [
        "export function run(a,b){const f=(x=3)=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>x;return f(a);}",
        "export function run(a,b){const f=(x=3)=>x;return f(undefined);}",
        "export function run(a,b){const f=(x=true)=>x;return f();}",
        "export function run(a,b){const f=(x='abc')=>x.length;return f();}",
        "export function run(a,b){const f=(x=3)=>y=>x+y;return f()(a);}",
        "export function run(a,b){const f=(x=3)=>x;return f.length;}",
        "export function run(a,b){const f=(x,y=3,z)=>y;return f.length;}",
        "export function run(a,b){const f=(x=3)=>{x++;return x;};return f();}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn arrow_defaults_keep_effect_scope_async_and_callback_arity_guards() {
    for source in [
        "export function run(a,b){const f=(x=a)=>x;return f();}",
        "export function run(a,b){const f=(x=Math.random())=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>Math.noSuch(x);return a;}",
        "export function run(a,b){const f=({x}=null)=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>x;const invoke=g=>g();return invoke(f);}",
        "export function run(a,b){const f=(x=3)=>x;const g=x=>x;const h=a?f:g;return h.length;}",
        "export function run(a,b){const f=(x='abc')=>x;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    for source in [
        "async function f(x=3){return x;}export async function run(a,b){return await f();}",
        "export async function run(a,b){const f=async(x=3)=>x;return await f();}",
    ] {
        assert!(crate::compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn synchronous_defaults_compose_with_async_callers() {
    for source in [
        // Retain the exact previously rejected named-helper and local-arrow inputs.
        "function f(x=3){return x;}export async function run(a,b){return f();}",
        "export async function run(a,b){const f=(x=3)=>x;return f();}",
        "export async function run(a,b){const f=(x,y=x||5)=>y;return f(a);}",
        "function f(x=3){return x;}export async function run(a,b){await b;return f(undefined);}",
        "function f(x=3){return x;}export async function run(a,b){const saved=f(a);await b;return saved;}",
        "function f(x,y=x+2){return y;}export async function run(a,b){return f(await a);}",
        "function f(x,y=x??3){return y;}export async function run(a,b){await b;return f(null);}",
        "function f(x,y=x){return x===y?1:0;}export async function run(a,b){const source={v:a};await b;return f(source);}",
        "function f(row,label=row.label??'missing'){return label.length;}export async function run(a,b){await b;return f({label:'ready'});}",
        "function f(prefix,label=prefix+':ok'){return label.length;}export async function run(a,b){return f('go')+await b;}",
        "export async function run(a,b){let f=(x=3)=>x;await b;return f();}",
        "export async function run(a,b){const o={f:(x=3)=>x};await b;return o.f();}",
        "export async function run(a,b){const make=(x=3)=>()=>x;const f=make();await b;return f();}",
        "export async function run(a,b){const f=(x,y=x+2)=>y;return f(await a);}",
        "export async function run(a,b){const f=(x=3)=>x;const invoke=g=>g(undefined);await b;return invoke(f);}",
    ] {
        for typescript in [false, true] {
            let result = crate::compile_async(source, typescript);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn async_module_defaults_keep_async_entry_scope_and_effect_guards() {
    for source in [
        "export async function run(a=3,b){return a;}",
        "async function f(x=3){return x;}export async function run(a,b){return await f();}",
        "export async function run(a,b){const f=async(x=3)=>x;return await f();}",
        "function f(x=x){return x;}export async function run(a,b){return f();}",
        "function f(x=y,y=3){return x;}export async function run(a,b){return f();}",
        "function f(x=a){return x;}export async function run(a,b){return f();}",
        "function f(x=Math.random()){return x;}export async function run(a,b){return f();}",
        "export async function run(a,b){const f=(x=a)=>x;return f();}",
        "export async function run(a,b){const f=(x=(a=3))=>x;return f();}",
        "export async function run(a,b){const f=(x=y,y=3)=>x;return f();}",
    ] {
        for typescript in [false, true] {
            assert!(
                crate::compile_async(source, typescript).is_err(),
                "{source}"
            );
        }
    }
}

#[test]
fn synchronous_defaults_in_async_modules_erase_types_without_changing_artifacts() {
    for (js, ts) in [
        (
            "function f(x,y=x+2){return y;}export async function run(a,b){return f(await a);}",
            "function f<T>(x:never,y:unknown=(x as number)+2):Missing{return y as never;}export async function run(a:string,b:unknown):Wrong{return f<Missing>(await(a as never));}",
        ),
        (
            "export async function run(a,b){const f=(x=3)=>x;await b;return f();}",
            "export async function run(a:never,b:unknown):Missing{const f=<T>(x:unknown=3):Wrong=>x as never;await(b as any);return f<Wrong>();}",
        ),
    ] {
        assert_eq!(
            crate::compile_async(js, false).unwrap().wasm,
            crate::compile_async(ts, true).unwrap().wasm,
            "{ts}",
        );
    }
}

#[test]
fn constant_parameter_defaults_check_intrinsic_resolution_and_void_operands() {
    for value in [
        "Infinity",
        "-Infinity",
        "+NaN",
        "void 0",
        "void null",
        "void 'abc'",
        "void void 0",
    ] {
        for body in [
            format!("function f(x={value}){{return x;}}export function run(a,b){{return f();}}"),
            format!("export function run(a,b){{const f=(x={value})=>x;return f();}}"),
            format!("export function run(a,b){{const o={{f:(x={value})=>x}};return o.f();}}"),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&body, ts);
                assert!(result.is_ok(), "{body}: {:?}", result.err());
            }
        }
    }
    for source in [
        "function f(x=void y,y=3){return x;}export function run(a,b){return f();}",
        "export function run(a,b){const Infinity=3;const f=(x=Infinity)=>x;return f();}",
        "function f(x=void missing){return x;}export function run(a,b){return f();}",
        "function f(x=void (a=3)){return x;}export function run(a,b){return f();}",
        "function f(x=void g()){return x;}export function run(a,b){return f();}",
        "function f(x=void {} ){return x;}export function run(a,b){return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [16, 129] {
                let source = format!(
                    "function f(x={}0){{return x;}}export function run(a,b){{return f();}}",
                    "void ".repeat(depth)
                );
                assert_eq!(crate::compile_value(&source, false).is_ok(), depth == 16);
            }
        })
        .unwrap()
        .join()
        .unwrap();
    let js = "function f(x=-Infinity){return x;}export function run(a,b){return f();}";
    let ts = "function f(x:never=(-Infinity as unknown)):boolean{return x;}export function run(a:string,b:boolean):never{return f();}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn earlier_parameter_defaults_use_initialized_values_in_all_sync_call_forms() {
    for source in [
        "function f(x,y=x){return y;}export function run(a,b){return f(a);}",
        "function f(x=3,y=x,z=y){return z;}export function run(a,b){return f();}",
        "function f(x=3,y=x){var x=9;return y;}export function run(a,b){return f();}",
        "function f(x,y=x){x=9;return y;}export function run(a,b){return f(a);}",
        "function f(undefined,x=undefined){return x;}export function run(a,b){return f(a);}",
        "function f(Infinity,x=Infinity){return x;}export function run(a,b){return f(a);}",
        "function f(NaN,x=NaN){return x;}export function run(a,b){return f(a);}",
        "function f(x='abc',y=x){return y.length;}export function run(a,b){return f();}",
        "function f(x,y=x){return x===y?1:0;}export function run(a,b){const o={a:a};return f(o);}",
        "export function run(a,b){const f=(x=3,y=x)=>y;return f();}",
        "export function run(a,b){const f=(undefined,x=undefined)=>x;return f();}",
        "export function run(a,b){const o={f:(x=3,y=x)=>y};return o.f();}",
        "export function run(a,b){const f=(x=3,y=x)=>()=>y;const g=f();return g();}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn earlier_parameter_defaults_keep_tdz_scope_and_effect_guards() {
    for source in [
        "function f(x=x){return x;}export function run(a,b){return f();}",
        "function f(x=y,y=3){return x;}export function run(a,b){return f();}",
        "function f(x=body){var body=3;return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=a)=>x;return f();}",
        "export function run(a,b){const f=(x=y,y=3)=>x;return f();}",
        "export function run(a,b){const f=(x=x)=>x;return f();}",
        "function f(x,y=(x=3)){return y;}export function run(a,b){return f(a);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    let js = "function f(x,y=x){return y;}export function run(a,b){return f(a);}";
    let ts = "function f<T>(x:boolean,y:string=(x as never)):Missing{return y as number;}export function run(a:string,b:boolean):never{return f<Wrong>(a);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [128, 129] {
                let source = format!(
                    "function f(x,y={}x{}){{return y;}}export function run(a,b){{return f(a);}}",
                    "(".repeat(depth),
                    ")".repeat(depth)
                );
                assert_eq!(
                    crate::compile_value(&source, false).is_ok(),
                    depth == 128,
                    "depth {depth}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn arithmetic_parameter_defaults_preserve_number_semantics_and_source_limits() {
    for value in [
        "1+2",
        "2*(3+4)",
        "7-9",
        "1/0",
        "0/0",
        "-0/3",
        "-4%2",
        "NaN+1",
        "Infinity-Infinity",
        "null*3",
        "true+2",
        "-null",
        "+'0x20'",
        "'2'**2",
        "void (1+2)",
    ] {
        for source in [
            format!("function f(x={value}){{return x;}}export function run(a,b){{return f();}}"),
            format!("export function run(a,b){{const o={{f:(x={value})=>x}};return o.f();}}"),
        ] {
            for ts in [false, true] {
                let r = crate::compile_value(&source, ts);
                assert!(r.is_ok(), "{source}: {:?}", r.err());
            }
        }
    }
    for value in [
        "'1'+2",
        "{}*3",
        "a+1",
        "missing+1",
        "g()+1",
        "void (missing+1)",
        "1**3",
    ] {
        let source =
            format!("function f(x={value}){{return x;}}export function run(a,b){{return f();}}");
        assert!(crate::compile_value(&source, false).is_err(), "{source}");
    }
    assert!(
        crate::compile_value(
            "function f(Infinity,x=Infinity+1){return x;}export function run(a,b){return f(a);}",
            false
        )
        .is_ok()
    );
    let js = "function f(x=2*(3+4)){return x;}export function run(a,b){return f();}";
    let ts = "function f(x:string=(2*((3 as boolean)+4) as never)):Wrong{return x as number;}export function run(a:string,b:boolean):never{return f();}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [128, 129] {
                let expression = (0..depth).fold("1".to_string(), |s, _| format!("1+{s}"));
                let source = format!(
                    "function f(x={expression}){{return x;}}export function run(a,b){{return f();}}"
                );
                assert_eq!(
                    crate::compile_value(&source, false).is_ok(),
                    depth == 128,
                    "{depth}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn unary_earlier_defaults_preserve_scope_conversion_and_depth_guards() {
    for source in [
        "function f(x,y=void x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=+x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=-x){return y;}export function run(a,b){return f(null);}",
        "function f(x,y=!x){return y;}export function run(a,b){return f({});}",
        "function f(x,y=+!!x){return y;}export function run(a,b){return f('abc');}",
        "export function run(a,b){const o={f:(x,y=-x)=>y};return o.f(a);}",
    ] {
        for ts in [false, true] {
            let r = crate::compile_value(source, ts);
            assert!(r.is_ok(), "{source}: {:?}", r.err());
        }
    }
    for source in [
        "function f(x=void x){return x;}export function run(a,b){return f();}",
        "function f(x=void y,y=3){return x;}export function run(a,b){return f();}",
        "function f(x,y=+x){return y;}export function run(a,b){return f({valueOf:()=>a});}",
        "function f(x,y=-x){return y;}export function run(a,b){return f('3');}",
        "export function run(a,b){const f=(x=void a)=>x;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    let js = "function f(x,y=-x){return y;}export function run(a,b){return f(a);}";
    let ts = "function f(x:string,y:boolean=(-(x as never) as unknown)):Wrong{return y as number;}export function run(a:string,b:boolean):never{return f(a);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [128, 129] {
                let source = format!(
                    "function f(x,y={}x){{return y;}}export function run(a,b){{return f(a);}}",
                    "void ".repeat(depth)
                );
                assert_eq!(
                    crate::compile_value(&source, false).is_ok(),
                    depth == 128,
                    "{depth}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn pure_expression_defaults_select_string_boolean_and_numeric_values() {
    for expression in [
        "'a'+'b'+'c'",
        "'\\u{1F600}'+'x'",
        "!(0/0)",
        "!('nonempty')",
        "false||7",
        "true&&'ready'",
        "null??'missing'",
        "undefined??false",
        "0??9",
        "NaN||4",
        "0?'wrong':'right'",
        "1?2+3:4*5",
        "(false||true)?'on'+'line':'off'",
        "void (1?2:3)",
    ] {
        for source in [
            format!(
                "function f(x={expression}){{return x===undefined?0:1;}}export function run(a,b){{return f();}}"
            ),
            format!(
                "export function run(a,b){{const f=(x={expression})=>x===undefined?0:1;return f();}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x={expression})=>x===undefined?0:1}};return o.f();}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn pure_expression_defaults_reject_unproven_coercion_and_hidden_effects() {
    for value in [
        "'a'+1",
        "3*{valueOf(){return 1;}}",
        "true?g():3",
        "false&&missing",
        "void (1||g())",
        "false?2:(a=3)",
        "`text${false && missing}`",
        "`text${{toString(){return 'boom'}}}`",
        "x??'fallback'",
        "1?2:Math.random()",
    ] {
        let source =
            format!("function f(x={value}){{return x;}}export function run(a,b){{return f();}}");
        assert!(crate::compile_value(&source, false).is_err(), "{source}");
    }
    let js = "function f(x='on'+'line'){return x.length;}export function run(a,b){return f();}";
    let ts = "function f(x=(('on' as never)+('line' as unknown))){return x.length;}export function run(a:number,b:number):number{return f();}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn earlier_parameter_logical_defaults_admit_pure_fallbacks() {
    for expression in [
        "x||'missing'",
        "x&&'enabled'",
        "x??42",
        "x||('a'+'b')",
        "x??(true?3:5)",
        "x&&(false||7)",
    ] {
        for source in [
            format!(
                "function f(x,y={expression}){{return y===undefined?0:1;}}export function run(a,b){{return f(a);}}"
            ),
            format!(
                "export function run(a,b){{const f=(x,y={expression})=>y===undefined?0:1;return f(a);}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x,y={expression})=>y===undefined?0:1}};return o.f(a);}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn earlier_parameter_logical_defaults_keep_scope_purity_and_arity_guards() {
    for source in [
        "function f(x=x||'missing'){return x;}export function run(a,b){return f();}",
        "function f(x,y=y||'missing'){return y;}export function run(a,b){return f();}",
        "function f(x,y=z||'missing',z=3){return y;}export function run(a,b){return f();}",
        "function f(x,y=x||g()){return y;}export function run(a,b){return f();}",
        "function f(x,y=x||missing){return y;}export function run(a,b){return f();}",
        "function f(x,y=x+1){return y;}export function run(a,b){return f({});}",
        "export function run(a,b){const f=(x,y=a||'wrong')=>y;return f();}",
        "export function run(a,b){const f=(x,y=(!x)||'wrong')=>y;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn negated_earlier_parameter_conditions_admit_pure_lazy_branches() {
    for expression in [
        "!x?'disabled':'enabled'",
        "!!x?'enabled':'disabled'",
        "!!!x?0:1",
        "!row.enabled?'disabled':'enabled'",
        "!!row.active?'active':'idle'",
    ] {
        for source in [
            format!(
                "function f(x,row,y={expression}){{return y===undefined?0:1;}}export function run(a,b){{return f(a,{{enabled:a,active:a}});}}"
            ),
            format!(
                "export function run(a,b){{const f=(x,row,y={expression})=>y===undefined?0:1;return f(a,{{enabled:a,active:a}});}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x,row,y={expression})=>y===undefined?0:1}};return o.f(a,{{enabled:a,active:a}});}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn negated_earlier_conditions_reject_hidden_reads_and_effects() {
    for source in [
        "function f(x,y=!y?1:0){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!later?1:0,later=3){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!missing?1:0){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!(x+1)?1:0){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!x?g():0){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!x?1:g()){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!(x++)?1:0){return y;}export function run(a,b){return f(a);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn earlier_parameter_conditional_defaults_admit_pure_branch_values() {
    for expression in [
        "x?'enabled':'disabled'",
        "x?1:0",
        "x?(1+2):(-3)",
        "x?(true&&'on'):(null??'off')",
        "x?null:undefined",
        "x?'\\u{1F600}':'plain'",
    ] {
        for source in [
            format!(
                "function f(x,y={expression}){{return y===undefined?0:1;}}export function run(a,b){{return f(a);}}"
            ),
            format!(
                "export function run(a,b){{const f=(x,y={expression})=>y===undefined?0:1;return f(a);}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x,y={expression})=>y===undefined?0:1}};return o.f(a);}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn logical_conditional_default_tests_admit_pure_prior_parameter_chains() {
    for expression in [
        "x&&backup?'both':'none'",
        "x||backup?'any':'none'",
        "x??backup?'value':'empty'",
        "(x??backup)||alt?'yes':'no'",
        "x&&(backup&&alt)?'all':'none'",
        "x||(backup||alt)?'any':'none'",
        "x??(backup??alt)?'value':'empty'",
    ] {
        for source in [
            format!(
                "function f(x,backup,alt,y={expression}){{return y==='none'?0:1;}}export function run(a,b){{return f(a,b,3);}}"
            ),
            format!(
                "export function run(a,b){{const f=(x,backup,alt,y={expression})=>y==='none'?0:1;return f(a,b,3);}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x,backup,alt,y={expression})=>y==='none'?0:1}};return o.f(a,b,3);}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn logical_conditional_default_tests_reject_unproven_terms_and_effects() {
    for source in [
        "function f(x,y,z=x&&g()?'on':'off'){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x||later?'on':'off',later=1){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x&&(y||x)?'on':'off'){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=(x+1)&&y?'on':'off'){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x??(y??z)?'on':'off'){return z;}export function run(a,b){return f(a,b);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn earlier_parameter_conditional_defaults_reject_scope_reads_and_branch_effects() {
    for source in [
        "function f(x=x?'a':'b'){return x;}export function run(a,b){return f();}",
        "function f(x,y=y?'a':'b'){return y;}export function run(a,b){return f();}",
        "function f(x,y=z?'a':'b',z=3){return y;}export function run(a,b){return f();}",
        "function f(x,y=x?g():3){return y;}export function run(a,b){return f();}",
        "function f(x,y=x?3:a){return y;}export function run(a,b){return f();}",
        "function f(x,y=(x+1)?3:4){return y;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x,y=a?'a':'b')=>y;return f(a);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn earlier_parameter_branches_admit_other_initialized_parameters() {
    for body in [
        "function f(x,backup,y=x??backup){return y===undefined?0:1;}export function run(a,b){return f(a,b);}",
        "function f(x,backup,y=x||backup){return y===undefined?0:1;}export function run(a,b){return f(a,b);}",
        "function f(x,backup,y=x&&backup){return y===undefined?0:1;}export function run(a,b){return f(a,b);}",
        "function f(flag,yes,no,y=flag?yes:no){return y===undefined?0:1;}export function run(a,b){return f(a,b,3);}",
        "function f(flag,yes,no,y=flag?yes:'off'){return y===undefined?0:1;}export function run(a,b){return f(a,b,3);}",
        "function f(flag,yes,no,y=flag?'on':no){return y===undefined?0:1;}export function run(a,b){return f(a,b,3);}",
        "export function run(a,b){const f=(x,backup,y=x??backup)=>y===undefined?0:1;return f(null,b);}",
        "export function run(a,b){const f=(flag,yes,no,y=flag?yes:no)=>y===undefined?0:1;return f(a,a,b);}",
        "export function run(a,b){const o={f:(flag,x,y,z=flag?x:y)=>z===undefined?0:1};return o.f(a,b,3);}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js =
        "function f(x,y,z=x??y){return z===undefined?0:1;}export function run(a,b){return f(a,b);}";
    let ts = "function f(x:unknown,y:unknown,z:unknown=(x as never)??(y as never)){return z===undefined?0:1;}export function run(a:unknown,b:unknown):unknown{return f(a,b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn earlier_parameter_branches_reject_self_later_scope_and_effectful_arms() {
    for body in [
        "function f(x,y=x??y){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x||z,z=3){return y;}export function run(a,b){return f(a);}",
        "function f(flag,x,y=flag?x:later,later=3){return y;}export function run(a,b){return f(a,b);}",
        "function f(flag,x,y=flag?y:x){return y;}export function run(a,b){return f(a,b);}",
        "function f(flag,x,y=flag?x:g()){return y;}export function run(a,b){return f(a,b);}",
        "function f(flag,x,y=flag?(x++):x){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,y=x||missing){return y;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x,y=a??x)=>y;return f(a,b);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn earlier_parameter_logical_chains_admit_policy_precedence() {
    for expression in [
        "x??backup??'missing'",
        "x||backup||'missing'",
        "x&&backup&&'enabled'",
        "(x??backup)||'none'",
        "(x||backup)??'missing'",
        "x??backup??alt",
        "x&&backup||alt",
    ] {
        for source in [
            format!(
                "function f(x,backup,alt,result={expression}){{return result===undefined?0:1;}}export function run(a,b){{return f(a,b,3);}}"
            ),
            format!(
                "export function run(a,b){{const f=(x,backup,alt,result={expression})=>result===undefined?0:1;return f(a,b,3);}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x,backup,alt,result={expression})=>result===undefined?0:1}};return o.f(a,b,3);}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn right_nested_logical_defaults_admit_pure_homogeneous_chains() {
    for expression in [
        "x??(backup??alt)",
        "x||(backup||alt)",
        "x&&(backup&&alt)",
        "(x??backup)??(alt??'missing')",
        "x??(backup??(alt??'missing'))",
        "x||(backup||'missing')",
        "x&&(backup&&'enabled')",
    ] {
        for source in [
            format!(
                "function f(x,backup,alt,result={expression}){{return result===undefined?0:1;}}export function run(a,b){{return f(a,b,3);}}"
            ),
            format!(
                "export function run(a,b){{const f=(x,backup,alt,result={expression})=>result===undefined?0:1;return f(a,b,3);}}"
            ),
            format!(
                "export function run(a,b){{const o={{f:(x,backup,alt,result={expression})=>result===undefined?0:1}};return o.f(a,b,3);}}"
            ),
        ] {
            for ts in [false, true] {
                let result = crate::compile_value(&source, ts);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
    }
}

#[test]
fn right_nested_logical_defaults_guard_effects_scope_and_mixed_operators() {
    for source in [
        "function f(x,y,z=x??(y??g())){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x||(y||missing)){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x&&(y&&z)){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x??(y??later),later=1){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x??(y||x)){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x||(y&&x)){return z;}export function run(a,b){return f(a,b);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn earlier_parameter_logical_chains_reject_hidden_reads_and_effects() {
    for source in [
        "function f(x,y=x??x??g()){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x||x||missing){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x??x??y){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x??x??later,later=3){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x??g()??3){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x??(x+1)??3){return y;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x,y=x??x??a)=>y;return f(a);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    let js = "function f(x,y,z=x??y??'missing'){return z;}export function run(a,b){return f(a,b);}";
    let ts = "function f(x:unknown,y:unknown,z:unknown=(x as never)??(y as never)??'missing'){return z;}export function run(a:unknown,b:unknown):unknown{return f(a,b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn numeric_defaults_evaluate_prior_parameters_in_source_order() {
    for body in [
        "function f(x,y=x+1){return y;}export function run(a,b){return f(a);}",
        "function f(x,d,y=x+d){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,d,y=2*x-d){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,d,y=(x+1)*(d-2)){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,d,y=x%(d+1)){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,d,y=x+d){return y;}export function run(a,b){return f(null,b)===b?1:0;}",
        "function f(x,d,y=x+d){return y;}export function run(a,b){return f(true,b)===b+1?1:0;}",
        "export function run(a,b){const f=(x,d,y=x+d)=>y;return f(a,b);}",
        "export function run(a,b){const o={f:(x,d,y=x+d)=>y};return o.f(a,b);}",
        "export function run(a,b){const f=(x,d,y=x+d)=>y;return f(a,b,9);}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js = "function f(x,d,y=x+d){return y;}export function run(a,b){return f(a,b);}";
    let ts = "function f(x:unknown,d:unknown,y:unknown=(x as never)+(d as never)){return y as never;}export function run(a:number,b:number):number{return f(a,b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn numeric_defaults_retain_effect_scope_and_coercion_guards() {
    for body in [
        "function f(x,y=x+'units'){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x+g()){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x+missing){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=y+1){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x+later,later=3){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x++ + 1){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x**x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x+1){return y;}export function run(a,b){return f('2');}",
        "function f(x,y=x+1){return y;}export function run(a,b){return f({valueOf:()=>3});}",
        "export function run(a,b){const f=(x,y=a+x)=>y;return f(a);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn earlier_string_defaults_compose_literal_fragments_and_templates() {
    for body in [
        "function f(x,y,z=x+'-'+y){return z.length;}export function run(a,b){return f('up','date')+a;}",
        "function f(x,y,z=x+y+'!'){return z.length;}export function run(a,b){return f('go','now')+a;}",
        "function f(x,y,z=(x+' ') + (y+'!')){return z.length;}export function run(a,b){return f('go','now')+a;}",
        "function f(x,y,z=`${x}:${y}`){return z.length;}export function run(a,b){return f('go','now')+a;}",
        "function f(x,y,z=`[${x}][${y}]`){return z.length;}export function run(a,b){return f('go','now')+a;}",
        "function f(x,y,z=`${x}`){return z.length;}export function run(a,b){return f('go','now')+a;}",
        "function f(x,y,z=x+y){return z.length;}export function run(a,b){return f('ab','cd')+a;}",
        "export function run(a,b){const f=(x,y=x+'!')=>y.length;return f('ok')+a;}",
        "export function run(a,b){const o={f:(x,y,z=x+':'+y)=>z.length};return o.f('a','bc')+a;}",
        "function f(x,y,z=x+'-'+y){return z.length;}export function run(a,b){return f('up','date','abc')+a;}",
        "function f(x,y,z=`${x}:${y}`){return z.length;}export function run(a,b){return f('up','date',undefined)+a;}",
        "function f(x,y,z=x+'\\u{1F642}'+y){return z.length;}export function run(a,b){return f('a','b')+a;}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js = "function f(x,y,z=x+':'+y){return z.length;}export function run(a,b){return f('a','bc')+a;}";
    let ts = "function f(x:never,y:never,z:unknown=(x as string)+':'+(y as string)){return (z as string).length;}export function run(a:number,b:number):number{return f('a','bc')+a;}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn earlier_string_defaults_reject_unproven_coercion_effects_and_tdz() {
    for body in [
        "function f(x,y=x+'!'){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=`${x}`){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=x+'!'){return y;}export function run(a,b){return f({});}",
        "function f(x,y=x+1){return y;}export function run(a,b){return f('text');}",
        "function f(x,y=x+later,later='!'){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=`${later}`,later='!'){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=`${y}`){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=x+missing){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=`${missing}`){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=x+g()){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=`${x++}`){return y;}export function run(a,b){return f('x');}",
        "function f(x,y=`${x+'!'}`){return y;}export function run(a,b){return f(1);}",
        "export function run(a,b){const f=(x,y=`${a}`)=>y;return f('x');}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn numeric_comparison_defaults_admit_thresholds_and_policy_selection() {
    for body in [
        "function f(x,y,z=x<y){return z?1:0;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x<=y){return z?10:20;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x>y){return z?3:7;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x>=y){return z?5:8;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x>=y?'high':'low'){return z.length;}export function run(a,b){return f(a,b);}",
        "function f(x,z=x>1?'large':'small'){return z.length;}export function run(a,b){return f(a);}",
        "function f(x,z=0<x){return z?1:0;}export function run(a,b){return f(a);}",
        "function f(x,z=x<NaN){return z?1:0;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x,y,z=x<y)=>z?1:0;return f(a,b);}",
        "export function run(a,b){const o={f:(x,y,z=x>y?'yes':'no')=>z.length};return o.f(a,b);}",
        "export function run(a,b){const f=(x,y,z=x<y)=>z?1:0;return f(a,b,false);}",
        "function f(x,y,z=x>y?x:y){return z;}export function run(a,b){return f(a,b);}",
    ] {
        for ts in [false, true] {
            let compiled = crate::compile_value(body, ts);
            assert!(compiled.is_ok(), "{body}: {:?}", compiled.err());
        }
    }
    let js = "function f(x,y,z=x>=y?'high':'low'){return z.length;}export function run(a,b){return f(a,b);}";
    let ts = "function f(x:unknown,y:unknown,z:unknown=(x as never)>=(y as never)?'high':'low'){return (z as string).length;}export function run(a:number,b:number):number{return f(a,b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn numeric_comparison_defaults_reject_unproven_coercion_scope_and_effects() {
    for body in [
        "function f(x,z=x<2){return z;}export function run(a,b){return f('1');}",
        "function f(x,z=x<2){return z;}export function run(a,b){return f({});}",
        "function f(x,z=x<later,later=2){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=z<2){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x<missing){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x<g()){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x++<2){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x===g()){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x<2?'yes':g()){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x<2?'yes':later,later='no'){return z;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x,z=a<2)=>z;return f(a);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn strict_equality_defaults_route_actions_by_value_and_identity() {
    for body in [
        "function f(x,y,z=x===y){return z?11:13;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x!==y){return z?17:19;}export function run(a,b){return f(a,b);}",
        "function f(x,z=x===2){return z?3:5;}export function run(a,b){return f(a);}",
        "function f(x,z=x!==undefined){return z?7:9;}export function run(a,b){return f(a);}",
        "function f(x,y,z=x===y?'match':'other'){return z.length;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x!==y?x:y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x===y){return z?1:0;}export function run(a,b){return f(a,b,false);}",
        "function f(x,y,z=x===y){return z?1:0;}export function run(a,b){const o={v:a};return f(o,o);}",
        "function f(x,y,z=x!==y){return z?1:0;}export function run(a,b){return f('go','go');}",
        "function f(x,y,z=x===y){return z?1:0;}export function run(a,b){return f(null,undefined);}",
        "export function run(a,b){const f=(x,y,z=x===y)=>z?1:0;return f(a,b);}",
        "export function run(a,b){const o={f:(x,y,z=x!==y)=>z?1:0};return o.f(a,b);}",
        "function f(x,y,z=x!==y?'different':'same'){return z.length;}export function run(a,b){return f(a,b);}",
    ] {
        for ts in [false, true] {
            let compiled = crate::compile_value(body, ts);
            assert!(compiled.is_ok(), "{body}: {:?}", compiled.err());
        }
    }
    let js = "function f(x,y,z=x===y?'same':'different'){return z.length;}export function run(a,b){return f(a,b);}";
    let ts = "function f(x:unknown,y:unknown,z:unknown=x===y?'same':'different'){return (z as string).length;}export function run(a:number,b:number):number{return f(a,b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn strict_equality_defaults_reject_coercion_side_effects_and_tdz() {
    for body in [
        "function f(x,y,z=x==y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x!=y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,z=x===later,later=2){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=z===x){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x===missing){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x===g()){return z;}export function run(a,b){return f(a);}",
        "function f(x,y,z=x++===y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,z=x==={v:1}){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x===2||false){return z;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x,z=a===x)=>z;return f(a);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn earlier_object_fields_supply_direct_logical_conditional_and_equality_defaults() {
    for body in [
        "function f(row,expected,selected=row.label){return selected===expected?1:0;}export function run(a,b){return f({label:a},a);}",
        "function f(row,backup,selected=row.label??backup){return selected===backup?1:0;}export function run(a,b){return f({label:undefined},a);}",
        "function f(row,backup,selected=row.label||backup){return selected===backup?1:0;}export function run(a,b){return f({label:''},a);}",
        "function f(row,backup,selected=row.label&&backup){return selected===backup?1:0;}export function run(a,b){return f({label:true},a);}",
        "function f(row,backup,selected=row.label?row.label:backup){return selected===backup?1:0;}export function run(a,b){return f({label:false},a);}",
        "function f(row,backup,selected=row.label===backup?'same':'other'){return selected.length;}export function run(a,b){return f({label:a},a);}",
        "function f(row,backup,selected=row.label!==backup?row.label:backup){return selected===backup?1:0;}export function run(a,b){return f({label:a},a);}",
        "function f(row,backup,selected=row['label']??backup){return selected===backup?1:0;}export function run(a,b){return f({label:a},a);}",
        "export function run(a,b){const f=(row,backup,selected=row.label??backup)=>selected===backup?1:0;return f({label:a},b);}",
        "export function run(a,b){const o={f:(row,backup,selected=row.label?row.label:backup)=>selected===backup?1:0};return o.f({label:a},b);}",
        "function f(row,backup,selected=row.missing??backup){return selected===backup?1:0;}export function run(a,b){return f({label:a},b);}",
        "function f(row,backup,selected=row.label??backup){return selected===backup?1:0;}export function run(a,b){return f({label:a},b,9);}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js = "function f(row,backup,selected=row.label??backup){return selected===backup?1:0;}export function run(a,b){return f({label:a},b);}";
    let ts = "function f(row:unknown,backup:unknown,selected:unknown=(row as {label:unknown}).label??backup){return selected===backup?1:0;}export function run(a:number,b:number):number{return f({label:a},b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn earlier_object_field_defaults_reject_unknown_receivers_and_effects() {
    for body in [
        "function f(row,selected=row.label){return selected;}export function run(a,b){return f(a);}",
        "function f(row,selected=row.label){return selected;}export function run(a,b){return f(null);}",
        "function f(row,selected=row.label){return selected;}export function run(a,b){return f(a?[a]:null);}",
        "function f(row,selected=row.toString){return selected;}export function run(a,b){return f({});}",
        "function f(row,selected=row.__proto__){return selected;}export function run(a,b){return f({});}",
        "function f(row,selected=row[key]){return selected;}export function run(a,b){return f({label:a});}",
        "function f(row,selected=row[read()]){return selected;}export function run(a,b){return f({label:a});}",
        "function f(row,selected=row?.label){return selected;}export function run(a,b){return f({label:a});}",
        "function f(row,selected=selected.label){return selected;}export function run(a,b){return f({label:a});}",
        "function f(row,selected=later.label,later={label:2}){return selected;}export function run(a,b){return f({label:a});}",
        "function f(row,selected=other.label){return selected;}export function run(a,b){return f({label:a});}",
        "export function run(a,b){const f=(row,selected=a.label)=>selected;return f({label:a});}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn earlier_array_elements_and_length_supply_lazy_parameter_defaults() {
    for body in [
        "function f(xs,want,z=xs[0]){return z===want?1:0;}export function run(a,b){return f([a,b],a);}",
        "function f(xs,want,z=xs[1]){return z===want?1:0;}export function run(a,b){return f([a,b],b);}",
        "function f(xs,want,z=xs['0']){return z===want?1:0;}export function run(a,b){return f([a,b],a);}",
        "function f(xs,want,z=xs[9]??want){return z===want?1:0;}export function run(a,b){return f([a,b],a);}",
        "function f(xs,want,z=xs[0]||want){return z===want?1:0;}export function run(a,b){return f([0,b],a);}",
        "function f(xs,want,z=xs[0]&&want){return z===want?1:0;}export function run(a,b){return f([true,b],a);}",
        "function f(xs,want,z=xs[0]?xs[0]:want){return z===want?1:0;}export function run(a,b){return f([false,b],a);}",
        "function f(xs,want,z=xs[0]===want?1:0){return z;}export function run(a,b){return f([a,b],a);}",
        "function f(xs,n=xs.length){return n===undefined?0:1;}export function run(a,b){return f([a,b]);}",
        "function f(xs,n=xs.length){return n===undefined?0:1;}export function run(a,b){return f([a,,b]);}",
        "function f(xs,n=xs.length){return n===undefined?0:1;}export function run(a,b){return f([]);}",
        "function f(xs,n=xs.length){return n===undefined?0:1;}export function run(a,b){return f([a,b],7);}",
        "function f(xs,want,z=xs[0]){return z===want?1:0;}export function run(a,b){const o={v:a};return f([o],o);}",
        "export function run(a,b){const f=(xs,want,z=xs[0])=>z===want?1:0;return f([a],a);}",
        "export function run(a,b){const o={f:(xs,n=xs.length)=>n===undefined?0:1};return o.f([a,b]);}",
        "function f(xs,want,z=xs[0]??want){return z===want?1:0;}export function run(a,b){return f([undefined],a);}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js = "function f(xs,want,z=xs[0]??want){return z===want?1:0;}export function run(a,b){return f([a],b);}";
    let ts = "function f(xs:unknown[],want:unknown,z:unknown=xs[0]??want){return z===want?1:0;}export function run(a:number,b:number):number{return f([a],b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn earlier_array_defaults_guard_prototypes_dynamic_keys_and_unknown_kinds() {
    for body in [
        "function f(xs,z=xs[0]){return z;}export function run(a,b){return f(a);}",
        "function f(xs,z=xs[0]){return z;}export function run(a,b){return f(null);}",
        "function f(xs,z=xs.map){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=xs.slice){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=xs.toString){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=xs.__proto__){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=xs[key]){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=xs[read()]){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=xs?.[0]){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=z[0]){return z;}export function run(a,b){return f([a]);}",
        "function f(xs,z=later[0],later=[a]){return z;}export function run(a,b){return f([a]);}",
        "export function run(a,b){const f=(xs,z=a[0])=>z;return f([a]);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn bitwise_parameter_defaults_handle_flags_shifts_and_primitive_coercion() {
    for body in [
        "function f(x,y,z=x&y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x|y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x^y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x<<y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x>>y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=x>>>y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,z=~x){return z;}export function run(a,b){return f(a);}",
        "function f(x,y,z=(x&255)>>>y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=~(x<<y)){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,y,z=(~x)&y){return z;}export function run(a,b){return f(a,b);}",
        "function f(x,z=~x){return z;}export function run(a,b){return f(null);}",
        "function f(x,z=~x){return z;}export function run(a,b){return f(false);}",
        "function f(x,y,z=x>>>y){return z;}export function run(a,b){return f(a,b,7);}",
        "export function run(a,b){const f=(x,y,z=x^y)=>z;return f(a,b);}",
        "export function run(a,b){const o={f:(x,y,z=x&y)=>z};return o.f(a,b);}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js = "function f(x,y,z=x>>> (y&31)){return z;}export function run(a,b){return f(a,b);}";
    let ts = "function f(x:unknown,y:unknown,z:unknown=(x as number)>>>((y as number)&31)){return z as number;}export function run(a:number,b:number):number{return f(a,b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn bitwise_parameter_defaults_reject_effects_unknown_kinds_and_scope() {
    for body in [
        "function f(x,z=~x){return z;}export function run(a,b){return f('5');}",
        "function f(x,z=x&1){return z;}export function run(a,b){return f({});}",
        "function f(x,z=x>>>1){return z;}export function run(a,b){return f([a]);}",
        "function f(x,z=z&1){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=later&1,later=2){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x&missing){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x&g()){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x++|1){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=~g()){return z;}export function run(a,b){return f(a);}",
        "function f(x,z=x**3){return z;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x,z=~a)=>z;return f(a);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}

#[test]
fn nested_action_record_default_paths_admit_proven_objects_and_arrays() {
    for body in [
        "function f(row,expected,z=row.meta.label){return z===expected?1:0;}export function run(a,b){return f({meta:{label:a}},a);}",
        "function f(row,backup,z=row.meta.label??backup){return z===backup?1:0;}export function run(a,b){return f({meta:{label:undefined}},a);}",
        "function f(row,backup,z=row.meta.label??backup){return z===backup?1:0;}export function run(a,b){return f({meta:{label:a}},b);}",
        "function f(row,backup,z=row.meta.label||backup){return z===backup?1:0;}export function run(a,b){return f({meta:{label:0}},b);}",
        "function f(row,backup,z=row.meta.label?row.meta.label:backup){return z===backup?1:0;}export function run(a,b){return f({meta:{label:false}},b);}",
        "function f(row,backup,z=row.meta.label===backup?'match':'other'){return z.length;}export function run(a,b){return f({meta:{label:a}},a);}",
        "function f(row,backup,z=row.meta.deep.label){return z===backup?1:0;}export function run(a,b){return f({meta:{deep:{label:a}}},a);}",
        "function f(row,backup,z=row.items[0].label){return z===backup?1:0;}export function run(a,b){return f({items:[{label:a}]},a);}",
        "function f(row,backup,z=row.meta.label){return z===backup?1:0;}export function run(a,b){return f({meta:{label:a}},b,b);}",
        "export function run(a,b){const f=(row,backup,z=row.meta.label??backup)=>z===backup?1:0;return f({meta:{label:a}},b);}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(body, ts);
            assert!(result.is_ok(), "{body}: {:?}", result.err());
        }
    }
    let js = "function f(row,backup,z=row.meta.label??backup){return z===backup?1:0;}export function run(a,b){return f({meta:{label:a}},b);}";
    let ts = "function f(row:unknown,backup:unknown,z:unknown=(row as {meta:{label:unknown}}).meta.label??backup){return z===backup?1:0;}export function run(a:number,b:number):number{return f({meta:{label:a}},b);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
}

#[test]
fn nested_action_record_defaults_reject_unknown_intermediates_and_effects() {
    for body in [
        "function f(row,z=row.meta.label){return z;}export function run(a,b){return f(a);}",
        "function f(row,z=row.meta.label){return z;}export function run(a,b){return f({meta:null});}",
        "function f(row,z=row.meta.label){return z;}export function run(a,b){return f({meta:a});}",
        "function f(row,z=row.meta[key]){return z;}export function run(a,b){return f({meta:{label:a}});}",
        "function f(row,z=row.meta[read()]){return z;}export function run(a,b){return f({meta:{label:a}});}",
        "function f(row,z=row.meta.toString){return z;}export function run(a,b){return f({meta:{label:a}});}",
        "function f(row,z=row.meta.__proto__){return z;}export function run(a,b){return f({meta:{label:a}});}",
        "function f(row,z=row.meta?.label){return z;}export function run(a,b){return f({meta:{label:a}});}",
        "function f(row,z=row.meta.label){return z;}export function run(a,b){const x={meta:{label:a}};return f(b?x:{meta:null});}",
        "function f(row,z=later.meta.label,later={meta:{label:1}}){return z;}export function run(a,b){return f({meta:{label:a}});}",
        "function f(row,z=row.meta.label){return z;}export function run(a,b){const f2=(x,y=a.meta.label)=>y;return f2(row);}",
    ] {
        assert!(crate::compile_value(body, false).is_err(), "{body}");
    }
}
