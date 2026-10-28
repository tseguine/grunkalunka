import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import {
  fingerprintWasmData, compareWasmDataFingerprints,
} from '../wasm-data-chunks.mjs';

const magic=Buffer.from([0,97,115,109,1,0,0,0]);
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
function u32(n){
  const bytes=[];
  do{
    const digit=n%128;
    n=Math.floor(n/128);
    bytes.push(digit|(n>0?128:0));
  }while(n>0);
  return bytes;
}
const section=(id,payload)=>Buffer.from([id,...u32(payload.length),...payload]);
const active=(data,offset=0)=>[0,0x41,offset,0x0b,...u32(data.length),...data];
const passive=data=>[1,...u32(data.length),...data];
const indexed=(data,offset=0)=>[2,0,0x41,offset,0x0b,...u32(data.length),...data];
const moduleOf=(...segments)=>Buffer.concat([magic,section(11,
  [ ...u32(segments.length),...segments.flat() ])]);

test('hashes exact initialized data segment bytes without returning them',()=>{
  const bytes=Buffer.from([1,2,3,4]);
  const r=fingerprintWasmData(moduleOf(active(bytes)));
  assert.equal(r.schemaVersion,1);
  assert.equal(r.chunkBytes,256);
  assert.equal(r.totalDataBytes,4);
  assert.equal(r.totalChunks,1);
  assert.equal(r.segments.length,1);
  assert.equal(r.segments[0].form,0);
  assert.equal(r.segments[0].memoryIndex,0);
  assert.equal(r.segments[0].bytes,4);
  assert.equal(r.segments[0].sha256,sha(bytes));
  assert.equal(Buffer.from(r.segments[0].chunkDigestsBase64,'base64').toString('hex'),
    sha(bytes));
  assert.ok(!JSON.stringify(r).includes('"01020304"'));
});

test('a single changed 256-byte chunk is localized without exposing data',()=>{
  const a=Buffer.alloc(600,7),b=Buffer.from(a);
  b[319]=8;
  const x=fingerprintWasmData(moduleOf(active(a)));
  const y=fingerprintWasmData(moduleOf(active(b)));
  const r=compareWasmDataFingerprints(x,y);
  assert.equal(r.sameDataSectionBytes,false);
  assert.equal(r.allSegmentsByteIdentical,false);
  assert.equal(r.differingSegments.length,1);
  assert.equal(r.differingSegments[0].sameLayout,true);
  assert.equal(r.differingSegments[0].samePayload,false);
  assert.equal(r.differingSegments[0].matchingChunks,2);
  assert.equal(r.differingSegments[0].changedChunkCount,1);
  assert.deepEqual(r.differingSegments[0].firstChangedChunks,[1]);
  assert.deepEqual(r.differingSegments[0].bytes,[600,600]);
});

test('data offsets change segment layout even when bytes are identical',()=>{
  const a=fingerprintWasmData(moduleOf(active([1,2],0)));
  const b=fingerprintWasmData(moduleOf(active([1,2],1)));
  const r=compareWasmDataFingerprints(a,b);
  assert.equal(r.sameDataSectionBytes,false);
  assert.equal(r.differingSegments[0].sameLayout,false);
  assert.equal(r.differingSegments[0].samePayload,true);
  assert.equal(r.differingSegments[0].changedChunkCount,0);
});

test('passive data and explicit memory index segment forms remain distinguishable',()=>{
  const report=fingerprintWasmData(moduleOf(passive([1,2]),indexed([3,4])));
  assert.equal(report.segments.length,2);
  assert.equal(report.segments[0].form,1);
  assert.equal(report.segments[0].offsetExpressionSha256,null);
  assert.equal(report.segments[0].memoryIndex,null);
  assert.equal(report.segments[1].form,2);
  assert.equal(report.segments[1].memoryIndex,0);
  assert.match(report.segments[1].offsetExpressionSha256,/^[a-f0-9]{64}$/);
  assert.deepEqual(compareWasmDataFingerprints(report,structuredClone(report)).differingSegments,[]);
});

test('different data segment counts and sizes do not invent chunk comparisons',()=>{
  const first=fingerprintWasmData(moduleOf(active([1,2,3])));
  const second=fingerprintWasmData(moduleOf(active([1,2]),passive([4])));
  const result=compareWasmDataFingerprints(first,second);
  assert.equal(result.differingSegments.length,2);
  assert.equal(result.differingSegments[0].matchingChunks,null);
  assert.equal(result.differingSegments[0].changedChunkCount,null);
  assert.equal(result.differingSegments[1].missingOn,'producer');
});

test('invalid/truncated LEB, unsupported offset opcodes and segment modes fail closed',()=>{
  for(const bytes of [
    Buffer.from([]),
    Buffer.from([1,2,3,4,1,0,0,0]),
    magic,
    Buffer.concat([magic,Buffer.from([11,255])]),
    Buffer.concat([magic,section(11,[1,3,0,0])]),
    Buffer.concat([magic,section(11,[1,0,0x42,0,0x0b,0])]),
    Buffer.concat([magic,section(11,[1,0,0x41,0,0x7,0])]),
    Buffer.concat([magic,section(11,[1,0,0x41,0,0x0b,4,1])]),
    Buffer.concat([moduleOf(active([1])),section(11,[1,0,0x41,0,0x0b,0])]),
  ]){
    assert.throws(()=>fingerprintWasmData(bytes),/Invalid/);
  }
});

test('truncated or forged chunk-vector fingerprints are rejected',()=>{
  const a=fingerprintWasmData(moduleOf(active([1,2,3])));
  const b=structuredClone(a);
  b.segments[0].chunkDigestsBase64='not base64!';
  assert.throws(()=>compareWasmDataFingerprints(a,b),/bounded Wasm data segment identity/);
  b.segments[0].chunkDigestsBase64=Buffer.from([1]).toString('base64');
  assert.throws(()=>compareWasmDataFingerprints(a,b),/canonical complete initialized-data chunk hashes/);
  b.segments[0].chunkDigestsBase64=a.segments[0].chunkDigestsBase64;
  b.totalDataBytes=4;
  assert.throws(()=>compareWasmDataFingerprints(a,b),/consistent Wasm data-section totals/);
  b.totalDataBytes=3;
  b.segments[0].chunkCount=2;
  assert.throws(()=>compareWasmDataFingerprints(a,b),/bounded Wasm data segment identity/);
});

test('empty passive data is bounded and yields zero chunks',()=>{
  const a=fingerprintWasmData(moduleOf(passive([])));
  const b=structuredClone(a);
  assert.equal(a.totalDataBytes,0);
  assert.equal(a.totalChunks,0);
  assert.equal(a.segments[0].chunkDigestsBase64,'');
  assert.equal(compareWasmDataFingerprints(a,b).sameDataSectionBytes,true);
});
