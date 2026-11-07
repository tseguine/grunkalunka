#[test]
fn primitive_exports_synthesize_undefined_only_on_normal_completion() {
    for body in [
        "",
        "a++;",
        "const undefined=b;",
        "if(a) return null;",
        "if(a)return;",
        "{if(a)return b;}",
        "while(a){if(b)break;return null;}",
        "do{if(a)continue;return b;}while(0);",
        "do{if(a)break;return b;}while(0);",
        "switch(a){case 1:return null;default:b++;}",
        "try{if(a)return null;}finally{b++;}",
        "try{a++;}finally{if(b)return b;}",
        "try{throw a;}catch(e){if(b)return e;}",
    ] {
        let js = format!("export function run(a,b) {{{body}}}");
        let ts = format!("export function run(a:number,b:number):unknown {{{body}}}");
        for (source, typed) in [(&js, false), (&ts, true)] {
            let result = crate::compile_value(source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
            assert!(crate::compile(source, typed).is_err(), "{source}");
        }
    }
}

#[test]
fn implicit_undefined_keeps_source_validation_and_private_number_guards() {
    for source in [
        "export function run(a,b){Math.random();}",
        "export function run(a,b){return b;Math.random();}",
        "export function run(a,b){if(a){missing;}}",
        "export function run(a,b){break;}",
        "export function run(a,b){continue;}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    assert_eq!(
        crate::compile_value("export function run(a,b){}", false)
            .unwrap()
            .wasm,
        crate::compile_value("export function run(a,b){return;}", false)
            .unwrap()
            .wasm
    );
}
