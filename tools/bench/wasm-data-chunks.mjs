// P1 core-Wasm data-section evidence, never executable binary transmission.
// This is a bounded parser for the segment forms used by standalone Rust
// fixtures, not a general Wasm interpreter or semantic comparison.
import { createHash } from 'node:crypto';
const MAGIC=Buffer.from([0,97,115,109,1,0,0,0]);
const MAX_WASM=16*1024*1024;
const MAX_CHUNKS=4096;
const MAX_SEGMENTS=128;
const CHUNK_BYTES=256;
const SHA=/^[a-f0-9]{64}$/;
const check=(ok,label)=>{if(!ok)throw new TypeError('Invalid '+label)};
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');

function varU32(bytes,cursor,limit){
  let value=0;
  for(let i=0;i<5;i++){
    check(cursor.offset<limit,'truncated unsigned Wasm LEB');
    const byte=bytes[cursor.offset++];
    check(i<4||(byte&0x70)===0,'oversized unsigned Wasm LEB');
    value+=(byte&0x7f)*2**(7*i);
    if(!(byte&0x80))return value;
  }
  throw new TypeError('Invalid unterminated unsigned Wasm LEB');
}
function offsetExpression(bytes,cursor,limit){
  const start=cursor.offset;
  check(cursor.offset<limit,'offset expression opcode');
  const opcode=bytes[cursor.offset++];
  if(opcode===0x41) {
    let finished=false;
    for(let i=0;i<5;i++){
      check(cursor.offset<limit,'truncated signed i32.const LEB');
      const value=bytes[cursor.offset++];
      if(i===4){
        const sign=(value&0x08)!==0;
        check((value&0x70)===(sign?0x70:0),
          'signed i32.const LEB overflow');
      }
      if(!(value&0x80)){finished=true;break}
    }
    check(finished,'terminated i32.const LEB');
  } else if(opcode===0x23) {
    varU32(bytes,cursor,limit); // global.get offset, without interpretation
  } else {
    throw new TypeError('Invalid unsupported Wasm data offset expression');
  }
  check(cursor.offset<limit&&bytes[cursor.offset++]===0x0b,
    'terminated simple Wasm data offset expression');
  return digest(bytes.subarray(start,cursor.offset));
}
function hashChunks(payload){
  const count=Math.ceil(payload.length/CHUNK_BYTES);
  check(count<=MAX_CHUNKS,'bounded data chunk digest count');
  const hashes=Buffer.alloc(count*32);
  for(let i=0;i<count;i++){
    createHash('sha256').update(
      payload.subarray(i*CHUNK_BYTES,Math.min((i+1)*CHUNK_BYTES,payload.length))
    ).digest().copy(hashes,i*32);
  }
  return {count,base64:hashes.toString('base64')};
}

