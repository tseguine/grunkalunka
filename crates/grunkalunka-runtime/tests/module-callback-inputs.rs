#[path = "managed/common.rs"]
mod common;

#[test]
fn module_callback_accepts_and_returns_the_original_object() {
    let source = "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);const o={x:a};return apply(id,o)===o?1:0;}";
    common::compare_sources(source, source);
}

fn compare(helpers: &str, body: &str) {
    let source = format!("{helpers}export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn callbacks_transport_arrays_strings_and_mixed_managed_inputs() {
    for body in [
        "const apply=(fn,x)=>fn(x);const o=[a,b];return apply(id,o)===o?1:0;",
        "const apply=(fn,x)=>fn(x);return apply(id,'😀')==='😀'?1:0;",
        "const apply=(fn,x)=>fn(x);const o={x:a};return apply(id,a)===a&&apply(id,o)===o?1:0;",
        "const apply=(fn,x)=>fn(x);const o={};return apply(id,true)===true&&apply(id,o)===o?1:0;",
        "const apply=(fn,x)=>fn(x);const o=[];return apply(id,'s')==='s'&&apply(id,o)===o?1:0;",
    ] {
        compare("function id(x){return x;}", body);
    }
    compare(
        "function kind(x){return typeof x;}",
        "const apply=(fn,x)=>fn(x);const o={};return apply(kind,o)==='object'?1:0;",
    );
    compare(
        "function size(x){return x.length;}",
        "const apply=(fn,x)=>fn(x);return apply(size,[a,b]);",
    );
}

#[test]
fn callback_input_facts_follow_forwarders_aliases_and_returned_factories() {
    for body in [
        "const apply=(fn,x)=>fn(x);const forward=(fn,x)=>apply(fn,x);const o={};return forward(id,o)===o?1:0;",
        "const apply=(fn,x)=>{let local=fn;const value=x;return local(value);};const o={};return apply(id,o)===o?1:0;",
        "const make=fn=>x=>fn(x);const apply=make(id);const o={};return apply(o)===o?1:0;",
        "const make=fn=>{const apply=x=>fn(x);return apply;};const apply=make(id);const o={};return apply(o)===o?1:0;",
        "const apply=(fn,x)=>{const again=y=>fn(y);return again(x);};const o=[];return apply(id,o)===o?1:0;",
        "const identity=fn=>fn;const apply=identity((fn,x)=>fn(x));const o={};return apply(id,o)===o?1:0;",
    ] {
        compare("function id(x){return x;}", body);
    }
}

#[test]
fn callback_alternatives_and_argument_snapshots_keep_original_effects() {
    for body in [
        "const apply=(fn,x)=>fn(x);const o={};return apply(a?id:other,o)===o?1:0;",
        "const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x);const o={};return apply(id,o)===o?1:0;",
        "const apply=(fn,x)=>fn(x);const f=a?id:x=>x;const o={};return apply(f,o)===o?1:0;",
        "let f=id;const apply=(fn,x)=>fn(x);const o={};return apply(f,(f=other,o))===o&&f===other?1:0;",
        "let n=0;const apply=(fn,x)=>fn(x);const o={};const value=apply.call((n++,b),id,(n++,o),(n++,b));return value===o&&n===3?1:0;",
    ] {
        compare(
            "function id(x){return x;}function other(x){return x;}",
            body,
        );
    }
    compare(
        "function id(x){return x;}",
        "let n=0;const apply=(fn,x)=>fn(x,(n++,{}));const result=apply(id,a);return n===1&&result===a?1:0;",
    );
}

#[test]
fn callback_input_facts_preserve_cycles_and_ignore_type_annotations() {
    compare(
        "function id(x){return x;}",
        "const make=fn=>x=>fn(x);const apply=make(id);const o={};o.self=o;const result=apply(o);return result===o&&result.self===o?1:0;",
    );
    common::compare_sources(
        "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);const o={x:a};return apply(id,o)===o?1:0;}",
        "function id<T>(x:number):string{return x;}export function run(a:boolean,b:string):void{const apply=(fn:never,x:number):boolean=>fn(x);const o={x:a};return apply<Missing>((id as unknown)!,o)===o?1:0;}",
    );
}

#[test]
fn module_helpers_mutate_and_read_managed_callback_arguments() {
    compare(
        "function set(o,x){o.x=x;return o;}",
        "const apply=(fn,o,x)=>fn(o,x);const o={x:b};const result=apply(set,o,a);return result===o&&o.x===a?1:0;",
    );
    compare(
        "function read(o){return o.x;}",
        "const apply=(fn,o)=>fn(o);const o={x:a};return apply(read,o)===a?1:0;",
    );
    compare(
        "function replace(o,x){o[0]=x;return o;}",
        "const apply=(fn,o,x)=>fn(o,x);const o=[b];return apply(replace,o,a)===o&&o[0]===a?1:0;",
    );
}

#[test]
fn type_only_string_literals_do_not_change_callback_admission() {
    let source = "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(id,true)===true?1:0;}";
    for prefix in [
        "type Unused=\"metadata\";",
        "interface Unused {\"metadata\":number}",
        "import type {Unused} from 'metadata';",
        "export type * from 'metadata';",
    ] {
        let typed = format!("{prefix}{source}");
        let js = grunkalunka_compiler::compile(source, false).unwrap();
        let ts = grunkalunka_compiler::compile(&typed, true).unwrap();
        assert_eq!(js.wasm, ts.wasm, "{prefix}");
        common::compare_sources(source, &typed);
    }
    for body in [
        "const apply=(fn,x)=>fn(x);return apply(id,null)===null?1:0;",
        "const apply=(fn,x)=>fn(x);return apply(id,undefined)===undefined?1:0;",
        "const apply=(fn,x)=>fn(x);return apply(id,a<b)===(a<b)?1:0;",
        "const apply=(fn,x)=>fn(x);return apply(id,void a)===undefined?1:0;",
        "const apply=(fn,x)=>fn(x);let value;return apply(id,value)===undefined?1:0;",
    ] {
        compare("function id(x){return x;}", body);
    }
}

#[test]
fn callback_input_evidence_does_not_admit_unknown_or_incompatible_calls() {
    for source in [
        "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(missing,{});}",
        "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(a,{});}",
        "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);const f=a?id:undefined;return apply(f,{});}",
        "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);const f=a?id:x=>1;return apply(f,{});}",
        "function id(x){return x;}function other(x){return 1;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(a?id:other,{})===null?1:0;}",
        "function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(id);}",
        "function id(x){return x+1;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(id,{});}",
        "function id(x){return x;}export async function run(a,b){const apply=(fn,x)=>fn(x);const o={};const g=async x=>{await b;return x;};const p=g(a);await p;return apply(id,o)===o?1:0;}",
    ] {
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| {
                if source.contains("export async") {
                    grunkalunka_compiler::compile_async(source, ts)
                } else {
                    grunkalunka_compiler::compile(source, ts)
                }
            })
            .unwrap_or_else(|_| panic!("compiler panicked for {source}"));
            let error = result
                .err()
                .unwrap_or_else(|| panic!("unexpected admission {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
            assert!(
                error.contains("unsupported")
                    || error.contains("signature")
                    || error.contains("reference-aware"),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn callback_input_relations_fit_a_two_megabyte_native_stack() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "function id(x){{return x;}}export function run(a,b){{const apply=(fn,x)=>fn(x);const o={{}};return apply({}id{},o)===o?1:0;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "depth={depth}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            assert!(error.contains("unsupported"), "{error}");
        }
    }
    for (aliases, admitted) in [(126, true), (127, false)] {
        let aliases_source = (1..=aliases)
            .map(|i| format!("const f{i}=f{};", i - 1))
            .collect::<String>();
        let source = format!(
            "function id(x){{return x;}}export function run(a,b){{const f0=(fn,x)=>fn(x);{aliases_source}const o={{}};return f{aliases}(id,o)===o?1:0;}}"
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "aliases={aliases}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            // Beyond the source bound, a signature query may reject before
            // declaration lowering reaches its explicit local-count check.
            assert!(error.contains("unsupported"), "{error}");
        }
    }
}
