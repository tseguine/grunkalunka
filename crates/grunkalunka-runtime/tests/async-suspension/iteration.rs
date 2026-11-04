use super::*;

#[test]
fn array_for_of_keeps_saved_arrays_and_live_length_across_suspension() {
    compare_promises(
        &[
            "let n=0;const source=[a,b];for(const item of (n=1,await a,n=n*10+2,source)){await b;n=n*10+(item===a?1:2);}return n;",
            "let source=[a,b];const original=source;let n=0;for(const item of source){await a;if(n===0){source=[b];original[1]=a;}n++;}await b;return n+(source!==original?10:0);",
            "const source=[a];let n=0;for(const item of source){await b;n++;if(n===1)source[2]=b;}return n;",
            "const source=[a,b,a];let n=0;for(const item of source){await a;n++;source.pop();}return n;",
            "let n=0;for(const item of (await a,[])){await b;n++;}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_for_of_preserves_selected_references_and_per_iteration_captures_after_await() {
    compare_promises(
        &[
            "let first=()=>0,last=()=>0;let n=0;for(const item of [a,b]){const read=()=>item===a?1:2;await a;if(n===0)first=read;last=read;n++;}await b;return first()*10+last();",
            "let first=()=>0,last=()=>0,change=()=>0;for(let item of [0,1]){await a;const read=()=>item===0?1:item===1?2:3;if(item===0){first=read;change=()=>{item=9;return 0;};continue;}last=read;}await b;change();return first()*10+last();",
            "const row={tag:a};const source=[row];let found=0;for(const item of source){await a;source[0]={tag:b};await b;found=item===row?1:0;}return found;",
            "let n=0;for(let item of [a,b]){await a;item=b;const read=()=>item===b?1:0;await b;n+=read();}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_for_of_routes_async_control_and_abrupt_completion() {
    compare_promises(
        &[
            "let n=0;for(const item of [0,1,2]){await a;if(item===1)continue;n++;if(item===2)break;}await b;return n;",
            "let n=0;for(const item of [0,1,2]){try{await a;if(item===0)continue;if(item===1)break;n=99;}finally{await b;n=n*10+1;}}return n;",
            "let n=0;for(const item of [0,1]){try{n=n*10+(item===0?1:2);break;}finally{await a;continue;}}await b;return n;",
            "let n=0;for(const item of [a,b]){try{n++;continue;}finally{await a;return item===a?n:9;}}return 0;",
            "let n=0;try{for(const item of [0,1,2]){n++;if(item===1)await Promise.reject(a);await b;}n=99;}catch(e){return n===2&&e===a?1:0;}return -1;",
            "let n=0;try{for(const item of [await Promise.reject(a)]){n=1;}}catch(e){return n===0&&e===a?1:0;}return -1;",
            "for(const item of [a,b]){await a;return item===a?1:2;}return 0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
    let source = "export async function run(a,b){for await(const item of [a,b]){}return a;}";
    for typed in [false, true] {
        let error = compile_async(source, typed)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}
