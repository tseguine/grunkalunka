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
    assert!(
        crate::compile_async(
            "function f(x=3){return x;}export async function run(a,b){return f();}",
            false
        )
        .is_err()
    );
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
        "export async function run(a,b){const f=(x=3)=>x;return f();}",
        "export async function run(a,b){const f=async(x=3)=>x;return await f();}",
    ] {
        assert!(crate::compile_async(source, false).is_err(), "{source}");
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
        "true+2",
        "null*3",
        "a+1",
        "missing+1",
        "g()+1",
        "void (missing+1)",
        "1**2",
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
        .is_err()
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
                "function f(x={expression}){{return x;}}export function run(a,b){{return f();}}"
            ),
            format!("export function run(a,b){{const f=(x={expression})=>x;return f();}}"),
            format!("export function run(a,b){{const o={{f:(x={expression})=>x}};return o.f();}}"),
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
        "true+2",
        "null*3",
        "true?g():3",
        "false&&missing",
        "void (1||g())",
        "false?2:(a=3)",
        "x??'fallback'",
        "1?2:Math.random()",
    ] {
        let source = format!(
            "function f(x={value}){{return x;}}export function run(a,b){{return f();}}"
        );
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
            format!("function f(x,y={expression}){{return y;}}export function run(a,b){{return f(a);}}"),
            format!("export function run(a,b){{const f=(x,y={expression})=>y;return f(a);}}"),
            format!("export function run(a,b){{const o={{f:(x,y={expression})=>y}};return o.f(a);}}"),
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
        "function f(x,y=x+1){return y;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x,y=a||'wrong')=>y;return f();}",
        "export function run(a,b){const f=(x,y=(!x)||'wrong')=>y;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    assert!(
        crate::compile_async(
            "export async function run(a,b){const f=(x,y=x||5)=>y;return f(a);}",
            false
        )
        .is_err()
    );
}
