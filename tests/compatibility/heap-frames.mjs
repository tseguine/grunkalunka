// Shared checks for the actual standalone and composed guest artifacts.
import assert from 'node:assert/strict';

export function checkHeapFrames(e) {
  const prepare=units=>{
    const ptr=e.gk_json_input_resize(units.length)>>>0;
    assert.notEqual(ptr,0);assert.equal(e.gk_json_status(),0);
    new Uint16Array(e.memory.buffer,ptr,units.length).set(units);
  };
  const output=()=>{
    const ptr=e.gk_json_output_ptr()>>>0,len=e.gk_json_output_len()>>>0;
    assert.equal(ptr%2,0);assert(len<=65536);
    assert(ptr+len*2<=e.memory.buffer.byteLength);
    return Array.from(new Uint16Array(e.memory.buffer,ptr,len));
  };
  const status=(actual,want=0)=>{
    assert.equal(actual,want);assert.equal(e.gk_json_status(),want);
    assert.equal(e.gk_json_error_offset(),0);
    assert.deepEqual(output(),[]);
  };
  const raw=(id,want)=>{
    assert.equal(e.gk_json_string_units(id),0);assert.deepEqual(output(),want);
  };
  status(e.gk_heap_clear(0),11);
  status(e.gk_heap_pin(0,0),11);
  status(e.gk_heap_leave(0,0),11);
  assert.equal(e.gk_heap_string(0),0);status(e.gk_json_status(),11);
  status(e.gk_heap_abort());
  prepare([72]);const host=e.gk_json_string_from_input()>>>0;assert(host>0);
  const outer=e.gk_heap_enter()>>>0;assert(outer>0);status(e.gk_json_status());
  prepare([0xd800,0,0xdc00]);const parent=e.gk_heap_string(outer)>>>0;assert(parent>host);
  status(e.gk_heap_pin(outer,parent));
  const inner=e.gk_heap_enter()>>>0;assert(inner>outer);
  status(e.gk_heap_clear(outer),11);
  status(e.gk_heap_pin(inner,host),12);
  status(e.gk_heap_pin(inner,0),4);
  prepare([0xffff,0xdfff,0]);const returned=e.gk_heap_string(inner)>>>0;assert(returned>parent);
  status(e.gk_heap_leave(inner,host),12);
  raw(returned,[0xffff,0xdfff,0]);
  status(e.gk_heap_leave(inner,0xffffffff),4);
  status(e.gk_heap_leave(outer,0),11);
  status(e.gk_heap_leave(inner,returned));
  for(let n=0;n<500;n++){
    prepare([n,0xd800,0,0xdc00]);const temporary=e.gk_heap_string(outer)>>>0;
    assert(temporary>returned);
    status(e.gk_heap_clear(outer));
    status(e.gk_heap_pin(outer,parent));status(e.gk_heap_pin(outer,returned));
    status(e.gk_heap_pin(outer,temporary));
    raw(temporary,[n,0xd800,0,0xdc00]);
    status(e.gk_json_release(temporary));
    raw(parent,[0xd800,0,0xdc00]);raw(returned,[0xffff,0xdfff,0]);
  }
  status(e.gk_heap_leave(outer,returned));
  status(e.gk_json_string_units(parent),4);
  const after=e.gk_heap_enter()>>>0;assert(after>inner);
  const garbage=e.gk_heap_string(after)>>>0;assert(garbage>returned);
  status(e.gk_heap_abort());
  status(e.gk_json_string_units(garbage),4);
  status(e.gk_heap_clear(after),11);
  raw(returned,[0xffff,0xdfff,0]);raw(host,[72]);
  status(e.gk_json_release(returned));status(e.gk_json_release(host));
  status(e.gk_heap_abort());
  let last=after;
  for(let n=0;n<128;n++){const next=e.gk_heap_enter()>>>0;assert(next>last);last=next;}
  assert.equal(e.gk_heap_enter(),0);status(e.gk_json_status(),10);
  status(e.gk_heap_clear(last));status(e.gk_heap_abort());
  const frame=e.gk_heap_enter()>>>0;assert(frame>last);
  prepare([48]);const owners=Array.from({length:63},()=>e.gk_json_parse()>>>0);
  assert(owners.every(id=>id>0));assert.equal(new Set(owners).size,63);
  const held=e.gk_heap_string(frame)>>>0;assert(held>owners.at(-1));
  status(e.gk_heap_pin(frame,held));
  assert.equal(e.gk_heap_string(frame),0);status(e.gk_json_status(),2);
  raw(held,[48]);
  status(e.gk_heap_clear(frame));
  const replacement=e.gk_heap_string(frame)>>>0;assert(replacement>held);
  status(e.gk_json_string_units(held),4);
  status(e.gk_heap_leave(frame,replacement));status(e.gk_heap_abort());
  raw(replacement,[48]);status(e.gk_json_release(replacement));
  for(const id of owners){assert.equal(e.gk_json_stringify(id),0);assert.deepEqual(output(),[48]);status(e.gk_json_release(id));}
  return {frameLimit:128,rootLimit:64,temporaryCycles:500,checkedTokens:true,transfer:true,abort:true};
}
