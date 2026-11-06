use super::*;

#[test]
fn object_rest_retains_the_receiver_across_a_default_await() {
    compare_promises(
        &[
            "let source={tail:a};const original=source;const {first=(await a,source={other:a},original.tail=b,1),...rest}=source;await b;return first===1&&rest.tail===b&&rest.other===undefined&&rest!==original?1:0;",
            "let n=0;try{const {x=(await Promise.reject(a),1),...rest}={};n=Object.keys(rest).length+1;}catch(e){await b;return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn whole_object_defaults_keep_saved_receivers_and_skip_unused_awaits() {
    compare_promises(
        &[
            "let fallback={first:undefined,next:a};const original=fallback;const root={tail:a};const {child:{first=(await b,fallback={next:b},root.child={next:b},root.tail=b,1),next}=(await a,fallback),tail}=root;return (first===1?100:0)+(next===a?10:0)+(tail===b?1:0)+(fallback!==original?1000:0);",
            "const {child:{value}=(await Promise.reject(a),{value:b})}={child:{value:a}};await b;return value===a?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn known_undefined_defaults_keep_proven_consumers_and_lazy_awaits() {
    compare_promises(
        &[
            "let n=0;const source=a?{label:' chosen '}:{};const {label=(await b,n=1,' fallback ')}=source;await a;return label.trim().length+n*100;",
            "const {row=(await a,{label:' ready '})}={};const [items=(await b,[row])]=[];await a;return items.length+(items.at(0)===row?10:0)+row.label.trim().length*100;",
            "const {label=(await Promise.reject(a),'bad')}={label:' safe '};await b;return label.trim().length;",
            "let n=0;try{const {label=(await Promise.reject(a),'bad'),after=(n=1,'bad')}={};n=2;}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_object_binding_defaults_keep_receivers_order_and_captures_across_await() {
    compare_promises(
        &[
            "const child={y:a};let source={child,tail:b};const old=source;const {child:{x=(await a,source.child={y:b},source={child:{y:b},tail:a},child.y=b,a),y},tail}=source;await b;return x===a&&y===b&&tail===b&&source!==old?1:0;",
            "let n=0;const {left:{x=(await a,n=n*10+1,a)},right:{y=(await b,n=n*10+2,x)}}={left:{},right:{}};await a;return n+(x===a?100:0)+(y===x?1000:0);",
            "let {child:{label}}={child:{label:'before'}};const read=()=>label;await a;label='after';await b;return read()==='after'?1:0;",
            "let n=0;try{const {before=(n=1,a),child:{x=await Promise.reject(b),y=(n=2)},after=(n=3)}={child:{}};n=4;}catch(e){return n===1&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_binding_defaults_suspend_only_on_undefined() {
    compare_promises(
        &[
            "let n=0;const {x=(await a,n=n*10+1),y=(await b,n=n*10+2)}={x:0};await a;return x===0&&y===2?n:-1;",
            "const {x=await Promise.reject(a)}={x:null};await b;return x===null?1:0;",
            "const {x=await a,y=await b}={x:undefined};await a;return x===a&&y===b?1:0;",
            "const undefined=a;const {x=(await b,9)}={};await a;return x===9?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_binding_defaults_keep_the_original_object_and_read_later_fields_after_await() {
    compare_promises(
        &[
            "let source={y:a};const old=source;const {x=(await a,source={y:b},b),y}=source;await b;return x===b&&y===a&&source!==old?1:0;",
            "const source={y:a};const alias=source;const {x=(await a,alias.y=b,a),y}=source;await b;return x===a&&y===b?1:0;",
            "const source={};const {x:first=(await a,source.x=b,a),x:second}=source;await b;return first===a&&second===b?1:0;",
            "const {x=(await a,a),y=(await b,x)}={};await a;return x===a&&y===a?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_binding_defaults_preserve_rejection_and_source_block_calls() {
    compare_promises(
        &[
            "let n=0;try{const {x=await Promise.reject(a),y=(n=1)}={};n=2;}catch(e){return n===0&&e===a?1:0;}return -1;",
            "const fallback=()=>{return {label:'ready'};};const {x=(await a,fallback())}={};await b;return x!==null?1:0;",
            "const build=()=>{const {x=a}={};return x===a?1:0;};const {y=(await b,build())}={};await a;return y===1?1:0;",
            "let n=0;try{const {x=await Promise.reject(a)}={};}catch(e){n=1;}finally{n=n*10+2;}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_binding_defaults_keep_live_captures_after_suspension() {
    compare_promises(
        &[
            "let {x=(await a,a)}={};const read=()=>x===b?1:0;await b;x=b;await a;return read();",
            "const {x=(await a,a),y=(await b,x)}={};const read=()=>y===a?1:0;await b;return read();",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_bindings_keep_values_and_origins_across_suspension() {
    compare_promises(
        &[
            "const {value,label}={value:await a,label:' ready '};await b;return value===a&&label.trim()==='ready'?1:0;",
            "const source={value:a,child:{tag:b}};const {value,child}=source;await b;source.value=b;source.child={tag:a};await a;return value===a&&child.tag===b?1:0;",
            "const source={items:[a,b]};const {items}=source;await a;items.pop();await b;return items.length===1&&source.items===items?1:0;",
            "let {label}={label:'before'};const read=()=>label;await a;label='after';await b;return read()==='after'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_binding_sources_resume_once_and_preserve_abrupt_completion() {
    compare_promises(
        &[
            "let n=0;const {value}=(n=1,await a,n=n*10+2,{value:b});await a;return value===b?n:-1;",
            "const {first}={first:a},{second}={second:(await b,first)};await a;return first===a&&second===a?1:0;",
            "const build=()=>{return {label:' x ',value:a};};const {label,value}=build();await b;return label.trim()==='x'&&value===a?1:0;",
            "let n=0;try{const {value}={value:await Promise.reject(a)};n=1;}catch(e){return n===0&&e===a?1:0;}return -1;",
            "let n=0;for(const {label}={label:(await a,'x')};n<1;n++){await b;return label==='x'?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_rest_strings_and_records_retain_their_types_across_await() {
    compare_promises(
        &[
            "const source={label:' before ',row:{value:a}};const {skip=(await a,source.label=' copied ',1),...rest}=source;await b;source.label='after';return rest.label.trim()==='copied'&&rest.row.value===a?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn computed_dictionary_keys_survive_suspension_and_abrupt_completion() {
    compare_promises(
        &[
            "const d=Object.create(null);d.first=a;d.second=b;d.third=a;let k='first';const {[k]:x,[(await a,k='third','second')]:y,...rest}=d;await b;const keys=Object.keys(rest);return x===a&&y===b&&keys.length===1&&keys[0]==='third'&&rest.third===a?1:0;",
            "const d=Object.create(null);d.keep=b;let n=0;try{const {[(n=1,await Promise.reject(a),'absent')]:x=(n=2,1),...rest}=d;n=3;}catch(e){await b;return n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}
