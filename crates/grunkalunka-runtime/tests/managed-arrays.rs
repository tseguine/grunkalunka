#[path = "managed/common.rs"]
mod common;

#[test]
fn empty_array_length_is_a_number() {
    common::compare_sources(
        "export function run(a,b){const x=[];return x.length;}",
        "export function run(a:number,b:number):number{const x=[];return x.length;}",
    );
    let former = "export function run(a,b){const x=[];return a;}";
    common::compare_sources(former, former);
    for former in [
        "export function run(a,b){const o={x:[]};return a;}",
        "export function run(a,b){return (a?'x':[])===null?a:b;}",
    ] {
        common::compare_sources(former, former);
    }
}

#[test]
fn array_literals_keep_holes_trailing_commas_and_element_order() {
    for js in [
        "export function run(a,b){const x=[a,b,,];return x.length;}",
        "export function run(a,b){const x=[,,,];return x.length;}",
        "export function run(a,b){const x=[a,b,];return x.length;}",
        "export function run(a,b){let n=0;const x=[(n=n*10+1,a),(n=n*10+2,b),,];return n*10+x.length;}",
    ] {
        common::compare_sources(js, js);
    }
}

#[test]
fn array_elements_snapshot_values_and_static_index_updates_grow_length() {
    for js in [
        "export function run(a,b){const x=[a,b];return x[0]===a&&x[1]===b?a:b;}",
        "export function run(a,b){const x=[a,,undefined];return x[1]===undefined&&x[2]===undefined?a:b;}",
        "export function run(a,b){const x=[a];const old=x[0];x[0]=b;x[3]=a;return old===a&&x[0]===b&&x.length===4?a:b;}",
        "export function run(a,b){const x=[];x['01']=a;x[4294967295]=b;x[4294967294]=a;return x.length===4294967295&&x['01']===a&&x[4294967295]===b?a:b;}",
    ] {
        common::compare_sources(js, js);
    }
}

#[test]
fn array_references_preserve_identity_in_containers_rebinding_and_helpers() {
    for js in [
        "export function run(a,b){let x=[a];const alias=x;const o={ref:x};const outer=[x];x=[b];alias[0]=b;return alias!==x&&o.ref===alias&&outer[0]===alias&&alias[0]===b?a:b;}",
        "export function run(a,b){const x=a?[a]:[b];return x&&x!==[]?a:b;}",
        "function id(x){return x;} function make(v){return [v];} export function run(a,b){const x=make(a);const alias=id(x);alias[0]=b;return id(x)===x&&x[0]===b?a:b;}",
        "function size(x){return x.length;} export function run(a,b){const x=[a,b];return size(x);}",
    ] {
        common::compare_sources(js, js);
    }
}

#[test]
fn array_length_writes_truncate_and_keep_numeric_results() {
    for js in [
        "export function run(a,b){const x=[a,b,a];const old=x[2];x.length=1;x.length=3;return x[0]===a&&x[1]===undefined&&x[2]===undefined&&old===a?a:b;}",
        "export function run(a,b){const x=[a,b];x['01']=a;x.length=0;return x['01']===a&&x.length===0?a:b;}",
        "export function run(a,b){const x=[a,b];return x.length=1;}",
        "function resize(x){return x.length=2;} export function run(a,b){const x=[a];return resize(x);}",
    ] {
        common::compare_sources(js, js);
    }
}

#[test]
fn array_static_keys_and_lengths_erase_type_wrappers() {
    common::compare_sources(
        "export function run(a,b){const x=[a,b];x['length']=1;return x['length']+(x[0]===a?1:0);}",
        "export function run(a:string,b:boolean):void{const x:number=[a,b];x[('length' as string)]=(1 as number);return x[('length' satisfies string)]+(x[(0 as number)]===a?1:0);}",
    );
    common::compare_sources(
        "function size(x){return x['length'];} export function run(a,b){return size([a,b]);}",
        "function size(x:number):string{return x[('length' as string)];} export function run(a:boolean,b:object):void{return size([a,b]);}",
    );
}