export function fingerprintWasmData(input){
  check(input instanceof Uint8Array&&input.byteLength>=8&&
    input.byteLength<=MAX_WASM,'bounded core Wasm bytes');
  const bytes=Buffer.from(input.buffer,input.byteOffset,input.byteLength);
  check(bytes.subarray(0,8).equals(MAGIC),'core Wasm magic/version');
  const cursor={offset:8};
  let data=null;
  while(cursor.offset<bytes.length){
    const section=bytes[cursor.offset++];
    check(section<=13,'core Wasm section id');
    const length=varU32(bytes,cursor,bytes.length);
    const start=cursor.offset,end=start+length;
    check(end<=bytes.length,'bounded core Wasm section');
    if(section===11){
      check(data===null,'single Wasm data section');
      const count=varU32(bytes,cursor,end);
      check(count>=1&&count<=MAX_SEGMENTS,'bounded nonempty Wasm data segment vector');
      const segments=[];
      let totalDataBytes=0,totalChunks=0;
      for(let i=0;i<count;i++){
        const form=varU32(bytes,cursor,end);
        check(form<=2,'supported active or passive data segment');
        let memoryIndex=null,offsetExpressionSha256=null;
        if(form===0||form===2){
          memoryIndex=form===2?varU32(bytes,cursor,end):0;
          offsetExpressionSha256=offsetExpression(bytes,cursor,end);
        }
        const size=varU32(bytes,cursor,end);
        check(size<=end-cursor.offset,'bounded Wasm data segment bytes');
        const payload=bytes.subarray(cursor.offset,cursor.offset+size);
        cursor.offset+=size;
        const chunks=hashChunks(payload);
        totalDataBytes+=size;
        totalChunks+=chunks.count;
        check(totalDataBytes<=MAX_WASM&&totalChunks<=MAX_CHUNKS,
          'bounded total initialized Wasm data chunks');
        segments.push({
          form,memoryIndex,offsetExpressionSha256,
          bytes:size,sha256:digest(payload),
          chunkCount:chunks.count,chunkDigestsBase64:chunks.base64,
        });
      }
      check(cursor.offset===end,'complete core Wasm data section');
      data={
        schemaVersion:1,chunkBytes:CHUNK_BYTES,sectionSha256:digest(bytes.subarray(start,end)),
        totalDataBytes,totalChunks,segments,
        interpretation:'raw initialized data segment chunk hashes; no instruction, data meaning or memory equivalence inference',
      };
    }
    cursor.offset=end;
  }
  check(data!==null,'present Wasm initialized-data section');
  return data;
}
function validate(report){
  check(report&&report.schemaVersion===1&&report.chunkBytes===CHUNK_BYTES&&
    SHA.test(report.sectionSha256)&&
    Array.isArray(report.segments)&&report.segments.length>=1&&
    report.segments.length<=MAX_SEGMENTS,'bounded core Wasm data receipt');
  let total=0,chunks=0;
  for(const segment of report.segments){
    check(segment&&Number.isSafeInteger(segment.form)&&segment.form>=0&&segment.form<=2&&
      (segment.form===1 ?
        segment.memoryIndex===null&&segment.offsetExpressionSha256===null :
        Number.isSafeInteger(segment.memoryIndex)&&segment.memoryIndex>=0&&
        SHA.test(segment.offsetExpressionSha256))&&
      Number.isSafeInteger(segment.bytes)&&segment.bytes>=0&&
      segment.bytes<=MAX_WASM&&SHA.test(segment.sha256)&&
      Number.isSafeInteger(segment.chunkCount)&&segment.chunkCount>=0&&
      segment.chunkCount===Math.ceil(segment.bytes/CHUNK_BYTES)&&
      typeof segment.chunkDigestsBase64==='string'&&
      (segment.chunkDigestsBase64===''||
        /^[A-Za-z0-9+/]+={0,2}$/.test(segment.chunkDigestsBase64)),
    'bounded Wasm data segment identity');
    const hashes=Buffer.from(segment.chunkDigestsBase64,'base64');
    check(hashes.length===segment.chunkCount*32&&
      hashes.toString('base64')===segment.chunkDigestsBase64,
    'canonical complete initialized-data chunk hashes');
    total+=segment.bytes;
    chunks+=segment.chunkCount;
    check(total<=MAX_WASM&&chunks<=MAX_CHUNKS,'total bounded data bytes and chunks');
  }
  check(total===report.totalDataBytes&&chunks===report.totalChunks,
    'consistent Wasm data-section totals');
  return report;
}

export function compareWasmDataFingerprints(producer,receiver){
  validate(producer);validate(receiver);
  const first=producer.segments,second=receiver.segments;
  const count=Math.max(first.length,second.length);
  const segmentComparisons=[];
  for(let i=0;i<count;i++){
    const a=first[i],b=second[i];
    if(!a||!b){
      segmentComparisons.push({index:i,missingOn:!a?'producer':'receiver'});
      continue;
    }
    const sameLayout=a.form===b.form&&a.memoryIndex===b.memoryIndex&&
      a.offsetExpressionSha256===b.offsetExpressionSha256&&a.bytes===b.bytes;
    const samePayload=a.bytes===b.bytes&&a.sha256===b.sha256;
    let matchingChunks=null,firstChangedChunks=[];
    if(a.chunkCount===b.chunkCount&&a.bytes===b.bytes){
      const left=Buffer.from(a.chunkDigestsBase64,'base64');
      const right=Buffer.from(b.chunkDigestsBase64,'base64');
      matchingChunks=0;
      for(let j=0;j<a.chunkCount;j++){
        if(left.subarray(j*32,(j+1)*32).equals(right.subarray(j*32,(j+1)*32))){
          matchingChunks++;
        }else if(firstChangedChunks.length<24){
          firstChangedChunks.push(j);
        }
      }
    }
    segmentComparisons.push({
      index:i,sameLayout,samePayload,
      bytes:[a.bytes,b.bytes],chunkCounts:[a.chunkCount,b.chunkCount],
      matchingChunks,firstChangedChunks,
      changedChunkCount:matchingChunks===null?null:a.chunkCount-matchingChunks,
    });
  }
  const changed=segmentComparisons.filter(s=>s.missingOn||
    !s.sameLayout||!s.samePayload);
  return {
    sameDataSectionBytes:producer.sectionSha256===receiver.sectionSha256,
    producerDataBytes:producer.totalDataBytes,
    receiverDataBytes:receiver.totalDataBytes,
    producerSegments:first.length,receiverSegments:second.length,
    differingSegments:changed,
    allSegmentsByteIdentical:changed.length===0,
    interpretation:'fixed 256-byte raw data-segment chunk comparison; does not identify strings, relocations, semantic values or byte-level changed positions',
  };
}
