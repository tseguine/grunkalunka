#!/usr/bin/env python3
"""Freeze declaration migration inputs; expectations come from the old compiler."""
import json
from pathlib import Path

ROOT = Path('/workspace/scratch/88caffc5ffb1')
DEST = ROOT / 'declaration-probes'
CASES = []

def add(name, body, profile='numeric', languages=('js', 'ts')):
    for lang in languages:
        case_id = name + '_' + lang
        source = ('export async function run(a,b){' if profile == 'async' else
                  'export function run(a,b){') + body + '}'
        path = DEST / (case_id + '.' + lang)
        CASES.append({'id': case_id, 'profile': profile, 'language': lang,
                      'body': body, 'source': source, 'path': str(path)})

for name, body in [
    ('const_numeric', 'const x=a+b;return x;'),
    ('const_scalar_block', 'const step=x=>{return x+a;};const n=step(b);return n;'),
    ('const_multi_block', 'const step=x=>{return x+a;};const x=step(b),y=step(x);return x+y;'),
    ('let_multi_block', 'const step=x=>{return x+a;};let x=step(b),y=step(x);return x+y;'),
    ('const_predicate', 'const x=a>b;return x?a:b;'),
    ('const_predicate_alias', 'const x=a>b,y=x;return y?a:b;'),
    ('const_predicate_block', 'const step=x=>{return x+a;};const x=step(b)>a,y=x;return y?a:b;'),
    ('const_general_null', 'const x=a?null:b;return +x;'),
    ('const_general_string', 'const x=a?"x":b,y=x;return y?a:b;'),
    ('let_multi_general', 'let x=a?null:b,y=x;x=a;return +y+x;'),
    ('let_implicit', 'let x,y=a;return +(x??y);'),
    ('let_shadow_undefined', 'const undefined=b;let x;return +(x??a);'),
    ('const_publish', 'const x=a,y=x+b,z=y+x;return z;'),
    ('let_publish_mutation', 'let x=a,y=(x=b,x),z=(x=a,y);return x+y+z;'),
    ('const_effect_order', 'let n=0;const x=(n=n*10+1,a),y=(n=n*10+2,x+b);return x+y+n;'),
    ('const_metadata_function', 'const f=x=>x+a;return f(b);'),
    ('const_metadata_alias', 'const f=x=>x+a,g=f;return g(b);'),
    ('const_metadata_multi', 'const x=a,f=y=>x+y,g=f;return g(b);'),
    ('let_function', 'let f=x=>x+a;return f(b);'),
    ('let_function_multi', 'let f=x=>x+a,g=f;f=x=>x-b;return g(a)+f(b);'),
    ('let_selected_function', 'let f=a?(x=>x+a):(x=>x-b);return f(b);'),
    ('const_selected_function', 'const f=a?(x=>x+a):(x=>x-b);return f(b);'),
    ('const_sequence_function', 'let n=0;const f=(n=n*10+1,x=>x+n);return f(a)+n;'),
    ('const_block_factory', 'const make=x=>{return y=>x+y;};const f=make(a);return f(b);'),
    ('const_concise_factory', 'const make=x=>y=>x+y;const f=make(a);return f(b);'),
    ('const_runtime_factory', 'let make=x=>()=>x;const f=make(a);return f();'),
    ('const_factory_call', 'const make=x=>{return y=>x+y;};const f=make.call(null,a);return f(b);'),
    ('const_identity_alias', 'const f=()=>a,g=f;return f===g?a:b;'),
    ('const_capture_mutable', 'let x=a;const f=()=>x,y=(x=b);return f()+y;'),
    ('let_capture_multi', 'let x=a,f=()=>x,y=(x=b,f());return y+x;'),
    ('const_capture_shadow', 'const x=a,f=()=>x;{const x=b;return f()+x;}'),
    ('unused_invalid_body', 'const f=()=>Math.noSuch(a);return b;'),
    ('unused_invalid_block', 'const f=()=>{return Math.noSuch(a);};return b;'),
    ('unused_capture_before_decl', 'const f=()=>x,x=a;return b;'),
    ('called_capture_before_decl', 'const f=()=>x,x=a;return f();'),
    ('tdz_self', 'let x=x;return x;'),
    ('tdz_forward', 'const x=y,y=a;return x;'),
    ('tdz_shadow', 'const x=a;{const x=x;return x;}'),
    ('const_write_later', 'const x=a,y=(x=b);return y;'),
    ('const_initial_error', 'const x=Math.noSuch(a),y=Math.noOther(b);return x+y;'),
    ('const_error_before_pattern', 'const x=Math.noSuch(a),[y]=[b];return y;'),
    ('pattern_before_error', 'const [x]=Math.noSuch(a);return x;'),
    ('var_before_error', 'var x=Math.noSuch(a);return x;'),
    ('missing_call_before_later_error', 'const f=(x,y)=>x+y;const x=f(a),y=Math.noSuch(b);return x+y;'),
    ('nested_body_function_decl', 'const step=x=>{let f=y=>x+y;return f(b);};const value=step(a);return value;'),
    ('nested_body_factory_decl', 'const seed=x=>{return y=>x+y;};const step=x=>{const cb=seed(x);return cb(b);};const value=step(a);return value;'),
    ('nested_factory_local_return', 'const make=x=>{const f=y=>x+y;return f;};const f=make(a);return f(b);'),
    ('managed_object', 'const o={x:a},v=o.x;return v===a?a:b;'),
    ('managed_array', 'const xs=[a,b],v=xs[0];return v===a?a:b;'),
    ('managed_capture_mutation', 'const o={x:a},f=()=>o.x===a?a:b;o.x=b;return f();'),
    ('managed_alias_snapshot', 'let o={x:a},p=o;o={x:b};return p.x===a?a:b;'),
    ('managed_union', 'const x=a?{v:b}:[b];return x===x?a:b;'),
    ('managed_multi_effect', 'let n=0;const o={x:(n=n*10+1,a)},p={x:(n=n*10+2,b)};return (o===p?a:b)+n;'),
    ('for_multi_decl', 'const step=x=>{return x+a;};let n=0;for(let x=step(a),y=x+b;x<b;x++){n+=y;}return n;'),
    ('for_function_decl', 'for(let f=()=>a,n=0;n<1;n++){return f();}return b;'),
    ('for_const_decl', 'let n=0;for(const x=a;n<1;n++){n+=x;}return n;'),
    ('for_tdz_decl', 'for(let x=x;x<b;x++){}return a;'),
    ('for_capture_decl', 'let f=()=>a;for(let x=0;x<2;x++){f=()=>x;}return f();'),
]:
    add(name, body)

