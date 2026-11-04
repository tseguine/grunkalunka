use super::*;

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
