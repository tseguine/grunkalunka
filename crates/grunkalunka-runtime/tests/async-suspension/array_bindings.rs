use super::*;

#[test]
fn uint32_delete_keys_keep_receiver_and_effect_snapshots_across_await() {
    compare_promises(
        &[
            "let xs=[a,b];const saved=xs;let n=0;const removed=delete (n=1,await a,xs)[(n=n*10+2,await b,xs=[b],0>>>0)];return removed&&n===12&&!Object.hasOwn(saved,0)&&Object.hasOwn(xs,0)?1:0;",
            "const xs=[a];try{delete xs[(await Promise.reject(a),0>>>0)];}catch(e){await b;return Object.hasOwn(xs,0)?1:0;}return 0;",
            "const xs=[a,b];delete xs[(await a)>>>0];await b;return xs.length;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn debugger_statements_keep_async_effects_and_cleanup() {
    compare_promises(
        &[
            "debugger;const x=await a;debugger;return x;",
            "try{debugger;return await a;}finally{debugger;await b;debugger;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_literal_spreads_preserve_prefixes_and_effects_across_await() {
    compare_promises(
        &[
            "let n=0;const xs=[(n=1,await a),...[(n=n*10+2,await b)],(n=n*10+3,await a)];return xs.length+n;",
            "const child={v:a};const xs=[await a,...[child],await b];await a;return xs[1]===child?xs.length:0;",
            "let n=0;try{const xs=[(n=1,await a),...[(n=2,await Promise.reject(b))],(n=3,a)];return xs.length;}catch(e){await a;return n===2&&e===b?1:0;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn literal_call_spreads_preserve_argument_snapshots_across_await() {
    compare_promises(
        &[
            "const f=(x,y)=>x+y;let n=0;return f(...[(n=1,await a),(n=n*10+2,await b)])+n;",
            "let f=x=>x;return f(...[(await a,a),(f=x=>b,await b)]);",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_object_assignments_keep_utf16_source_across_default_awaits() {
    compare_promises(
        &[
            "let x,n=0;let source='';const result=({'99':x=(await a,source='abc','?'),length:n}=source);await b;return result===''&&n===0&&x==='?'&&source==='abc'?1:0;",
            "let x,n=0;({0:x=(n++,await Promise.reject(a))}='abc');await b;return n===0&&x==='a'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_array_assignments_keep_saved_rhs_and_copies_across_await() {
    compare_promises(
        &[
            "let x,y;let source='';const result=([x=(await a,source='ab','?'),y=(await b,'!')]=source);await a;return result===''&&source==='ab'&&x==='?'&&y==='!'?1:0;",
            "let x,n=0;[x=(n++,await Promise.reject(a))]='abc';await b;return n===0&&x==='a'?1:0;",
            r"let x,rest=[];const result=([x,...rest]='\u{1f600}ab');await a;return result.length===4&&x==='\u{1f600}'&&rest.length===2?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn assignment_defaults_preserve_waits_skips_and_abrupt_completion() {
    compare_promises(
        &[
            "let x=0,y=0;const source=[undefined,a];const result=([x=(await a,source[1]=b,a),y]=source);await b;return result===source&&x===a&&y===b?1:0;",
            "let x=0,y=0;const source=[];[x=(await a,source[1]=b,a),y]=source;await b;return x===a&&y===undefined?1:0;",
            "let x=0,n=0;[x=(n++,await Promise.reject(a))]=[b];await a;return n===0&&x===b?1:0;",
            "let x=0,n=0;try{({x=(await Promise.reject(a))}={});n=100;}catch(e){await b;return n===0&&e===a?1:0;}return -1;",
            "let x=0,y=0;const source={x:undefined,y:a};const result=({x=(await a,source.y=b,a),y}=source);await b;return result===source&&x===a&&y===b?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn assignment_planned_local_cells_keep_undefined_and_captures_across_await() {
    compare_promises(
        &[
            "let x;const read=()=>x;const before=read()===undefined;await a;[x]=[b];await a;return before&&read()===b?1:0;",
            "let x=0;const read=()=>x;({name:x}={name:(await a,'text')});await b;return read()==='text'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn assignment_patterns_keep_rhs_and_captured_cells_across_await() {
    compare_promises(
        &[
            "let x='';const read=()=>x;[x]=[(await a,'text')];await b;return read().length;",
            "let x='';const read=()=>x;({name:x}={name:(await a,'text')});await b;return read().length;",
            "let [x]=[];const source=[a];const result=([x]=(await b,source));await a;return result===source&&x===a?1:0;",
            "let x='';const source={name:'text'};const result=({name:x}=(await b,source));await a;return result===source&&x==='text'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_rest_bindings_keep_snapshots_and_abrupt_defaults_across_await() {
    compare_promises(
        &[
            "const child={tag:a};let source=[undefined,,child];const original=source;const [head=(await a,source=[b],b),...rest]=source;const copied=rest[0]===undefined&&rest[1]===child;await b;const keys=Object.keys(rest);original[2]={tag:b};return copied&&rest.length===2&&keys.length===2&&keys[0]==='0'&&keys[1]==='1'&&rest[0]===undefined&&rest[1]===child&&source!==original?1:0;",
            "const source=[];const [head=(await a,source[1]=b,a),...rest]=source;await b;return rest.length+(rest!==source?10:0);",
            "let n=0;try{const [first=(await Promise.reject(a),1),...rest]=[];n=rest.length+1;}catch(e){await b;return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn whole_array_defaults_keep_empty_patterns_and_saved_iterators_across_await() {
    compare_promises(
        &[
            "const outer=[];const [[]=(await a,outer[1]=9,[]),tail]=outer;await b;return (tail===undefined?10:0)+(tail===9?1:0);",
            "const inner=[undefined,a];let root=[];const original=root;const [[x=(await b,inner[1]=b,original[1]=b,a),y]=(await a,root=[[b]],inner),tail=9]=root;await a;return x===a&&y===b&&tail===9&&root!==original?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_array_binding_defaults_keep_each_iterator_live_or_exhausted_across_await() {
    compare_promises(
        &[
            "const inner=[];const outer=[inner];const [[x=(await a,inner[1]=7,outer[1]=9,1),y],z]=outer;await b;return (x===1?100:0)+(y===7?10:0)+(z===9?1:0);",
            "const inner=[,];const outer=[inner];const [[x=(await a,inner[1]=7,outer[1]=9,1),y],z]=outer;await b;return (x===1?100:0)+(y===7?10:0)+(z===9?1:0);",
            "const inner=[];const outer=[inner,inner];const [[x=(await a,inner[0]=7,1),y],[z,w]]=outer;await b;return (x===1?1000:0)+(y===undefined?100:0)+(z===7?10:0)+(w===undefined?1:0);",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_binding_patterns_retain_receivers_captures_and_abrupt_defaults_across_await() {
    compare_promises(
        &[
            "const inner=[undefined,a];const record={items:inner};const root=[record,b];const [{items:[x=(await a,record.items=[b,b],inner[1]=b,a),y]},z]=root;await b;return x===a&&y===b&&z===b&&record.items!==inner?1:0;",
            "let [{label}]=[{label:'before'}];const read=()=>label;await a;label='after';await b;return read()==='after'?1:0;",
            "let n=0;try{const [{items:[x=(n=1,await Promise.reject(a)),y=(n=2)]},z=(n=3)]=[{items:[]}];n=4;}catch(e){return n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_binding_defaults_suspend_only_on_undefined() {
    compare_promises(
        &[
            "let n=0;const [x=(await a,n=n*10+1),y=(await b,n=n*10+2)]=[0];await a;return x===0&&y===2?n:-1;",
            "const [x=await Promise.reject(a)]=[null];await b;return x===null?1:0;",
            "const [x=(await a,a),y=(await b,b)]=[,undefined];await a;return x===a&&y===b?1:0;",
            "const undefined=a;const [x=(await b,9)]=[];await a;return x===9?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_binding_defaults_keep_live_length_and_exhaustion_across_await() {
    compare_promises(
        &[
            "const source=[];const [x=(await a,source[1]=b,a),y=(await b,9)]=source;return x===a&&y===9?1:0;",
            "const source=[];const [x=(await a,source[1]=b,a),y]=source;await b;return x===a&&y===undefined?1:0;",
            "const source=[,];const [x=(await a,source[1]=b,a),y=(await b,9)]=source;return x===a&&y===b?1:0;",
            "const source=[undefined,8,9];const [x=(await a,source.length=0,a),y=(await b,source[2]=b,2),z=(await a,3)]=source;return x===a&&y===2&&z===3?1:0;",
            "const source=[undefined,8,9];const [x=(await a,source.length=0,await b,source[2]=b,a),y=(await a,2),z=3]=source;return x===a&&y===2&&z===b?1:0;",
            "const source=[,];const [x=(await a,source[2]=b,a),,y=(await b,9)]=source;return x===a&&y===b?1:0;",
            "const source=[,];const [,x=(await a,source[2]=b,a),y=(await b,9)]=source;return x===a&&y===9?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_binding_defaults_retain_the_original_array_and_prior_values_across_await() {
    compare_promises(
        &[
            "let source=[undefined,a];const old=source;const [x=(await a,source=[b,b],old[1]=b,a),y]=source;await b;return x===a&&y===b&&source!==old?1:0;",
            "const source=[undefined,a];const alias=source;const [x=(await a,alias[1]=b,a),y]=(await b,source);await a;return x===a&&y===b?1:0;",
            "const [x=(await a,a),y=(await b,x),z=(await a,y)]=[];return x===a&&y===x&&z===y?1:0;",
            "const selected={tag:a};const [x=(await a,selected)]=[];await b;selected.tag=b;return x===selected?1:0;",
            "const source=[{tag:a},undefined];const [saved,x=(await a,source[0]={tag:b},a)]=source;await b;return saved.tag===a&&source[0]!==saved?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_binding_defaults_preserve_rejection_helpers_and_live_captures() {
    compare_promises(
        &[
            "let n=0;try{const [x=await Promise.reject(a),y=(n=1)]=[];n=2;}catch(e){return n===0&&e===a?1:0;}return -1;",
            "let n=0;try{const [x=(await a,n=1,a),y=await Promise.reject(b),z=(n=3)]=[];n=4;}catch(e){return n===1&&e===b?1:0;}return -1;",
            "const fallback=()=>{return {label:'ready'};};const [x=(await a,fallback())]=[];await b;return x!==null?1:0;",
            "let [x=(await a,a)]=[];const read=()=>x===b?1:0;await b;x=b;await a;return read();",
            "const [x=(await a,a),y=(await b,x)]=[];const read=()=>y===a?1:0;await b;return read();",
            "let n=0;try{const [x=await Promise.reject(a),y=(n=9)]=[];}catch(e){n=1;}finally{await b;n=n*10+2;}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_bindings_keep_selected_values_origins_and_captures_across_suspension() {
    compare_promises(
        &[
            "const [value,label]=[await a,' ready '];await b;return value===a&&label.trim()==='ready'?1:0;",
            "const source=[a,{tag:b}];const [value,child]=source;await b;source[0]=b;source[1]={tag:a};await a;return value===a&&child.tag===b?1:0;",
            "const source=[[a,b]];const [items]=source;await a;items.pop();await b;return items.length===1&&source[0]===items?1:0;",
            "let [label]=['before'];const read=()=>label;await a;label='after';await b;return read()==='after'?1:0;",
            "const [hole,,last,missing]=[,a,b];await a;return hole===undefined&&last===b&&missing===undefined?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_binding_sources_resume_once_and_preserve_order_and_abrupt_completion() {
    compare_promises(
        &[
            "let n=0;const [value]=(n=1,await a,n=n*10+2,[b]);await a;return value===b?n:-1;",
            "const [first]=[a],[second]=[(await b,first)];await a;return first===a&&second===a?1:0;",
            "const build=()=>{return [' x ',a];};const [label,value]=build();await b;return label.trim()==='x'&&value===a?1:0;",
            "let n=0;try{const [value]=[await Promise.reject(a)];n=1;}catch(e){return n===0&&e===a?1:0;}return -1;",
            "let n=0;for(const [label]=[(await a,'x')];n<1;n++){await b;return label==='x'?1:0;}return -1;",
            "const []=(await a,[]),[value]=(await b,[a]);await a;return value===a?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_rest_positions_keep_traced_values_across_awaits() {
    compare_promises(
        &[
            "const source=[a,{label:' before '},' end '];const [,...tail]=source;source[1]={label:' replacement '};await b;const row=tail[0];await a;return row.label.trim()==='before'&&tail[1].trim()==='end'?1:0;",
            "const [,...tail]=[a,,{label:'last'}];const [first=(await a,' empty ')]=tail;await b;return first.trim()==='empty'&&tail[1].label.trim()==='last'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn rest_target_defaults_suspend_after_the_entire_tail_is_copied() {
    compare_promises(
        &[
            "const src=[a,undefined,' old '];const [,...[x=(await a,src[2]='changed',b),label]]=src;await b;return x===b&&label.trim()==='old'&&src[2]==='changed'?1:0;",
            "let n=0;try{const [...[x=await Promise.reject(a),y=(n=1,b)]]=[];n=2;}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_property_bindings_read_live_length_and_saved_receivers_after_await() {
    compare_promises(
        &[
            "const src=[undefined,b];const {0:first=(await a,src.length=0,src[2]=a,1),length:n,...rest}=src;await b;return n===3&&rest[2]===a&&rest.length===undefined?1:0;",
            "const src=[a,undefined,' saved '];const [,...{0:x=(await a,src[2]='changed',b),1:y,length:n}]=src;return x===b&&y.trim()==='saved'&&n===2?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}
