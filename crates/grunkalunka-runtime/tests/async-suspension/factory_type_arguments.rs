use super::*;

fn compare_typed(plain: &str, typed: &str) {
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true)
            .unwrap_or_else(|error| panic!("{typed}: {error}"))
            .wasm,
        "{typed}"
    );
    compare_async_sources(
        &[plain.to_string()],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn factory_type_arguments_erase_on_selected_runtime_results() {
    compare_typed(
        &source_for(
            "const one=x=>()=>++x,two=x=>()=>x+b;const make=a?one:two;const f=make(a);return f()+f();",
        ),
        &source_for(
            "const one=x=>()=>++x,two=x=>()=>x+b;const make=a?one:two;const f=make<Missing>(a);return f()+f();",
        ),
    );
    for body in [
        "const make=a?x=>()=>x+a:x=>()=>x-b;const f=make(b);return f();",
        "const make=a?x=>async()=>{await a;return x;}:x=>async()=>{await b;return -x;};const f=make(b);return await f();",
        "const make=a?p=>()=>p:p=>()=>p;return await make(Promise.resolve(b))();",
        "const make=a?x=>()=>false:x=>()=>undefined;const f=make(await b);await null;return f();",
        "let make=x=>y=>()=>x+y;return make(a)(b)();",
    ] {
        let plain = source_for(body);
        let typed = plain.replace("make(", "make<typeof absent>(");
        compare_typed(&plain, &typed);
    }
    compare_typed(
        &source_for("let make=x=>y=>()=>x+y;return make(a)(b)();"),
        &source_for("let make=x=>y=>()=>x+y;return make<Missing>(a)<Other>(b)();"),
    );
}

#[test]
fn factory_type_arguments_keep_fresh_captures_callees_and_async_order() {
    for body in [
        "const one=x=>()=>++x,two=x=>()=>x+b;const make=a?one:two;const f=make(await a),g=make(b);await null;return f()*100+g()*10+f();",
        "let make=x=>()=>x+a;const f=make(await(make=x=>()=>x-b,a));await null;return f()+make(b)();",
        "let leaf=x=>x+a;let make=()=>leaf;const f=make();await null;leaf=x=>x-b;return f(a)+make()(a);",
        "const factory=a?(fn,x)=>()=>fn(x):(fn,x)=>()=>fn(x)+b;const f=factory(x=>x+a,await b);await null;return f()+f();",
        "let make=x=>()=>x+a;try{return make(await Promise.reject(a))();}catch(reason){return make(+reason)();}",
        "let n=0;const make=a?x=>()=>x+n:x=>()=>x-n;const f=make(a,await(n=n+1),n=n+10);return f()+n;",
        "let make=x=>(a++,()=>x);make(b);return a;",
    ] {
        let plain = source_for(body);
        let typed = plain
            .replace("make(", "make<never,Missing>(")
            .replace("factory(", "factory<Missing>(");
        compare_typed(&plain, &typed);
    }
}

#[test]
fn factory_type_arguments_do_not_supply_values_or_function_signatures() {
    for body in [
        "let make=x=>()=>x;return make<never>()();",
        "let make=x=>()=>x;return make<never>(...a)();",
        "let make=x=>()=>x;return make?.<never>(a)();",
        "let make=x=>()=>x;return make<number>('x')();",
        "let make=x=>()=>x;return make<number>({})();",
        "let make=x=>()=>x;return make<never>(missing)();",
        "const make=a?()=>()=>a:()=>x=>x;return make<()=>number>()();",
        "const make=a?()=>()=>a:()=>b;return make<()=>number>()();",
        "const make=a;return make<()=>number>(b)();",
    ] {
        assert!(compile_async(&source_for(body), true).is_err(), "{body}");
    }
}

#[test]
fn factory_type_arguments_keep_metadata_argument_and_source_limits() {
    let plain = source_for("let make=x=>()=>x;return make(a)();");
    for metadata in ["Missing", "never", "typeof absent", "{call:()=>void}"] {
        let typed = plain.replace("make(", &format!("make<{metadata}>("));
        compare_typed(&plain, &typed);
    }
    let types = vec!["never"; 1001].join(",");
    let typed = plain.replace("make(", &format!("make<{types}>("));
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [1000, 1001] {
        let args = vec!["a"; count].join(",");
        let plain = source_for(&format!("let make=x=>()=>x;return make({args})();"));
        let typed = plain.replace("make(", "make<never>(");
        if count == 1000 {
            assert_eq!(
                compile_async(&plain, false).unwrap().wasm,
                compile_async(&typed, true).unwrap().wasm
            );
        } else {
            assert!(compile_async(&typed, true).err().unwrap().contains("1000"));
        }
    }
    for (count, admitted) in [(8, true), (9, false)] {
        let mut expression = "a".to_string();
        for _ in 0..count {
            expression = format!("make({expression})()");
        }
        let plain = source_for(&format!("let make=x=>()=>x;return {expression};"));
        let typed = plain.replace("make(", "make<never>(");
        if admitted {
            assert_eq!(
                compile_async(&plain, false).unwrap().wasm,
                compile_async(&typed, true).unwrap().wasm
            );
        } else {
            assert!(
                compile_async(&typed, true)
                    .err()
                    .unwrap()
                    .contains("8 active")
            );
        }
    }
    let prefix = "export async function run(a,b){let make=x=>()=>x;return make<\"";
    let suffix = "\">(a)();}";
    let exact = format!(
        "{prefix}{}{suffix}",
        "x".repeat(16_384 - prefix.len() - suffix.len())
    );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&exact, true).unwrap().wasm
    );
    assert!(
        compile_async(&(exact + " "), true)
            .err()
            .unwrap()
            .contains("16 KiB")
    );
}
