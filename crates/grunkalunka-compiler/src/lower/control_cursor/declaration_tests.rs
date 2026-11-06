//! Declarations schedule one initializer at a time while preserving publication.
use super::SCHEDULED_DECLARATION_VALUES;

#[test]
fn object_rest_bindings_keep_receiver_and_scope_guards() {
    for (body, admitted) in [
        ("const {...rest}={value:a};return a;", true),
        ("const {child:{...rest}={}}={};return a;", true),
        (
            "const [ {x,...rest} ]=[{x:a,y:b}];return rest.y===b?1:0;",
            true,
        ),
        ("const {x,...rest}=a;return a;", false),
        ("const {x,...rest}=[a];return a;", false),
        ("const {child:{...rest}}={child:null};return a;", false),
        ("const {first=rest,...rest}={};return a;", false),
        ("const {...rest}={};rest={};return a;", false),
        (
            "let {...rest}={value:a};rest=null;return Object.keys(rest).length;",
            false,
        ),
        (
            "const {...rest}={label:a};const {label='x'}=rest;return label.trim().length;",
            false,
        ),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let (result, _) = on_small_stack(&source, false, typed);
            if admitted {
                assert!(result.is_ok(), "{source}: {result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(
                    error.contains("unsupported") && !error.contains("internal IR"),
                    "{source}: {error}"
                );
            }
        }
    }
}

