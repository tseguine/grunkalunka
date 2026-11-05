use super::*;

#[test]
fn whole_pattern_defaults_in_for_of_heads_keep_fresh_cells_and_abrupt_completion() {
    compare_promises(
        &[
            "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let {row:{label}=(await a,n++,{label:n===1?' a ':' bb '}),unused=(last=()=>label.trim().length,0)} of [{},{}]){await b;if(n===1){first=last;change=()=>{label=' ccc ';return 0;};}}await a;change();return first()*10+last();",
            "let n=0;try{for(const {child:{x=(n=1)}=(await Promise.reject(a),{}),tail=(n=2)} of [{}])n=3;}catch(e){await b;return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

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
fn array_for_of_keeps_proven_values_and_fresh_cells_across_suspension() {
    compare_promises(
        &[
            "let n=0;for(const item of (await a,[' x ',' yy '])){await b;n=n*10+item.trim().length;}return n;",
            "const make=()=>[{label:' x ',values:[a,b]}];let n=0;for(const row of make()){await a;n+=row.label.trim().length+(row.values.at(-1)===b?10:0);}await b;return n;",
            "let n=0;for(const values of [[' x '],[],[' yy ']]){await a;for(const item of values){await b;n=n*10+item.trim().length;}}return n;",
            "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let item of [' x ',' yy ']){const read=()=>item.trim().length;await a;if(n===0){first=read;change=()=>{item=' zzz ';return 0;};n++;continue;}last=read;}await b;change();return first()*10+last();",
            "let first=()=>0,last=()=>0;let n=0;for(const row of [{f:()=>1},{f:()=>2}]){const read=()=>row.f();await a;if(n===0)first=read;last=read;n++;}await b;return first()*10+last();",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_for_of_keeps_safe_write_proofs_and_rejects_taint_across_suspension() {
    compare_promises(
        &[
            "const row={label:' a '};const source=[row,{label:' bb '}];const alias=source;let n=0;for(const item of source){await a;alias[0]={label:' ccc '};alias[1]={label:' dddd '};await b;n=n*10+item.label.trim().length+(item===row?100:0);}return n;",
            "const source=[' a ',' bb '];const replace=()=>{source[1]=' ccc ';return source;};let n=0;for(const item of replace()){await a;source.named=b;n=n*10+item.trim().length;}await b;return n;",
            "let n=0;for(const item of (await a,a?[]:[' x '])){await b;n+=item.trim().length;}await a;return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
    for body in [
        "const items=[' x ',' yy '];const alias=items;let n=0;for(const item of items){await a;alias[1]=b;n+=item.trim().length;}return n;",
        "const items=[' x ',' yy '];const alias=()=>items;let n=0;for(const item of items){await a;alias()[1]=b;n+=item.trim().length;}return n;",
        "const items=['ready'];await a;items.length=2;let n=0;for(const item of items){await b;n+=item.trim().length;}return n;",
        "for(let item of ['ready']){const read=()=>item.trim().length;await a;item=b;await a;return read();}return 0;",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = compile_async(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
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

#[test]
fn destructuring_for_of_suspends_in_heads_and_retains_nested_receivers() {
    compare_promises(
        &[
            "let n=0;for(const {first=(await a,1),next=first} of [{},{first:null}]){await b;n=n*10+(first===next?1:2);}return n;",
            "const row={inner:{first:undefined,next:a},tail:a};let n=0;for(const {inner:{first=(await a,row.inner={next:b},row.tail=b,1),next},tail} of [row]){await b;n=(first===1?100:0)+(next===a?10:0)+(tail===b?1:0);}return n;",
            "let n=0;for(const [first=(await a,1),next=first] of [[],[null]]){await b;n=n*10+(first===next?1:2);}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn destructuring_for_of_keeps_fresh_cells_and_abrupt_head_completion() {
    compare_promises(
        &[
            "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let {label,unused=(await a,last=()=>label.trim().length,0)} of [{label:' a '},{label:' bb '}]){if(n===0){first=last;change=()=>{label=' ccc ';return 0;};}n++;}await b;change();return first()*10+last();",
            "let read=()=>0,n=0;try{for(const {label,check=(read=()=>label.trim().length,await Promise.reject(a)),last=(n=99)} of [{label:' abc '}])n=100;}catch(e){await b;return read()*10+n+(e===a?1:0);}return 0;",
            "let n=0;for(const {tag} of [{tag:0},{tag:1}]){try{for(const {x=await Promise.reject(a),y=(n=99)} of [{}])n=100;}catch(e){await a;n=n*10+(e===a?1:2);continue;}finally{await b;n=n*10+3;}}return n;",
            "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let {label} of [{label:' a '},{label:' bb '}]){const read=()=>label.trim().length;await a;if(n===0){first=read;change=()=>{label=' ccc ';return 0;};n++;continue;}last=read;}await b;change();return first()*10+last();",
            "let n=0;try{for(const {first=await Promise.reject(a),next=(n=99)} of [{}])n=100;}catch(e){await b;return n===0&&e===a?1:0;}return -1;",
            "let n=0;for(const {value} of [{value:0},{value:1}]){try{await a;if(value===0)continue;break;}finally{await b;n=n*10+(value===0?1:2);}}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}