for name, body in [
    ('fulfilled', 'const p=Promise.resolve(a);return await p;'),
    ('pending_multi', 'const f=async x=>{await x;return x+b;};const p=f(a),q=f(b);return(await p)+(await q);'),
    ('await_initializer', 'const f=async x=>{await x;return x+b;};let p=f(a),x=await p;return x;'),
    ('mixed_mutable', 'let v=a?Promise.resolve(b):a;v=Promise.resolve(a);return await v;'),
    ('mixed_implicit', 'let v;v=a?Promise.resolve(b):a;return await v;'),
    ('mixed_const', 'const p=Promise.resolve(a),v=b?p:a;return await v;'),
    ('promise_effect_order', 'let n=0;const f=async x=>{n=n*10+x;await x;return n;};const p=f(1),q=(n=n*10+2,f(3));return(await p)+(await q)+n;'),
    ('await_publish', 'let x=a,y=await Promise.resolve(x+b);return x+y;'),
    ('promise_capture', 'const p=Promise.resolve(a),f=async()=>await p;return await f();'),
    ('reject_initializer', 'const p=Promise.reject(a);return await p;'),
    ('promise_tdz', 'const p=q,q=Promise.resolve(a);return await p;'),
    ('await_function_value', 'const seed=async x=>{await x;return x+a;};let f=async y=>await seed(y);return await f(b);'),
    ('managed_async', 'const o={x:a},f=async()=>o.x===a?a:b;return await f();'),
    ('for_await_decl', 'let n=0;for(let x=await Promise.resolve(a),y=x+b;n<1;n++){return y;}return b;'),
]:
    add(name, body, 'async')

add('heap_object', 'const o={x:a};return o;', 'heap')
add('heap_array', 'const xs=[a,b];return xs;', 'heap')

for n in (124, 126, 127, 128, 129):
    expression = '(' * n + 'a' + ')' * n
    add(f'const_parens_{n}', f'const x={expression};return x;')
    add(f'multi_second_parens_{n}', f'let x=a,y={expression};return x+y;')

for n in (62, 63, 64, 65):
    condition = 'a?0:' * n + 'a'
    add(f'caller64_decl_{n}', 'const step=()=>{return ' + condition + ';};' + '{' * 64 + 'const x=step(),y=x+b;return y;' + '}' * 64)

for n in (127, 128, 129):
    declarations = ','.join(f'x{i}=' + ('a' if i == 0 else f'x{i-1}') for i in range(n))
    add(f'locals_multi_{n}', 'const ' + declarations + f';return x{n-1};')
    separate = ''.join(f'const x{i}=' + ('a' if i == 0 else f'x{i-1}') + ';' for i in range(n))
    add(f'locals_separate_{n}', separate + f'return x{n-1};')

declarations = ','.join(f'x{i}=a' for i in range(128))
add('local_charge_before_error', 'const ' + declarations + ',bad=Math.noSuch(a);return b;')

for n in (254, 255, 256):
    add(f'statement_budget_{n}', ';' * n + 'const x=a;return x;')
    add(f'statement_charge_before_error_{n}', ';' * n + 'const x=Math.noSuch(a);return x;')

for n in (126, 127, 128, 129):
    add(f'block_decl_depth_{n}', '{' * n + 'const x=a;return x;' + '}' * n)

for name, body in [
    ('ts_scalar_annotation', 'const x:number=a,y:number=x+b;return y;'),
    ('ts_predicate_annotation', 'const x:number=(a>b) as unknown as number;return x?a:b;'),
    ('ts_function_annotation', 'let f:unknown=(x:number)=>x+a;return f(b);'),
    ('ts_erased_initializer', 'const x=((a as number) satisfies number)!;return x;'),
    ('ts_definite', 'let x!:number;return a;'),
    ('ts_declare', 'declare const x:number;return a;'),
]:
    add(name, body, languages=('ts',))

assert len({c['id'] for c in CASES}) == len(CASES)
DEST.mkdir(exist_ok=True)
assert not (DEST / 'matrix.tsv').exists()
for c in CASES:
    Path(c['path']).write_text(c['source'])
(DEST / 'matrix.tsv').write_text(''.join('\t'.join((c['id'], c['profile'], c['language'], 'observe', c['path'])) + '\n' for c in CASES))
(DEST / 'cases.json').write_text(json.dumps(CASES, indent=2) + '\n')
print(json.dumps({'cases': len(CASES), 'manifest': str(DEST / 'matrix.tsv')}))