#[test]
fn array_rest_bindings_compose_with_defaults_captures_and_iteration() {
    for (body, asynchronous) in [
        (
            "const [head,...rest]=[a,b];return rest.length+(head===a?1:0);",
            false,
        ),
        ("const [,,...rest]=[a,b,a];return rest.length;", false),
        ("const [[...rest]=[]]=[];return a;", false),
        (
            "const {items:[first,...rest]=[a,b]}={};return rest.length;",
            false,
        ),
        ("const [...rest]=Object.keys(a);return rest.length;", false),
        (
            "const source=[];const [head=(source[1]=b,a),,...rest]=source;return rest.length;",
            false,
        ),
        (
            "let first=()=>0,last=()=>0,n=0;for(const [head,...rest] of [[a,b],[b,a,b]]){const read=()=>+rest.length;if(n===0)first=read;last=read;n++;}return first()*10+last();",
            false,
        ),
        (
            "const source=[,];const [head=(await a,source[1]=b,a),...rest]=source;await b;return rest.length;",
            true,
        ),
    ] {
        let source = format!(
            "export {}function run(a,b){{{body}}}",
            if asynchronous { "async " } else { "" }
        );
        for typed in [false, true] {
            let (result, _) = on_small_stack(&source, asynchronous, typed);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
    }
}

#[test]
fn array_rest_bindings_keep_target_receiver_and_element_proof_guards() {
    for body in [
        "const [...[value]]=[a];return a;",
        "const [...{length}]=[a];return a;",
        "const [...rest]=a;return a;",
        "const [...rest]='text';return a;",
        "const [first=rest,...rest]=[];return a;",
        "const [...rest]=rest;return a;",
        "const [...rest]=[];rest=[];return a;",
        "const [...rest]=['text'];return rest[0].trim().length;",
        "const [...rest]=[[a]];const [[value]]=rest;return a;",
        "const [...rest]=['text'];const [label='fallback']=rest;return label.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let error = on_small_stack(&source, false, false)
            .0
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
    let typed = "export function run(a:number,b:number){const [...rest]:string[]=['text'];return rest[0].trim().length;}";
    let error = on_small_stack(typed, false, true)
        .0
        .err()
        .unwrap_or_else(|| panic!("admitted {typed}"));
    assert!(error.contains("unsupported"), "{error}");
}

#[test]
fn array_rest_own_keys_uses_positive_container_proofs_without_losing_nullable_guards() {
    for body in [
        "const [...rest]=[,a];return Object.keys(rest).length;",
        "const [...rest]=Object.keys(a);return Object.keys(rest).length;",
        "const [items]=[[a]];return Object.keys(items).length;",
        "const {record={value:a}}={};return Object.keys(record).length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let (result, _) = on_small_stack(&source, false, false);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
    for body in [
        "let [...rest]=[a];rest=null;return Object.keys(rest).length;",
        "let [...rest]=[a];rest=a;return Object.keys(rest).length;",
        "let [...rest]=[a];rest=a?rest:null;return Object.keys(rest).length;",
        "let [...rest]=[a];return Object.keys(rest,rest=null).length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let error = on_small_stack(&source, false, false)
            .0
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("nullable Object.keys"), "{source}: {error}");
    }
    // The async profile retains its runtime check on the original saved value,
    // including after a surplus argument changes the source variable.
    let source = "export async function run(a,b){let [...rest]=[a];return Object.keys(rest,rest=null).length;}";
    let (result, _) = on_small_stack(source, true, false);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn array_rest_counts_one_leaf_without_an_extra_container_level() {
    for count in [128, 129] {
        let prefix = (0..count - 1)
            .map(|index| format!("value{index}"))
            .collect::<Vec<_>>()
            .join(",");
        let source =
            format!("export function run(a,b){{const [{prefix},...rest]=[];return rest.length;}}");
        let (result, _) = on_small_stack(&source, false, false);
        if count == 128 {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let error = result.unwrap_err();
            assert!(error.contains("at most 128 source locals"), "{error}");
        }
    }
    for depth in [128, 129] {
        let mut pattern = String::from("[...rest]");
        let mut initializer = String::from("[a]");
        for _ in 1..depth {
            pattern = format!("[{pattern}]");
            initializer = format!("[{initializer}]");
        }
        let source = format!(
            "export function run(a,b){{const {pattern}={initializer};return rest.length;}}"
        );
        let (result, _) = on_small_stack(&source, false, false);
        if depth == 128 {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let error = result.unwrap_err();
            assert!(
                error.contains("binding pattern nesting exceeds 128"),
                "{error}"
            );
        }
    }
}

#[test]
fn identifier_defaults_admit_consumers_of_proven_missing_values() {
    for (body, asynchronous) in [
        ("const {label=' x '}={};return label.trim().length;", false),
        ("const [label=' x ']=[,];return label.trim().length;", false),
        ("const [values=[a,b]]=[];return values.length;", false),
        (
            "const {record={label:' x '}}={};return record.label.trim().length;",
            false,
        ),
        (
            "let missing;const {label=' x '}={label:missing};return label.trim().length;",
            false,
        ),
        (
            "const source=Object.create(null);const {constructor:label=' x '}=source;return label.trim().length;",
            false,
        ),
        (
            "const source={...{label:a},label:undefined};const {label=' x '}=source;return label.trim().length;",
            false,
        ),
        (
            "let n=0;for(const {label=' x '} of [{}])n+=label.trim().length;return n;",
            false,
        ),
        (
            "const {label=(await a,' x ')}={};return label.trim().length;",
            true,
        ),
    ] {
        let source = format!(
            "export {}function run(a,b){{{body}}}",
            if asynchronous { "async " } else { "" }
        );
        for typed in [false, true] {
            let (result, _) = on_small_stack(&source, asynchronous, typed);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
    }
}

#[test]
fn identifier_defaults_retain_unknown_guards() {
    for body in [
        "const {label=' x '}={label:null};return label.trim().length;",
        "const undefined=a;const {label=' x '}={label:undefined};return label.trim().length;",
        "const source={...{label:a}};const {label=' x '}=source;return label.trim().length;",
        "const source={label:undefined,...{label:a}};const {label=' x '}=source;return label.trim().length;",
        "const source=[];const {label=' x '}={label:source.length};return label.trim().length;",
        "const source={};const {constructor:label=' x '}=source;return label.trim().length;",
        "let missing;missing++;const {label=' x '}={label:missing};return label.trim().length;",
        "let missing=undefined;missing += a;const {label=' x '}={label:missing};return label.trim().length;",
        "let missing;const mutate=()=>{missing++;};mutate();const {label=' x '}={label:missing};return label.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let error = on_small_stack(&source, false, false)
            .0
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
    for mutation in ["=a", "+=a", "++"] {
        let source = format!(
            "export function run(a,b){{let missing=undefined;(missing as unknown){mutation};const {{label=' x '}}={{label:missing}};return label.trim().length;}}"
        );
        let error = on_small_stack(&source, false, true)
            .0
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn nested_pattern_defaults_use_the_post_default_receiver_before_binding_leaves() {
    for (body, asynchronous) in [
        ("const {child:{label}={label:' x '}}={};return a;", false),
        ("const [[label]=[' x ']]=[];return a;", false),
        ("const {child:{x}={x:a}}={child:{x:b}};return a;", false),
        ("const [[x]=[a]]=[[b]];return a;", false),
        ("const [{x}={x:a}]=[{x:b}];return a;", false),
        (
            "const source=a?{}:{child:{label:'present'}};const {child:{label}={label:'fallback'}}=source;return label.trim().length;",
            false,
        ),
        (
            "const {child:{constructor:label=' x '}=Object.create(null)}={};return label.trim().length;",
            false,
        ),
        ("const [[]=[],{}={}]=[,];return a;", false),
        (
            "const outer=[];const [[]=(outer[1]=9,[]),tail]=outer;return tail===undefined?1:0;",
            false,
        ),
        (
            "const outer=[,];const [[]=(outer[1]=9,[]),tail]=outer;return tail===9?1:0;",
            false,
        ),
        (
            "let n=0;for(const {child:{label}={label:' x '}} of [{}])n+=label.trim().length;return n;",
            false,
        ),
        (
            "const {child:{label}=(await a,{label:' x '})}={};return label.trim().length;",
            true,
        ),
        (
            "const [[label]=(await a,[' x '])]=[];return label.trim().length;",
            true,
        ),
    ] {
        let source = format!(
            "export {}function run(a,b){{{body}}}",
            if asynchronous { "async " } else { "" }
        );
        for typed in [false, true] {
            let (result, _) = on_small_stack(&source, asynchronous, typed);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
    }
}

#[test]
fn nested_pattern_defaults_keep_kind_tdz_and_unsupported_pattern_guards() {
    for body in [
        "const {child:{}={}}={child:null};return a;",
        "const {child:{}={}}={child:a};return a;",
        "const {child:{}=[]}={};return a;",
        "const [[]=[]]=[null];return a;",
        "const [[]={}]=[];return a;",
        "const {child:{}={}}={...{child:null}};return a;",
        "const source={};const alias=source;alias.child=null;const {child:{}={}}=source;return a;",
        "const {child:{value}={value:value}}={};return a;",
        "const {child:{}=(later,{}),later=a}={};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let error = on_small_stack(&source, false, false)
            .0
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn nested_pattern_default_annotations_do_not_supply_receiver_proofs() {
    let typed = "export function run(a:number,b:number){const {child:{label}={label:' x '}}:{child?:{label:string}}={};return label.trim().length;}";
    let (result, _) = on_small_stack(typed, false, true);
    assert!(result.is_ok(), "{result:?}");

    let typed = "export function run(a:number,b:number){const source={child:a} as {child?:{label:string}};const {child:{label}={label:' x '}}=source;return label.trim().length;}";
    let error = on_small_stack(typed, false, true)
        .0
        .err()
        .unwrap_or_else(|| panic!("admitted {typed}"));
    assert!(error.contains("source-proven managed object"), "{error}");
}

#[test]
fn nested_defaults_charge_containers_without_charging_default_wrappers() {
    for (depth, admitted) in [(32, true), (128, true), (129, false)] {
        let mut pattern = String::from("leaf");
        for _ in 0..depth {
            pattern = format!("{{next:{pattern}={{}}}}");
        }
        let source = format!("export function run(a,b){{const {pattern}={{}};return a;}}");
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
}

#[test]
fn nested_defaults_charge_only_bound_identifiers() {
    for (count, admitted) in [(128, true), (129, false)] {
        let fields = (0..count)
            .map(|index| format!("child{index}:{{leaf{index}}}={{}}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {{empty:{{}}={{}},{fields}}}={{}};return a;}}"
        );
        let (result, _) = on_small_stack(&source, false, false);
        if admitted {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let error = result.unwrap_err();
            assert!(error.contains("at most 128 source locals"), "{error}");
        }
    }
}

#[test]
fn for_of_patterns_use_the_shared_cursor_with_lexical_admission() {
    for (body, asynchronous) in [
        (
            "let n=0;for(const {row:{label},items:[value]} of [{row:{label:' x '},items:[a]}])n+=label.trim().length+(value===a?1:0);return n;",
            false,
        ),
        (
            "let n=0;for(const {first=(await a,1),next=first} of [{}]){await b;n+=first===next?1:0;}return n;",
            true,
        ),
        (
            "let first=()=>0,last=()=>0;let n=0;for(let [label] of [[' x '],[' yy ']]){const read=()=>label.trim().length;if(n===0)first=read;last=read;n++;}return first()*10+last();",
            false,
        ),
    ] {
        let source = format!(
            "export {}function run(a,b){{{body}}}",
            if asynchronous { "async " } else { "" }
        );
        for typed in [false, true] {
            let (result, _) = on_small_stack(&source, asynchronous, typed);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
    }
    for body in [
        "for(const {value} of [a]){}return a;",
        "for(const {value=value} of [{}]){}return a;",
        "for(const {first=next,next=a} of [{}]){}return a;",
        "for(const [value] of [[a]]){}return value;",
        "for(const {value} of [{value:a}])value=b;return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let error = on_small_stack(&source, false, false)
            .0
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

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
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let (result, _) = on_small_stack(&source, false, false);
        let error = result.err().unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn mixed_binding_patterns_share_the_small_stack_depth_and_receiver_guards() {
    for (depth, admitted) in [(32, true), (128, true), (129, false)] {
        let mut pattern = String::from("value");
        let mut initializer = String::from("a");
        for index in 0..depth {
            if index % 2 == 0 {
                pattern = format!("[{pattern}]");
                initializer = format!("[{initializer}]");
            } else {
                pattern = format!("{{next:{pattern}}}");
                initializer = format!("{{next:{initializer}}}");
            }
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
        "const [[]]=[];return a;",
        "const [[]]=[null];return a;",
        "const {items:[]}={items:{}};return a;",
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
fn managed_and_stored_for_let_header_closures_compile_on_small_stacks() {
    for source in [
        "export function run(a,b){let first=()=>({x:a});let last=()=>({x:b});for(let i=0;i<3;i++){const make=()=>({x:i});if(i===0)first=make;last=make;}return first().x===0&&last().x===2?a:b;}",
        "export function run(a,b){let first=()=>a;for(let i=0;i<3;i++){first=()=>i;}const o={first};return first();}",
    ] {
        for typescript in [false, true] {
            let (result, _) = on_small_stack(source, false, typescript);
            assert!(
                result.is_ok(),
                "{source}, typescript={typescript}: {result:?}"
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

#[test]
fn chained_object_rest_copies_keep_field_proofs_on_a_small_stack() {
    let mut source = String::from("export function run(a,b){const first={label:' ready '};");
    let mut previous = "first".to_string();
    for index in 0..48 {
        let next = format!("r{index}");
        source.push_str(&format!("const {{...{next}}}={previous};"));
        previous = next;
    }
    source.push_str(&format!("return {previous}.label.trim().length;}}"));
    for typed in [false, true] {
        let (result, _) = on_small_stack(&source, false, typed);
        assert!(result.is_ok(), "{result:?}");
    }
}

#[test]
fn computed_binding_keys_require_dictionary_and_string_proofs() {
    for (body, admitted) in [
        (
            "const d=Object.create(null);const k='x';const {[k]:v,...rest}=d;return Object.keys(rest).length;",
            true,
        ),
        (
            "const d=Object.create(null);const k=a?'x':'y';const {[k]:v}=d;return v===undefined?1:0;",
            true,
        ),
        ("const d={x:a};const k='x';const {[k]:v}=d;return a;", false),
        (
            "const d=a?Object.create(null):{};const k='x';const {[k]:v}=d;return a;",
            false,
        ),
        (
            "const d=Object.create(null);const k=a?'x':b;const {[k]:v}=d;return a;",
            false,
        ),
        (
            "const d=Object.create(null);const k='x';const {[k]:v='ok'}=d;return v.trim().length;",
            false,
        ),
        (
            "const d=Object.create(null);const k='x';const {[k]:v,...rest}=d;const {label='ok'}=rest;return label.trim().length;",
            false,
        ),
        (
            "const d=Object.create(null);const {[k]:v,k='x'}=d;return a;",
            false,
        ),
        (
            "const d=Object.create(null);d.key='x';const {key,[key]:v}=d;return a;",
            false,
        ),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let (result, _) = on_small_stack(&source, false, typed);
            if admitted {
                assert!(result.is_ok(), "{source}: {result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(
                    error.contains("unsupported") && !error.contains("internal IR"),
                    "{source}: {error}"
                );
            }
        }
    }
}
