// Check frame-owned JSON operations against pristine Node oracles.
import assert from 'node:assert/strict';

export function checkHeapJson(e,cases){
  const units=text=>Array.from({length:text.length},(_,i)=>text.charCodeAt(i));
  const prepare=text=>{
    const data=units(text),ptr=e.gk_json_input_resize(data.length)>>>0;
    assert(ptr>0);assert.equal(e.gk_json_status(),0);
    new Uint16Array(e.memory.buffer,ptr,data.length).set(data);
  };
  const output=()=>{
    const ptr=e.gk_json_output_ptr()>>>0,len=e.gk_json_output_len()>>>0;
    assert.equal(ptr%2,0);assert(len<=65536&&ptr+len*2<=e.memory.buffer.byteLength);
    return Array.from(new Uint16Array(e.memory.buffer,ptr,len));
  };
  const failure=(id,status)=>{
    assert.equal(id,0);assert.equal(e.gk_json_status(),status);assert.deepEqual(output(),[]);
  };
  failure(e.gk_heap_parse(0,0),11);failure(e.gk_heap_stringify(0,0),11);
  let count=0;
  for(const [kind,text,want] of cases){
    const frame=e.gk_heap_enter()>>>0;assert(frame>0);
    prepare(text);const raw=e.gk_heap_string(frame)>>>0;assert(raw>0);
    const parsed=e.gk_heap_parse(frame,raw)>>>0;
    if(kind==='invalid'||kind==='parse-limit'){
      failure(parsed,Number(want));assert(e.gk_json_error_offset()<=text.length);
      assert.equal(e.gk_json_string_units(raw),0);assert.deepEqual(output(),units(text));
      assert.equal(e.gk_heap_abort(),0);
    }else{
      assert(parsed>0);const result=e.gk_heap_stringify(frame,parsed)>>>0;
      if(kind==='output-limit'){
        failure(result,Number(want));assert.equal(e.gk_json_stringify(parsed),7);
        assert.equal(e.gk_heap_abort(),0);
      }else{
        assert.equal(kind,'valid');assert(result>0);assert.equal(e.gk_json_output_len(),0);
        assert.equal(e.gk_heap_leave(frame,result),0);
        assert.equal(e.gk_json_string_units(result),0);assert.deepEqual(output(),units(want));
        assert.equal(e.gk_json_release(result),0);
      }
    }
    count++;
  }
  const frame=e.gk_heap_enter()>>>0;assert(frame>0);
  for(let i=0;i<500;i++){
    prepare('[1,true,null]');const text=e.gk_heap_string(frame)>>>0;
    const parsed=e.gk_heap_parse(frame,text)>>>0;
    const result=e.gk_heap_stringify(frame,parsed)>>>0;
    assert(text>0&&parsed>text&&result>parsed);
    assert.equal(e.gk_json_string_units(result),0);assert.deepEqual(output(),units('[1,true,null]'));
  }
  assert.equal(e.gk_heap_abort(),0);
  const nested=e.gk_heap_enter()>>>0;assert(nested>0);
  prepare('null');const raw=e.gk_heap_string(nested)>>>0;
  assert.equal(e.gk_heap_pin(nested,raw),0);
  const inner=e.gk_heap_enter()>>>0;assert(inner>nested);
  failure(e.gk_heap_parse(nested,raw),11);
  const parsed=e.gk_heap_parse(inner,raw)>>>0;assert(parsed>raw);
  failure(e.gk_heap_parse(inner,parsed),9);
  const result=e.gk_heap_stringify(inner,parsed)>>>0;assert(result>parsed);
  assert.equal(e.gk_heap_leave(inner,result),0);assert.equal(e.gk_heap_leave(nested,result),0);
  assert.equal(e.gk_json_string_units(result),0);assert.deepEqual(output(),units('null'));
  assert.equal(e.gk_json_release(result),0);
  prepare('null');const owners=Array.from({length:64},()=>e.gk_json_string_from_input()>>>0);
  assert(owners.every(id=>id>0));const full=e.gk_heap_enter()>>>0;
  failure(e.gk_heap_parse(full,owners[0]),2);failure(e.gk_heap_stringify(full,owners[0]),2);
  assert.equal(e.gk_json_release(owners[1]),0);
  assert(e.gk_heap_parse(full,owners[0])>0);assert.equal(e.gk_heap_leave(full,0),0);
  for(const id of owners.filter((_,i)=>i!==1))assert.equal(e.gk_json_release(id),0);
  return {cases:count,cycles:500,maxOutputUnits:65536,sourceLowering:false};
}
