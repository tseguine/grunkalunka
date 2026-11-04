use super::*;

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
