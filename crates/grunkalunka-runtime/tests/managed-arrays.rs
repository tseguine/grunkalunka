#[path = "managed/common.rs"]
mod common;

#[test]
fn unit_assignment_steps_keep_index_order_and_per_iteration_captures() {
    for step in ["i+=1", "i=i+1"] {
        let source = format!(
            "export function run(a,b){{const xs=[1,2,3];let n=0;for(let i=0;i<xs.length;{step}){{n=n*10+(+xs[i]);}}return n;}}"
        );
        common::compare_sources(&source, &source);
    }
    for step in ["i-=1", "i=i-1"] {
        let source = format!(
            "export function run(a,b){{const xs=[11,22];let f=()=>0;for(let i=xs.length-1;i>=0;{step}){{if(i===1)f=()=>+xs[i];}}return f();}}"
        );
        common::compare_sources(&source, &source);
    }
}

#[test]
fn reverse_loops_index_the_current_element_and_skip_empty_arrays() {
    for body in [
        "const xs=[1,2,3];let n=0;for(let i=xs.length-1;i>=0;--i)n=n*10+(+xs[i]);return n;",
        "const xs=[1,,3];let n=0;for(let i=xs.length;i-->0;){if(i in xs)n=n*10+(+xs[i]);}return n;",
        "const xs=[];let n=0;for(let i=xs.length-1;i>=0;--i)xs[i]=++n;for(let i=xs.length;i-- > 0;)xs[i]=++n;return n;",
        "const xs=[11,22,33];let last=()=>0;for(let i=xs.length-1;i>-1;i--){last=()=>+xs[i];if(i===0)break;}return last();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn counted_loops_use_the_live_index_for_reads_and_writes() {
    for body in [
        "const xs=[11,,33];let sum=0;for(let i=0;i<xs.length;++i){if(i in xs)sum+=+xs[i];}return sum;",
        "const src=[a,b,7],dst=[];for(let i=0;i<src.length;i++){dst[i]=src[i];}return dst.length===3&&dst[1]===b?1:0;",
        "const xs=[11,22,33];let sum=0;for(let i=0;i<xs.length;i++){sum+=+xs[i];xs.pop();}return sum;",
        "const xs=[];for(let i=4294967294;i<4294967295;i++){xs[i]=a;}return xs.length;",
        "const xs=[11,22];let last=()=>0;for(let i=0;i<xs.length;i++){last=()=>+xs[i];}return last();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn unreassigned_let_bindings_keep_index_and_length_facts() {
    for body in [
        "let xs=[11,22];let i=a&1;const read=()=>+xs[i];return read();",
        "let xs=[a,b],sum=0;for(let i=0;i<xs.length;i++){sum+=+xs[i];}return sum;",
        "let xs=[a];let saved=xs.length;xs.push(b);return Object.hasOwn(xs,saved)&&!Object.hasOwn(xs,xs.length)?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn length_indices_observe_mutation_and_count_utf16_units() {
    for body in [
        "const xs=[a];xs[xs.length]=b;return xs.length===2&&xs[1]===b?1:0;",
        "const xs=[a],alias=xs;const old=alias.length;xs.push(b);const o={[xs.length]:b};return old in o?0:Object.hasOwn(o,alias['length'])?1:0;",
        "const s='💠';const xs=[11,22,33];return +(xs[s.length]);",
        "const xs=[a];xs[xs.length]=(xs.length=0,b);return xs.length===2&&!Object.hasOwn(xs,0)&&xs[1]===b?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn const_indices_snapshot_values_across_array_and_record_operations() {
    for body in [
        "let n=0;const i=((n=n+1,a)&3),j=i;const xs=[];xs[j]=b;return n*10+xs.length+(Object.hasOwn(xs,i)?10:0);",
        "const i=a&1;const o={[i]:b};const {[i]:x,...rest}=o;return Object.keys(rest).length===0&&x===b?1:0;",
        "const i=4294967295;const xs=[a];xs[i]=b;return xs.length===1&&xs[i]===b&&delete xs[i]&&!(i in xs)?1:0;",
        "const i=a&1;const xs=b?[11,22]:null;return xs?.[i]===22?1:0;",
        "const i=a&1;const xs=[11,22];const read=()=>xs[i];return read()===11?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn masked_indices_keep_key_effects_and_sparse_array_semantics() {
    for body in [
        "const xs=[11,,33,44];return Object.hasOwn(xs,a&3)?+(xs[a&3]):-1;",
        "const xs=[];let n=0;xs[(n=n+1,a)&3]=(n=n+1,b);return n*10+xs.length;",
        "const o={[3&a]:b};const {[a&3]:x,...rest}=o;return x===b&&Object.keys(rest).length===0?1:0;",
        "const xs=[a,b];delete xs[1&a];return ((a&1) in xs)?-1:xs.length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn uint32_delete_keys_preserve_properties_boundaries_and_effects() {
    for body in [
        "const xs=[a,b];const removed=delete xs[a>>>0];return removed?xs.length*10+(Object.hasOwn(xs,0)?1:0)+(Object.hasOwn(xs,1)?2:0):-1;",
        "const xs=[a,b];delete xs[a>>>b];return (Object.hasOwn(xs,0)?1:0)+(Object.hasOwn(xs,1)?2:0);",
        "const o={0:a,1:b,length:7};delete o[a>>>0];return Object.keys(o).length+(o.length===7?10:0);",
        "const d=Object.create(null);d[0]=a;d[1]=b;delete d[a>>>0];return Object.keys(d).length;",
        "const xs=[a,b],alias=xs;delete alias[0>>>0];return !Object.hasOwn(xs,0)&&Object.hasOwn(xs,1)&&xs.length===2?1:0;",
        "let xs=[a,b];const saved=xs;let n=0;const removed=delete (n=1,xs)[(n=n*10+2,xs=[b],0>>>0)];return removed&&n===12&&!Object.hasOwn(saved,0)&&Object.hasOwn(xs,0)?1:0;",
        "const xs=[a,b];delete xs[a?(0>>>0):(1>>>0)];return (Object.hasOwn(xs,0)?1:0)+(Object.hasOwn(xs,1)?2:0);",
        "const xs=[a,b];let key=0;delete xs[key=a>>>0];return Object.hasOwn(xs,0)?1:0;",
        "const xs=[a];xs[4294967295]=b;delete xs[(-1)>>>0];return xs.length===1&&!Object.hasOwn(xs,4294967295)&&Object.hasOwn(xs,0)?1:0;",
        "const xs=[a];xs[4294967294]=b;delete xs[(-2)>>>0];return xs.length===4294967295&&!Object.hasOwn(xs,4294967294)?1:0;",
        "const xs=[a,b];const erase=()=>{delete xs[a>>>0];return xs.length;};return erase();",
        "const xs=[a];let n=0;delete xs[((n=1,a)>>>((n=n*10+2,b)))];return n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn mixed_literal_spreads_preserve_holes_order_identity_and_element_proofs() {
    for body in [
        "const x=[a,...[b]];return a;",
        "const xs=[a,...[b,3],4];return xs[0]+xs[1]+xs[2]+xs[3];",
        "const xs=[a,...[],b];return xs.length===2&&xs[0]===a&&xs[1]===b?1:0;",
        "const xs=[,...[,undefined,a],,b];return xs.length===6&&!Object.hasOwn(xs,0)&&Object.hasOwn(xs,1)&&xs[1]===undefined&&Object.hasOwn(xs,2)&&!Object.hasOwn(xs,4)&&xs[5]===b?1:0;",
        "const xs=[a,...[...[b],3],4];return xs[2];",
        "const xs=[a,...[b,b],x=>x+1];return xs[3](a);",
        "const xs=[a,...['x',' y '],b];return xs[2].trim().length;",
        "const child={v:a};const xs=[b,...[child,child],a];xs[1].v=b;return xs[1]===xs[2]&&child.v===b?1:0;",
        "let n=0;const xs=[(n=1,a),...[(n=n*10+2,b),(n=n*10+3,a)],(n=n*10+4,b)];return n;",
        "const xs=[a,...[b,...[]],...[3],4];return xs.length*10+xs[2];",
        "const xs=[a,...[,],b];const [x,y=9,z]=xs;return y+xs.length;",
        "const xs=[,...[,undefined]];return Object.keys(xs).length;",
        "const xs=[a,...[b]];const ys=[a,...[b]];return xs!==ys?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn array_push_appends_after_argument_effects_and_returns_length() {
    for source in [
        "export function run(a,b){const xs=[];return xs.push()===0&&xs.length===0?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push(b)===2&&xs.length===2&&xs[1]===b?1:0;}",
        "export function run(a,b){const xs=[a,,];return xs.push(b)===3&&xs.length===3&&!Object.hasOwn(xs,1)&&xs[2]===b?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push(b,a)===3&&xs[1]===b&&xs[2]===a?1:0;}",
        "export function run(a,b){const xs=[a];let n=0;const length=xs.push((n=1,b),(n=n*10+2,a));return length===3&&n===12&&xs[1]===b&&xs[2]===a?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push((xs.length=0,b))===1&&xs[0]===b?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push(xs.push(b))===3&&xs[1]===b&&xs[2]===2?1:0;}",
        "export function run(a,b){const xs=[a];xs.push(b);return xs.pop()===b&&xs.length===1?1:0;}",
        "export function run(a,b){const xs=[a];const alias=xs;alias.push(b);return xs.length===2&&xs===alias?1:0;}",
        "export function run(a,b){const xs=[a];xs.push('text',null,false,undefined);return xs.length===5&&xs[1]==='text'&&xs[2]===null&&xs[3]===false&&xs[4]===undefined?1:0;}",
        "export function run(a,b){const xs=[a];return xs['push'](b)===2&&xs.length===2?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push(...[b,a])===3&&xs[1]===b&&xs[2]===a?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push(...[...[b],a])===3&&xs.length===3?1:0;}",
        "export function run(a,b){const xs=[a];return xs.push(...[])===1&&xs.length===1?1:0;}",
        "export function run(a,b){const xs=[a];let n=0;const len=xs.push(...[(n=1,b),(n=n*10+2,a)],n=n*10+3);return len===4&&n===123&&xs[1]===b&&xs[2]===a&&xs[3]===123?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_unshift_prepends_after_arguments_and_preserves_sparse_slots() {
    for source in [
        "export function run(a,b){const xs=[];return xs.unshift()===0&&xs.length===0?1:0;}",
        "export function run(a,b){const xs=[a];return xs.unshift()===1&&xs[0]===a?1:0;}",
        "export function run(a,b){const xs=[a,b];return xs.unshift(b,a)===4&&xs[0]===b&&xs[1]===a&&xs[2]===a&&xs[3]===b?1:0;}",
        "export function run(a,b){const xs=[,a,,b];const n=xs.unshift(b);return n===5&&xs[0]===b&&!Object.hasOwn(xs,1)&&xs[2]===a&&!Object.hasOwn(xs,3)&&xs[4]===b?1:0;}",
        "export function run(a,b){const xs=[a];let n=0;const count=xs.unshift((n=1,b),(n=n*10+2,a));return count===3&&n===12&&xs[0]===b&&xs[1]===a?1:0;}",
        "export function run(a,b){const xs=[a];const alias=xs;const count=xs.unshift(b);return count===2&&xs===alias&&alias[0]===b&&alias[1]===a?1:0;}",
        "export function run(a,b){const xs=[a];const count=xs.unshift((xs.length=0,b));return count===1&&xs[0]===b?1:0;}",
        "export function run(a,b){const xs=[a];const count=xs.unshift(xs.unshift(b));return count===3&&xs[0]===2&&xs[1]===b&&xs[2]===a?1:0;}",
        "export function run(a,b){const xs=[a];return xs.unshift(...[b,a])===3&&xs[0]===b&&xs[1]===a?1:0;}",
        "export function run(a,b){const xs=[a];return xs.unshift(...[...[b],a])===3&&xs[0]===b&&xs[1]===a?1:0;}",
        "export function run(a,b){const xs=[a];return xs.unshift('x',null,false,undefined)===5&&xs[0]==='x'&&xs[1]===null&&xs[2]===false&&xs[3]===undefined?1:0;}",
        "export function run(a,b){const xs=[a,b];xs.unshift(a);return xs.shift()===a&&xs.shift()===a&&xs.shift()===b&&xs.length===0?1:0;}",
        "export function run(a,b){const xs=[a];xs['01']=b;xs.unshift(b);return xs['01']===b&&xs[0]===b&&xs[1]===a?1:0;}",
        "export function run(a,b){const xs=[a];xs.length=4294967295;return xs.unshift()===4294967295&&xs.length===4294967295?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_pop_removes_current_elements_holes_and_sparse_extremes() {
    for source in [
        "export function run(a,b){const x=[];return x.pop()===undefined&&x.length===0?1:0;}",
        "export function run(a,b){const x=[a,b];const last=x.pop();return last===b&&x.length===1&&x[1]===undefined?1:0;}",
        "export function run(a,b){const x=[a,,];const last=x.pop();return last===undefined&&x.length===1&&x[0]===a?1:0;}",
        "export function run(a,b){const x=[a,undefined];const last=x.pop();const keys=Object.keys(x);return last===undefined&&keys.length===1&&keys.at(0)==='0'?1:0;}",
        "export function run(a,b){const x=[];x.length=4294967295;return x.pop()===undefined&&x.length===4294967294?1:0;}",
        "export function run(a,b){const x=[];x[4294967294]=a;x['01']=b;x[4294967295]='named';const last=x.pop();return last===a&&x.length===4294967294&&x[4294967294]===undefined&&x['01']===b&&x[4294967295]==='named'?1:0;}",
        "export function run(a,b){const x=[a,b];x.pop();x.length=2;return x[1]===undefined&&x.pop()===undefined&&x.pop()===a&&x.pop()===undefined&&x.length===0?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_pop_snapshots_receiver_before_all_surplus_argument_effects() {
    for source in [
        "export function run(a,b){let x=[a,b];const old=x;let n=0;const value=(n++,x).pop((x=[b],n=n*10+2,old[1]=a),(n=n*10+3,old.length=1));return value===a&&old.length===0&&x.length===1&&n===123?1:0;}",
        "export function run(a,b){const x=[a,b];const last=x.pop(x.pop());return last===a&&x.length===0?1:0;}",
        "export function run(a,b){const x=[a,b];let n=0;const last=x.pop((n++,x.length=0),(n++,x[2]=a));return last===a&&n===2&&x.length===2&&x[2]===undefined?1:0;}",
        "export function run(a,b){const x=[a];return x.pop({x:b},[b],'ignored',()=>b)===a&&x.length===0?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_pop_transports_snapshots_through_helpers_and_captures() {
    for source in [
        "function take(x){return x.pop();}export function run(a,b){const o={x:a};const x=[o];const saved=take(x);x[0]=b;return saved===o&&x[0]===b?1:0;}",
        "export function run(a,b){let x=[a,b];const take=()=>x.pop();const first=take();x=[a];return first===b&&take()===a&&x.length===0?1:0;}",
        "export function run(a,b){const f=()=>a;const child=[b];const o={x:a};const x=[o,child,f,'text',null,false,undefined];const u=x.pop();const n=x.pop();const nil=x.pop();const text=x.pop();const fn=x.pop();const array=x.pop();const obj=x.pop();return u===undefined&&n===false&&nil===null&&text==='text'&&fn===f&&array===child&&obj===o&&x.length===0?1:0;}",
        "export function run(a,b){const x=[];x[0]=x;return x.pop()===x&&x.length===0?1:0;}",
        "export function run(a,b){const x=[[a,b]];return x[0].pop()===b&&x[0].length===1?1:0;}",
        "export function run(a,b){const x=[a,b];let n=0;while(x.length){if(x.pop()===a)n++;}return n;}",
        "export function run(a,b){const x=[a];x['01']='named';x.pop();return x['01']==='named'?1:0;}",
        "export function run(a,b){const o={pop:()=>a};return o.pop();}",
    ] {
        common::compare_sources(source, source);
    }
    common::compare_sources(
        "export function run(a,b){const x=[a,b];return x['pop']()===b&&x.length===1?1:0;}",
        "export function run(a:string,b:boolean):void{const x:number=[a,b];return (x as string)[('pop' as string)]()===b&&x.length===1?1:0;}",
    );
}

#[test]
fn array_pop_invalidates_indexed_receiver_string_and_callable_proofs() {
    for source in [
        "export function run(a,b){const x=['text'];x.pop();return x[0].length;}",
        "export function run(a,b){const x=['text'];const alias=x;alias.pop();return x[0].length;}",
        "function take(x){x.pop();return 0;}export function run(a,b){const x=['text'];take(x);return x[0].length;}",
        "export function run(a,b){const o={items:['text']};o.items.pop();return o.items[0].length;}",
        "export function run(a,b){const x=['text'];const take=()=>x.pop();take();return x[0].length;}",
        "export function run(a,b){const x=[()=>a];x.pop();return x[0]();}",
        "export function run(a,b){const x=[[a]];x.pop();return x[0].length;}",
        "export function run(a,b){const x=[{value:a}];x.pop();return x[0].value===a?1:0;}",
        "export function run(a,b){const x=['text'];return x.pop().length;}",
        "export function run(a,b){const x=['text'];return (x.pop() as string).length;}",
    ] {
        let error = grunkalunka_compiler::compile(source, true)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn array_pop_keeps_unknown_receivers_overrides_and_borrowing_explicit() {
    for body in [
        "return a.pop();",
        "const x=a?[a]:null;return x.pop();",
        "const x=[a];x.pop=()=>b;return x.pop();",
        "const x=[a];x['pop']=()=>b;return x.pop();",
        "const x=[a];return x.pop.call(x);",
        "const x=[a];return x.pop?.();",
        "const x=[a];return x?.pop();",
        "const x=[a];return x.pop(...[b]);",
        "const x=[a];return x.pop();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = grunkalunka_compiler::compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn array_pop_argument_cursors_keep_native_stack_and_count_bounds() {
    for (count, admitted, nested) in [
        (64, true, true),
        (140, false, true),
        (1000, true, false),
        (1001, false, false),
    ] {
        let expression = if nested {
            format!("{}0{}", "x.pop(".repeat(count), ")".repeat(count))
        } else {
            format!("x.pop({})", vec!["a"; count].join(","))
        };
        let source =
            format!("export function run(a,b){{const x=[a,b];{expression};return x.length;}}");
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "count={count}, nested={nested}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            if nested {
                // Exhausted shape queries may report the enclosing source
                // rejection before the evaluator's explicit nesting guard.
                assert!(error.contains("unsupported"), "{error}");
                assert!(!error.contains("internal IR"), "{error}");
            } else {
                assert!(error.contains("1000"), "{error}");
            }
        }
    }
}

#[test]
fn array_at_reads_runtime_number_positions_and_sparse_holes() {
    for source in [
        "export function run(a,b){const x=[b,,a];return x.at(a)===b?1:0;}",
        "export function run(a,b){const x=[a,,b];return x.at(1)===undefined&&x.at(-2)===undefined&&x.at(3)===undefined&&x.at(-4)===undefined?1:0;}",
        "export function run(a,b){const x=[a,b];return x.at()===a&&x.at(-0.9)===a&&x.at(1.9)===b&&x.at(-1.9)===b?1:0;}",
        "export function run(a,b){const x=[];return x.at(a)===undefined?1:0;}",
        "export function run(a,b){const x=[];x[4294967294]=a;x['-1']=b;return x.at(-1)===a&&x.at(-4294967295)===undefined&&x.at(4294967295)===undefined?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_at_snapshots_receiver_and_index_before_surplus_argument_effects() {
    for source in [
        "export function run(a,b){let x=[a,b];const old=x;let i=0;const value=x.at((x=[b],i++),(old[0]=b,i++));return value===b&&old!==x&&i===2?1:0;}",
        "export function run(a,b){const x=[a,b];const saved=x.at(-1,x.length=1);return saved===a&&x.length===1?1:0;}",
        "export function run(a,b){let n=0;const x=[a,b];(n++,x).at((n=n*10+2,0),(n=n*10+3,0));return n;}",
        "export function run(a,b){const x=[a,b];const saved=x.at(0);x[0]=b;return saved===a&&x.at(0)===b?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_at_transports_managed_elements_and_composes_with_helpers() {
    for source in [
        "export function run(a,b){const o={x:a};const child=[b];const x=[o,child,'text',null,false,undefined];return x.at(0)===o&&x.at(1)===child&&x.at(2)==='text'&&x.at(3)===null&&x.at(4)===false&&typeof x.at(5)==='undefined'?1:0;}",
        "export function run(a,b){const f=()=>a;const x=[f];return x.at(0)===f?1:0;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){const x=[a,b];return pick(x,1)===b?1:0;}",
        "function pick(x,i){return x.at(i);} function forward(x,i){return pick(x,i);} export function run(a,b){const o={value:a};const x=[o];return forward(x,0)===o&&forward(x,1)===undefined?1:0;}",
        "function pick(x,i){return x['at'](i);} export function run(a,b){const o={value:a};const x=[a,o,b];const choose=()=>pick(x,1);return choose()===o?1:0;}",
        "export function run(a,b){const x=[a,b,a];let n=0;for(let i=0;i<x.length;i++){if(x.at(i)===a)n++;}return n;}",
        "export function run(a,b){const x=Object.keys({...{first:a,second:b}});return x.at(-1)==='second'?1:0;}",
        "export function run(a,b){const o={at:()=>a};return o.at();}",
    ] {
        common::compare_sources(source, source);
    }
    common::compare_sources(
        "export function run(a,b){const x=[a,b];return x['at'](-1)===b?1:0;}",
        "export function run(a:string,b:boolean):void{const x:number=[a,b];return x[('at' as string)]((-1 as number))===b?1:0;}",
    );
}

#[test]
fn array_at_defaults_compose_with_strings_records_copies_and_lazy_effects() {
    for source in [
        "export function run(a,b){const {text=' ? '}={text:[' first ',' second '].at(a)};return text.trim().length;}",
        "function pick(xs,i){return xs.at(i);}function forward(xs,i){return pick(xs,i);}export function run(a,b){const rows=[{label:' first '},{label:' second '}];const read=()=>forward(rows,a);const {item={label:' missing '}}={item:read()};return item.label.trim().length;}",
        "export function run(a,b){const {item=[]}={item:[[a],[a,b]].at(a)};return item.length;}",
        "export function run(a,b){let n=0;const row={label:' first '};let rows=[row,{label:' second '}];const old=rows;const {item=(n++,{label:' missing '})}={item:rows.at((rows=[{label:' new '}],a))};old[0].label=' changed ';return item.label.trim().length*10+n;}",
        "export function run(a,b){const rows=[{label:' first '},{label:' second '}];const [...copy]=rows.slice();const {item={label:' missing '}}={item:copy.at(a)};rows[0].label=' changed ';return item.label.trim().length;}",
        "export function run(a,b){let n=0;const [text=(n++,' ? ')]=[[' first ',' second '].at(a)];return text.trim().length*10+n;}",
        "export function run(a,b){const {key='none'}={key:Object.keys({first:a,second:b}).at(a)};return key.trim().length;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_at_keeps_unsupported_receivers_coercions_and_overrides_explicit() {
    for body in [
        "const x=[a];return x.at('0')===a?1:0;",
        "const x=[a];return x.at(null)===a?1:0;",
        "const x=[a];x.at=()=>b;return x.at(0);",
        "const x=[a];x['at']=()=>b;return x.at(0);",
        "const x=[a];return x.at.call(x,0);",
        "const x=a?[a]:null;return x.at(0);",
        "const x=[a];return x.at(...[0]);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = grunkalunka_compiler::compile(&source, typed).err().unwrap();
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

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
fn numeric_literal_array_properties_only_grow_length_for_array_indices() {
    for body in [
        "const x=[];x[0.5]=a;return a;",
        "const x=[];x[-1]=a;return a;",
        "const x=[];x[4294967296]=a;return a;",
        "const x=[];x[-1]=a;x[0.5]=b;x[4294967295]=a;x[4294967296]=b;const initial=x.length;x[-0]=a;x[4294967294]=b;const before=x.length;x.length=0;const keys=Object.keys(x);return initial===0&&before===4294967295&&x[0]===undefined&&x[-1]===a&&x[0.5]===b&&keys.length===4&&keys[0]==='-1'&&keys[3]==='4294967296'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn unsupported_array_semantics_reject_before_ir_validation() {
    for body in [
        "const x=[a];return x[b]===a?a:b;",
        "const x=[];x.length=0.5;return a;",
        "const x=[];x.length=-1;return a;",
        "const x=[];x.length=4294967296;return a;",
        "const x=[];x.length=a;return a;",
        "const x=[];x.length='2';return a;",
        "const x=[];x.length+=1;return a;",
        "const x=[];x.length++;return a;",
        "const x=[...a];return a;",
        "const x=[...{length:1,0:b}];return a;",
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

#[test]
fn array_is_array_checks_the_saved_value_across_managed_kinds() {
    for body in [
        "const values=[[],{},Object.create(null),'x',null,undefined,a,false,()=>b];let n=0;for(const value of values){n=n*2+(Array.isArray(value)?1:0);}return n;",
        "const options={items:a?[b]:{length:1,0:b}};return Array['isArray'](options.items)?1:0;",
        "let n=0;const x={items:[a]};const test=Array.isArray.call((n=1,null),(n=n*10+2,x.items),(n=n*10+3,x.items={}));return test&&n===123&&!Array.isArray(x.items)?1:0;",
        "let n=0;Array.isArray((n=1,[]),(n=n*10+2,b));return n;",
        "return !Array.isArray()&&!Array.isArray.call([])&&Array.isArray(Object.entries({x:b}))?1:0;",
        "const Array={isArray:x=>x};return Array.isArray(a);",
        "return +Array.isArray(a?[b]:{});",
        "const box={ok:Array.isArray([b])};return box.ok===true?1:0;",
        "const array=[];array[4294967294]=b;array.length=0;return Array.isArray(array)?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    common::compare_sources(
        "export function run(a,b){return Array.isArray(a?[b]:{})?1:0;}",
        "type Array = number; export function run(a:string,b:boolean):void{return Array.isArray((a?[b]:{}) as string)?1:0;}",
    );
}

#[test]
fn array_is_array_rejects_optional_calls_and_spread_arguments() {
    for body in [
        "return Array.isArray(...[[a]])?1:0;",
        "return Array.isArray?.([a])?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&source, ts)
                .err()
                .expect("unsupported call");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn copy_spread_creates_a_fresh_dense_array_and_preserves_child_identity() {
    for body in [
        "const x=[...[a]];return a;",
        "const src=[,a,,undefined];src.named=b;const copy=[...src];let n=0;for(const key of Object.keys(copy))n+=key.length;return n===4&&copy.length===4&&Object.hasOwn(copy,0)&&!Object.hasOwn(src,0)&&!Object.hasOwn(copy,'named')?1:0;",
        "const child={label:' x '};const src=[child];const copy=[...src];src[0]={label:' no '};child.label=' yes ';return src!==copy&&copy[0]===child?copy[0].label.trim().length:0;",
        "const src=[a,b];const copy=[...[...src]];copy.length=0;return src.length*10+copy.length;",
        "let n=0;const src=[a];const copy=[...(n++,src[1]=b,src)];return n===1&&copy[1]===b?1:0;",
        "const src=[];const copy=[...src];return copy!==src&&copy.length===0?1:0;",
        "const src=[{label:' hello '}];let n=0;for(const {label} of [...src])n+=label.trim().length;return n;",
        "const names=Object.keys({expand:a});const copy=[...names];let n=0;for(const name of copy)n+=name.trim().length;return n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn string_copy_spread_collects_code_points_into_independent_dense_slots() {
    for body in [
        r"const chars=[...'A\u{1f600}\ud800x\udc00'];return chars.length===5&&chars[0]==='A'&&chars[1]==='\u{1f600}'&&chars[2]==='\ud800'&&chars[3]==='x'&&chars[4]==='\udc00'?1:0;",
        r"const s='\ud800\ud801\udc01\udc02\udbff\udfff\ud800';const xs=[...s];let i=0;for(const ch of s){if(xs.at(i)!==ch)return -1;i++;}return xs.length===i&&Object.keys(xs).length===i?i:-1;",
        "let n=0;const xs=[...(n++,'')];return n===1&&xs.length===0?1:0;",
        "const label=a?' OK ':'';const chars=[...label];let n=0;for(const ch of chars)n+=ch.trim().length;const [first='empty']=chars;return n+first.trim().length;",
        "const s='xy';const a1=[...s];const a2=[...s];a1[0]='z';return a1!==a2&&a2[0]==='x'&&s==='xy'?1:0;",
        "const [...tail]=[...'ab'];const xs=[...tail];return xs.pop()==='b'&&Object.values(xs)[0]==='a'&&xs.length===1?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn string_copy_spread_rejects_unknown_inputs_and_invalidated_string_proofs() {
    for body in [
        "const s=a?'x':{};return [...s].length;",
        "const s=a?'x':[b];return [...s].length;",
        "const xs=[...''];return xs[0].trim().length;",
        "const xs=[...'x'];xs[0]=a;const [x='default']=xs;return x.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let err = grunkalunka_compiler::compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn array_includes_uses_same_value_zero_sparse_holes_and_numeric_start() {
    for body in [
        "const xs=[a,NaN,-0,null,false,'x'];return xs.includes(b)&&xs.includes(0)&&xs.includes(NaN)?1:0;",
        "const object={n:a};const other={n:a};const fn=()=>b;const xs=[object,fn,['child']];return xs.includes(object)&&!xs.includes(other)&&xs.includes(fn)&&!xs.includes(()=>b)&&!xs.includes(['child'])?1:0;",
        "const xs=[,a];xs.named='named';return xs.includes()&&!xs.includes(undefined,1)&&!xs.includes('named')&&![].includes()?1:0;",
        "const xs=[a,undefined,,b];return xs['includes'](undefined,b)?1:0;",
        "const xs=['x','y','z'];return xs.includes('y',a)?1:0;",
        "const xs=['x','y','z'];return xs.includes('y',-1.9)||!xs.includes('y',-2.9)?0:1;",
        "const xs=[];xs.length=4294967295;xs[4294967294]='z';return xs.includes(undefined)&&xs.includes('z',-1)&&!xs.includes(undefined,-1)&&!xs.includes('missing',4294967295)?1:0;",
        "let n=0;let xs=['x'];const old=xs;const r=xs.includes((n=1,xs=['y'],'z'),(n=n*10+2,0),(n=n*10+3,old[0]='z'));return r&&n===123&&!xs.includes('z')?1:0;",
        "let n=0;const xs=['a'];const r=xs.includes('a',void(n=1),(n=n*10+2));return r?n:0;",
        "const obj={includes:x=>x};return obj.includes(a);",
        "return +[a].includes(b);",
        "let n=0;const xs=['x'];xs.includes('x',(n++,0));return n;",
        "const xs=['x','y'];const read=()=>xs.includes('y');return read()?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn array_includes_requires_a_checked_receiver_and_position() {
    for body in [
        "const xs=a?[b]:null;return xs.includes(b)?1:0;",
        "const xs=[b];return xs.includes(b,{})?1:0;",
        "const xs=[b];return xs.includes(...[b])?1:0;",
        "const xs=[b];xs.includes=()=>a;return xs.includes(b)?1:0;",
        "const xs=[b];return xs.includes?.(b)?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let err = grunkalunka_compiler::compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn array_index_search_uses_numeric_order_skips_holes_and_matches_strictly() {
    for body in [
        "const xs=['abc'];return xs.indexOf('abc');",
        "const xs=[];xs[7]='x';xs[1]='x';xs[4]='x';xs.named='x';return xs.indexOf('x')*10+xs.lastIndexOf('x');",
        "const xs=[,undefined,,undefined];return xs.indexOf()*10+xs.lastIndexOf();",
        "const xs=[,,];return xs.indexOf(undefined)+xs.lastIndexOf(undefined);",
        "const xs=[a,NaN,0,-0,b];return xs.indexOf(NaN)===-1&&xs.lastIndexOf(NaN)===-1?xs.indexOf(b)*10+xs.lastIndexOf(b):-100;",
        "const o={v:a};const xs=[o,{v:b},o];return xs.indexOf({v:a})===-1&&xs.lastIndexOf(o)===2?xs.indexOf(o):99;",
        "const src=['x','y','x','y'];return src.indexOf('y',b)*10+src.lastIndexOf('x',b);",
        "const xs=['y','x','x'];return xs.lastIndexOf('x')*10+xs.lastIndexOf('x',undefined);",
        "const xs=['x','y','x','y'];return xs.indexOf('y',-2.9)*10+xs.lastIndexOf('x',-1.9);",
        "const xs=[];xs[4294967294]='last';return xs.indexOf('last',-1)+xs.lastIndexOf('last');",
        "let n=0;const xs=['x','y'];const i=xs.lastIndexOf('y',void(n++),xs[0]='y');return n*10+i;",
        "const record={indexOf:x=>x,lastIndexOf:x=>x};return record.indexOf(a)+record.lastIndexOf(b);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let source = "function find(list,value){return list.indexOf(value);}export function run(a,b){const xs=['a','b'];const get=()=>xs['lastIndexOf']('b');return find(xs,'b')+get();}";
    common::compare_sources(source, source);
}

#[test]
fn array_slice_preserves_sparse_positions_identity_and_fresh_slots() {
    for body in [
        "const src=[,a,undefined,,b];src.named=a;const xs=src.slice(1,4);return xs!==src&&xs.length===3&&xs[0]===a&&Object.hasOwn(xs,1)&&!Object.hasOwn(xs,2)&&!Object.hasOwn(xs,'named')?1:0;",
        "const child={v:a};const src=[child];const xs=src.slice();src[0]=b;return xs[0]===child&&src[0]===b&&xs.length===1?1:0;",
        "const src=[a,b];const xs=src.slice();xs[0]=b;xs.length=0;return src[0]===a&&src.length===2?1:0;",
        "const src=[];src[7]='x';src[1]='y';src[4]='z';const xs=src.slice(1,8);const keys=Object.keys(xs);return keys[0]==='0'&&keys[1]==='3'&&keys[2]==='6'&&xs[6]==='x'&&xs.length===7?1:0;",
        "const src=[];src[4294967294]='last';src[4294967295]='named';const xs=src.slice(-2);return xs.length===2&&!Object.hasOwn(xs,0)&&xs[1]==='last'&&Object.keys(xs).length===1?1:0;",
        "const src=[];const xs=src.slice();return xs!==src&&xs.length===0?1:0;",
        "const src=[a,b];return src.slice(a,b).length;",
        "const src=['a','b','c','d'];const xs=src.slice(-2.9,-0.5);const ys=src.slice(-3.8,3.9);return xs.length===0&&ys.length===2&&ys[0]==='b'&&ys[1]==='c'?1:0;",
        "const src=[a,b];return src.slice(undefined,undefined).length+src.slice(NaN,NaN).length;",
        "const src=[null,false,()=>a];const xs=src.slice();return xs[0]===null&&xs[1]===false&&xs[2]===src[2]?1:0;",
        "const src=[a,b];const xs=src.slice().slice(1);return xs.length===1&&xs[0]===b?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn array_slice_saves_receiver_endpoints_and_runs_all_argument_effects() {
    for source in [
        "export function run(a,b){let n=0;let src=[a,b,a];const saved=src;let start=1;const xs=src.slice((n=1,start),(n=n*10+2,start=0,src=[],2),(n=n*10+3,saved[1]=a));return n===123&&xs.length===1&&xs[0]===a&&src.length===0?1:0;}",
        "export function run(a,b){const src=[a,b];const xs=src.slice(0,undefined,src.length=1);return xs.length===1&&xs[0]===a?1:0;}",
        "export function run(a,b){let n=0;const xs=[a,b].slice(void(n=1),void(n=n*10+2),(n=n*10+3));return n+xs.length;}",
        "function copy(xs,from){return xs.slice(from);}export function run(a,b){const src=[a,b];const read=()=>copy(src,1);const xs=read();return xs[0]===b&&xs.length===1?1:0;}",
        "export function run(a,b){const obj={slice:x=>x};return obj.slice(a);}",
        "export function run(a,b){const box={items:[a,b].slice(1)};return box.items.length;}",
        "export function run(a,b){let n=0;[a].slice((n++,0));return n;}",
    ] {
        common::compare_sources(source, source);
    }
    common::compare_sources(
        "export function run(a,b){const src=[a,b];return src['slice'](1).length;}",
        "export function run(a:string,b:boolean):void{const src:number=[a,b];return (src as string)[('slice' as string)](1).length;}",
    );
}

#[test]
fn array_slice_copied_values_retain_string_and_record_origins() {
    for source in [
        "export function run(a,b){const xs=[{label:' One '},{label:' Two '}];let n=0;for(const {label} of xs.slice(a,b))n+=label.trim().length;return n;}",
        "function page(xs,start){return xs.slice(start);}export function run(a,b){const xs=[' One ',' Two '];const get=()=>page(xs,a);let n=0;for(const label of get())n+=label.trim().length;return n;}",
        "export function run(a,b){const xs=[' x '];xs.named=a;const copy=xs.slice().slice();let n=0;for(const label of [...copy])n+=label.trim().length;return n;}",
        "export function run(a,b){const xs=[' x '];const copy=xs.slice();copy[0]=a;return xs[0].trim().length;}",
        "export function run(a,b){const xs=[' x '].slice(a);const [name='empty']=xs;return name.trim().length;}",
        "export function run(a,b){const item={label:' x '};const xs=[item].slice();item.label=' changed ';let n=0;for(const {label} of xs)n+=label.trim().length;return n;}",
        "export function run(a,b){const xs=[[' x '],[' y ']];let n=0;for(const [label] of xs.slice(a,b))n+=label.trim().length;return n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_slice_value_proofs_reject_holes_and_incompatible_alias_writes() {
    for body in [
        "const xs=['x'];const alias=xs;alias[0]=a;for(const s of xs.slice())s.trim();return a;",
        "const copy=['x'].slice();copy[0]=a;for(const s of copy)s.trim();return a;",
        "const xs=['x'];xs.length=2;for(const s of xs.slice())s.trim();return a;",
        "const xs=[,'x'];for(const s of xs.slice())s.trim();return a;",
        "const xs=[{label:'x'}];const copy=xs.slice();for(const child of copy)child.label=a;return xs[0].label.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let err = grunkalunka_compiler::compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn array_reverse_reorders_sparse_slots_and_returns_the_original_identity() {
    for body in [
        "const xs=[a,b];const alias=xs.reverse();return alias===xs&&xs[0]===b&&xs[1]===a?1:0;",
        "const xs=[,a,undefined,,b];xs.named='keep';const r=xs.reverse();return r===xs&&xs.length===5&&xs[0]===b&&!Object.hasOwn(xs,1)&&Object.hasOwn(xs,2)&&xs[3]===a&&!Object.hasOwn(xs,4)&&xs.named==='keep'?1:0;",
        "const xs=[];xs[8]='a';xs[2]='b';xs[4]='c';const r=xs.reverse();const keys=Object.keys(r);return keys[0]==='0'&&keys[1]==='4'&&keys[2]==='6'&&xs[0]==='a'&&xs[4]==='c'&&xs[6]==='b'?1:0;",
        "const xs=[];xs.named=a;return xs.reverse()===xs&&xs.length===0&&xs.named===a?1:0;",
        "const xs=[,];return xs.reverse().length===1&&!Object.hasOwn(xs,0)?1:0;",
        "const child={v:a};const fn=()=>b;const xs=[child,fn];xs['reverse']();return xs[0]===fn&&xs[1]===child?1:0;",
        "const xs=[a,,b];xs.reverse().reverse();return xs[0]===a&&xs[2]===b&&!Object.hasOwn(xs,1)?1:0;",
        "const xs=[a,b];const alias=xs.reverse();alias[0]=a;return xs[0]===a&&xs[1]===a?1:0;",
        "const xs=['a','b','a'];return xs.reverse().indexOf('b');",
        "const xs=[];xs.length=10000;xs[1]='x';xs.reverse();return xs[9998]==='x'&&!Object.hasOwn(xs,1)&&xs.length===10000?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn array_reverse_saves_receiver_before_argument_effects_helpers_and_captures() {
    for source in [
        "export function run(a,b){let n=0;let xs=[a,b];const old=xs;const result=xs.reverse((n=1,xs=[]),(n=n*10+2,old[0]=b),(n=n*10+3,old.length=1));return result===old&&result.length===1&&old[0]===b&&xs.length===0?n:0;}",
        "function reverse(xs){return xs.reverse();}export function run(a,b){const xs=[a,b];const f=()=>reverse(xs);return f()===xs&&xs[0]===b?1:0;}",
        "export function run(a,b){const object={reverse:x=>x};return object.reverse(a);}",
        "export function run(a,b){const xs=[a,b];return xs.reverse(xs.pop()).length;}",
        "export function run(a,b){const xs=[a,b];const copy=xs.slice().reverse();return copy[0]===b&&xs[0]===a?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_shift_removes_first_value_and_moves_sparse_indices() {
    for body in [
        "const xs=[a,b];const first=xs.shift();return first===a&&xs.length===1&&xs[0]===b&&!Object.hasOwn(xs,1)?1:0;",
        "const xs=[,a,,undefined,b];const first=xs.shift();return first===undefined&&xs.length===4&&xs[0]===a&&!Object.hasOwn(xs,1)&&Object.hasOwn(xs,2)&&xs[3]===b?1:0;",
        "const xs=[];xs.named=a;return xs.shift()===undefined&&xs.length===0&&xs.named===a?1:0;",
        "const xs=[undefined];const first=xs.shift();return first===undefined&&xs.length===0&&!Object.hasOwn(xs,0)?1:0;",
        "const xs=[,];const first=xs.shift();return first===undefined&&xs.length===0?1:0;",
        "const xs=[];xs[7]='a';xs[2]='b';xs[0]='first';xs.named='keep';const first=xs.shift();const keys=Object.keys(xs);return first==='first'&&xs.length===7&&xs[1]==='b'&&xs[6]==='a'&&keys[0]==='1'&&keys[1]==='6'&&keys[2]==='named'&&xs.named==='keep'?1:0;",
        "const child={v:a};const fn=()=>b;const xs=[child,fn,null,false];return xs.shift()===child&&xs.shift()===fn&&xs.shift()===null&&xs.shift()===false&&xs.length===0?1:0;",
        "const xs=[];xs.length=10000;xs[9999]='last';return xs.shift()===undefined&&xs.length===9999&&xs[9998]==='last'&&!Object.hasOwn(xs,9999)?1:0;",
        "const xs=[a];xs['01']=b;xs[4294967295]='named';xs.shift();return xs['01']===b&&xs[4294967295]==='named'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn array_shift_snapshots_receiver_and_runs_all_argument_effects() {
    for source in [
        "export function run(a,b){let xs=[a,b];const old=xs;let n=0;const first=xs.shift((n=1,xs=[]),(n=n*10+2,old[0]=b),(n=n*10+3,old.length=1));return first===b&&old.length===0&&xs.length===0?n:0;}",
        "export function run(a,b){const xs=[a,b];return xs.shift(xs.shift())===b&&xs.length===0?1:0;}",
        "function take(xs){return xs['shift']();}export function run(a,b){const xs=[a,b];const f=()=>take(xs);return f()===a&&xs[0]===b?1:0;}",
        "export function run(a,b){const xs=[a,b];const take=()=>xs.shift();return take()===a&&take()===b?1:0;}",
        "export function run(a,b){const obj={shift:x=>x};return obj.shift(a);}",
        "export function run(a,b){const xs=[];xs[0]=xs;return xs.shift()===xs&&xs.length===0?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn reversed_iteration_retains_record_and_string_origins() {
    for source in [
        "export function run(a,b){const xs=[{label:' One '},{label:' Two '}];let s='';for(const {label} of xs.reverse())s=s+label.trim();return s==='TwoOne'?1:0;}",
        "export function run(a,b){const xs=[' A ',' BB ',' CCC '];xs.reverse();const [,...tail]=xs;let n=0;for(const s of tail)n=n*10+s.trim().length;return n;}",
        "function flip(xs){return xs.reverse();}export function run(a,b){const xs=[' x ',' yy '];const f=()=>flip(xs);let n=0;for(const s of f())n=n*10+s.trim().length;return n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn to_reversed_copies_dense_values_and_preserves_source_slots() {
    for source in [
        "export function run(a,b){const xs=[a,b];const ys=xs.toReversed();return ys!==xs&&ys[0]===b&&ys[1]===a&&xs[0]===a?1:0;}",
        "export function run(a,b){const xs=[,a,,undefined];xs.named=b;const ys=xs.toReversed();return ys.length===4&&Object.keys(ys).length===4&&Object.hasOwn(ys,0)&&Object.hasOwn(ys,1)&&Object.hasOwn(ys,3)&&ys[2]===a&&!Object.hasOwn(ys,'named')&&!Object.hasOwn(xs,0)?1:0;}",
        "export function run(a,b){const child={label:' x '};const xs=[child];const ys=xs.toReversed();ys[0]=a;return xs[0]===child?xs[0].label.trim().length:0;}",
        "export function run(a,b){const xs=[{label:' One '},{label:' Two '}];let s='';for(const {label} of xs.toReversed())s=s+label.trim();return s==='TwoOne'?1:0;}",
        "export function run(a,b){let xs=[a,b];const old=xs;let n=0;const ys=xs.toReversed((n=1,xs=[]),(n=n*10+2,old[1]=a));return ys[0]===a&&old.length===2&&xs.length===0?n:0;}",
        "function copy(xs){return xs['toReversed']();}export function run(a,b){const xs=[' x ',' yy '];const get=()=>copy(xs);let n=0;for(const v of get())n=n*10+v.trim().length;return n;}",
        "export function run(a,b){const xs=[];const ys=xs.toReversed();return xs!==ys&&ys.length===0?1:0;}",
        "export function run(a,b){const obj={toReversed:x=>x};return obj.toReversed(a);}",
    ] {
        common::compare_sources(source, source);
    }
}