#[test]
fn array_prototype_guards_match_native_intrinsic_names_without_restricting_objects() {
    let output = std::process::Command::new("node")
        .args([
            "-p",
            "Object.getOwnPropertyNames(Array.prototype).join('\\n')",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout).unwrap();
    for key in names.lines().filter(|s| *s != "length") {
        let js =
            format!("export function run(a,b){{const x=[];return x[{key:?}]===undefined?a:b;}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&js, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted inherited array key {key}"));
            assert!(error.contains("prototype"), "{key}: {error}");
        }
    }
    for key in ["map", "push", "with", "at"] {
        let js = format!("export function run(a,b){{const o={{{key}:a}};return o.{key}===a?a:b;}}");
        common::compare_sources(&js, &js);
    }
}

#[test]
fn unsupported_array_semantics_reject_before_ir_validation() {
    for body in [
        "const x=[a];return x[b]===a?a:b;",
        "const x=[];x[0.5]=a;return a;",
        "const x=[];x[-1]=a;return a;",
        "const x=[];x[4294967296]=a;return a;",
        "const x=[];x.length=0.5;return a;",
        "const x=[];x.length=-1;return a;",
        "const x=[];x.length=4294967296;return a;",
        "const x=[];x.length=a;return a;",
        "const x=[];x.length='2';return a;",
        "const x=[];x.length+=1;return a;",
        "const x=[];x.length++;return a;",
        "const x=[...[a]];return a;",
        "const x=[a];return x;",
        "const x=[a];return x[0].length;",
        "if(0){const x=[];x.map(a);}return a;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&js, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {js}"));
            assert!(!error.contains("internal IR"), "{js}: {error}");
            assert!(
                error.contains("unsupported") || error.contains("managed"),
                "{js}: {error}"
            );
        }
    }
    let error = grunkalunka_compiler::compile_async(
        "export async function run(a,b){const x=[a];return x.length;}",
        false,
    )
    .err()
    .unwrap();
    assert!(error.contains("reference-aware async frames"), "{error}");
}

#[test]
fn array_receiver_snapshots_and_mixed_helper_values_keep_call_order() {
    for js in [
        "export function run(a,b){let n=0;let x=[a];const old=x;x[0]=(x=[b],n++,a);return old!==x&&old[0]===a&&x[0]===b&&n===1?a:b;}",
        "function id(v){return v;} export function run(a,b){const x=[a];const o={};return id(x)===x&&id(o)===o&&id('s')==='s'&&id(a)===a?a:b;}",
        "function id(v){return v;} export function run(a,b){let n=0;const x=[a];return id.call((n++,[b]),x,(n++,[a]))===x&&n===2?a:b;}",
        "export function run(a,b){const outside=[a];const f=v=>{const x=[v,b];return x.length+a;};return f(b)+outside.length;}",
        "export function run(a,b){const x=[];x[0]=x;return x[0]===x?a:b;}",
        "export function run(a,b){const o={value:a};const x=[o,'text',null,false,undefined];return x[0]===o&&x[1]==='text'&&x[2]===null&&x[3]===false&&x[4]===undefined?a:b;}",
        r"export function run(a,b){const x=[];x['\ud800\0']=a;x['\ufffd\0']=b;return x['\ud800\0']===a&&x['\ufffd\0']===b&&x.length===0?a:b;}",
    ] {
        common::compare_sources(js, js);
    }
}

#[test]
fn array_source_and_key_queries_keep_depth_off_the_native_stack() {
    for (depth, admitted, array) in [
        (16, true, true),
        (128, true, true),
        (129, false, true),
        (16, true, false),
        (125, true, false),
        (126, false, false),
    ] {
        let source = if array {
            format!(
                "export function run(a,b){{const x={}a{};return x.length;}}",
                "[".repeat(depth),
                "]".repeat(depth)
            )
        } else {
            format!(
                "export function run(a,b){{const x=[a];return x[{}0{}]===a?a:b;}}",
                "(".repeat(depth),
                ")".repeat(depth)
            )
        };
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "depth={depth}, array={array}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            assert!(error.contains("nesting"), "{error}");
        }
    }
}
