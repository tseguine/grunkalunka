use super::*;

fn compare_vars(sources: Vec<String>) {
    compare_primitive_sources(
        sources
            .into_iter()
            .map(|js| {
                let ts = js.replace(
                    "function run(a,b)",
                    "function run(a:unknown,b:unknown):unknown",
                );
                (js, ts)
            })
            .collect(),
    );
}

#[test]
fn hoisted_var_cells_preserve_values_scope_and_shared_captures() {
    compare_vars([
        "var x;return x;",
        "const before=x;var [x]=[a];return before;",
        "var [x=x]=[];return x;",
        "var [x,x]=[3,null];return x;",
        "var [x=3,y=x]=[];return y;",
        "const get=()=>x;var {x}={x:a};var {x}={x:b};return get();",
        "var [a]=[null];return a;",
        "var {x,...rest}={x:a,y:b};return rest.y;",
        "var [x,...rest]=[a,b];return rest[0];",
        "const obj={v:a};var {x}={x:obj};return x===obj?1:0;",
        "const before=key;for(var key in {}){}return before===undefined?1:0;",
        "var key=3;for(var key in {}){}return key===3?1:0;",
        "var key=3;for(var key in {x:a}){}return typeof key==='string'?1:0;",
        "const get=()=>a;for(var a in {x:b}){}return get()==='x'?1:0;",
        "for(var item of [null,b]){}return item;",
        "var item=3;for(var item of [null]){}return item;",
        "const f=()=>item;var item=3;for(var item of [null]){}return f();",
        "for(var item of [a,b]){}var item;return item;",
        "const before=item;for(var item of []){}return before;",
        "const o={v:a};for(var item of [o]){}return item===o?1:0;",
        "const before=x;var x=a;return before;",
        "const before=x===undefined?1:0;var x=a;return before;",
        "x=a;var x;return x;",
        "var x=a,copy=x,y=b;return copy;",
        "var x=a;const before=x;x=b;var x;return before;",
        "var x=1;var x;return x;",
        "var x=1;var x=2;return x;",
        "var x=a;const get=()=>x;var x=b;return get();",
        "const get=()=>x;var x=3;return get();",
        "const get=()=>x;const before=get();var x=3;return before===undefined?get():0;",
        "if(a){var x=b;}return x;",
        "var x=a;{var x=b;}return x;",
        "var x=a;{let x=b;}return x;",
        "var a;return a;",
        "var a=b;return a;",
        "var a=null;return a;",
        "var undefined=3;var x;return x;",
        "var x=null;return x;",
        "var x=true;return x;",
        "var x=-0;return x;",
        "var x;x++;return x;",
        "var x=null;var old=x++;return old;",
        "var x=false;return ++x;",
        "const get=()=>x;var x=null;x++;return typeof get()==='number'?1:0;",
        "const get=()=>x;var x=null;x++;return get()??9;",
        "const change=()=>{x=null;};var x=3;change();return x;",
        "var x=3;({x}={});return x;",
        "var x;for(x of [a,b]){}return x;",
        "var x=a;var result=x+=(x=b,2);return result;",
        "for(var i=0;i<0;i++){}return i;",
        "for(var i=0;i<3;i++){}return i;",
        "var x=4;for(var x=0;x<3;x++){}return x;",
        "for(let i=0;i<2;i++){var x=i;}return x;",
        "do{var x=a;break;}while(1);return x;",
        "switch(a){case 0:var x=b;break;default:var x=3;}return x;",
        "try{var x=a;}finally{var x=b;}return x;",
        "try{throw a;}catch(e){var x=e;}return typeof x==='number'?1:0;",
        "var s='abc';var s;return s.length;",
        r"var s='\u{1f600}\ud800';return s.length+s.charCodeAt(2);",
        "var o={v:a};var alias=o;var o;alias.v=b;return o.v;",
        "var xs=[a,b];var same=xs;xs[0]=3;return same[0];",
        "const f={a:()=>a?i:0,b:()=>a?i:0,c:()=>a?i:0};for(var i=0;i<3;i++){if(i===0)f.a=()=>i;else if(i===1)f.b=()=>i;else f.c=()=>i;}return f.a()+f.b()+f.c();",
    ].into_iter().map(|body|format!("export function run(a,b){{{body}}}")).collect());
}

#[test]
fn var_activations_and_parameter_aliases_stay_independent() {
    compare_vars([
        "export function run(a,b) {var x; return a;}",
        "function f(x){var n=0;n++;return +n+x;}export function run(a,b){return f(a)+f(b);}",
        "function f(n){var x=0;if(n<=0)return x;x++;return x+f(n-1);}export function run(a,b){return f(3);}",
        "function f(x){var x;return x;}export function run(a,b){return f(a);}",
        "function f(x=3){var [x]=[null];return x;}export function run(a,b){return f();}",
        "export function run(a,b){const o={f:(x=3)=>{var [x]=[null];return x;}};return o.f();}",
        "function f(x){for(var x in {key:1}){}return x;}export function run(a,b){return f(a)==='key'?1:0;}",
        "function f(x=3){for(var x in {key:1}){}return x;}export function run(a,b){return f()==='key'?1:0;}",
        "export function run(a,b){const f=(x=3)=>{for(var x in {key:1}){}return x;};return f()==='key'?1:0;}",
        "export function run(a,b){const o={f:(x=3)=>{for(var x in {key:1}){}return x;}};return o.f()==='key'?1:0;}",
        "function f(x){const get=()=>x;for(var x of [null]){}return get();}export function run(a,b){return f(a);}",
        "function f(x=3){for(var x of [null]){}return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=3)=>{for(var x of [null]){}return x;};return f();}",
        "export function run(a,b){const o={f:(x=3)=>{for(var x of [null]){}return x;}};return o.f();}",
        "function f(x=3){var x;return x;}export function run(a,b){return f();}",
        "function f(x=3){var x=4;return x;}export function run(a,b){return f();}",
        "function f(x='abc'){var x;return x.length;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=3)=>{var x;return x;};return f();}",
        "export function run(a,b){const o={f:(x=3)=>{var x;return x;}};return o.f();}",
    ].into_iter().map(str::to_owned).collect());
}
