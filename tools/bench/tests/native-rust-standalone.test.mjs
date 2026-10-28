import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import { fingerprintWasmData } from '../wasm-data-chunks.mjs';
import { compareNativeRustWitnesses } from '../native-rust-standalone.mjs';

const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const flags = [
  '--edition=2024', '--crate-type=cdylib', '--target', 'wasm32-unknown-unknown',
  '-C', 'panic=abort', '-C', 'opt-level=0', '-C', 'debuginfo=0',
  '-C', 'codegen-units=1', '-C', 'metadata=p1standalonecontrol',
];
const fixturePaths = {
  core: { path:'tests/build/fixtures/p1-rust-standalone/no_std.rs',
    crate:'p1_rust_core' },
  std: { path:'tests/build/fixtures/p1-rust-standalone/std_alloc.rs',
    crate:'p1_rust_std' },
};
const segment=(id,bytes)=>{
  assert.ok(bytes.length<128);
  return Buffer.from([id,bytes.length,...bytes]);
};
function moduleBytes(bodies, custom=null) {
  const chunks=[magic,segment(10,[bodies.length,
    ...bodies.flatMap(b=>[b.length,...b])])];
  if(custom!==null) chunks.push(segment(0,[4,...Buffer.from('name'),...custom]));
  chunks.push(segment(11,[1,0,0x41,0,0x0b,3,1,2,3]));
  return Buffer.concat(chunks);
}
function fixture(name,bodies,custom=null) {
  const bytes=moduleBytes(bodies,custom);
  return {
    source:{...fixturePaths[name],sha256:'a'.repeat(64),bytes:32},
    wasm:inspectWasmSections(bytes,{codeBodyDigests:true}),
    data:fingerprintWasmData(bytes),
    goldens:{gcdCases:4,extraCases:3,hostImports:0},
  };
}
function receipt(os='linux', arch='x64', core=[[0,11]], std=[[0,11]]) {
  const rustcHost=os==='linux'?'x86_64-unknown-linux-gnu':
    os==='win32'?'x86_64-pc-windows-msvc':'aarch64-apple-darwin';
  return {
    schemaVersion:1, workload:'p1-native-rust-standalone-wasm',
    revision:'d'.repeat(40),
    environment:{
      os,arch,node:'v24.21.0',rustc:'rustc 1.99.1 (synthetic)',
      rustcHost,rustcCommit:'e'.repeat(40),llvm:'23.1.1',
      target:'wasm32-unknown-unknown',flags:[...flags],
      sourcePathMapping:'per-host private source directory remapped to /p1/standalone',
    },
    fixtures:{core:fixture('core',core),std:fixture('std',std)},
  };
}

test('same compiler output across native hosts gives exact artifact identity',()=>{
  const result=compareNativeRustWitnesses(receipt(),receipt('win32'));
  assert.equal(result.workload,'p1-native-rust-standalone-comparison');
  for(const f of Object.values(result.findings)){
    assert.equal(f.identicalWasmDigestAndSize,true);
    assert.deepEqual(f.changedSections,[]);
    assert.deepEqual(f.rawBodyOverlap.counts,[1,1]);
    assert.equal(f.rawBodyOverlap.sameAtOrdinal,1);
    assert.equal(f.dataChunks.sameDataSectionBytes,true);
    assert.equal(f.dataChunks.differingSegments.length,0);
    assert.equal(f.rawBodyOverlap.sameIgnoringOrder,1);
    assert.deepEqual(f.rawBodyOverlap.unmatched,[0,0]);
    assert.equal(f.identicalVerifiedGoldens,true);
  }
});

test('only standard-library fixture changing is reported separately',()=>{
  const linux=receipt();
  const mac=receipt('darwin','arm64',[[0,11]],[[0,1,11]]);
  const result=compareNativeRustWitnesses(linux,mac);
  assert.equal(result.findings.core.identicalWasmDigestAndSize,true);
  assert.equal(result.findings.std.identicalWasmDigestAndSize,false);
  assert.deepEqual(result.findings.std.changedSections,['section:10']);
  assert.deepEqual(result.findings.std.rawBodyOverlap.unmatched,[1,1]);
});

test('body order is distinguished from changed byte sequences',()=>{
  const a=receipt('linux','x64',[[0,11],[0,1,11]]);
  const b=receipt('win32','x64',[[0,1,11],[0,11]]);
  const r=compareNativeRustWitnesses(a,b).findings.core.rawBodyOverlap;
  assert.equal(r.sameAtOrdinal,0);
  assert.equal(r.sameIgnoringOrder,2);
  assert.deepEqual(r.unmatched,[0,0]);
});

test('different function counts keep positional comparison unavailable',()=>{
  const a=receipt('linux','x64',[[0,11],[0,1,11]]);
  const b=receipt('darwin','arm64',[[0,11]]);
  const result=compareNativeRustWitnesses(a,b).findings.core.rawBodyOverlap;
  assert.equal(result.sameAtOrdinal,null);
  assert.equal(result.sameIgnoringOrder,1);
  assert.deepEqual(result.unmatched,[1,0]);
});

test('custom metadata divergence is not mislabeled as executable divergence',()=>{
  const a=receipt('linux'),b=receipt('win32');
  a.fixtures.std=fixture('std',[[0,11]],[1]);
  b.fixtures.std=fixture('std',[[0,11]],[2]);
  const r=compareNativeRustWitnesses(a,b).findings.std;
  assert.equal(r.identicalWasmDigestAndSize,false);
  assert.deepEqual(r.changedSections,['custom:name']);
  assert.equal(r.rawBodyOverlap.sameAtOrdinal,1);
});

test('rejects mismatched canonical original Git bytes even if compiled output matches',()=>{
  const a=receipt(),b=receipt('win32');
  b.fixtures.core.source.sha256='b'.repeat(64);
  assert.throws(()=>compareNativeRustWitnesses(a,b),/exact same original Git fixture source/);
});

test('rejects changed Rust release, compiler commit or target flags',()=>{
  const a=receipt(),b=receipt('win32');
  b.environment.rustcCommit='f'.repeat(40);
  assert.throws(()=>compareNativeRustWitnesses(a,b),/equal Rust release\/commit/);
  b.environment.rustcCommit=a.environment.rustcCommit;
  b.environment.flags.push('-C','opt-level=3');
  assert.throws(()=>compareNativeRustWitnesses(a,b),/same documented Rust control configuration/);
  b.environment.flags=[...flags];
  b.environment.target='wasm32-wasip1';
  assert.throws(()=>compareNativeRustWitnesses(a,b),/same documented Rust control configuration/);
});

test('rejects omitted goldens, malformed code-body digests or invalid host role',()=>{
  const a=receipt(),b=receipt('win32');
  b.fixtures.core.goldens.gcdCases=3;
  assert.throws(()=>compareNativeRustWitnesses(a,b),/observed bounded execution goldens/);
  b.fixtures.core.goldens.gcdCases=4;
  b.fixtures.std.wasm.codeBodies.bodyDigestsBase64='not canonical!';
  assert.throws(()=>compareNativeRustWitnesses(a,b),/bounded code bodies/);
  // Canonically encoded but truncated digest vector must fail independently.
  b.fixtures.std.wasm.codeBodies.bodyDigestsBase64=Buffer.from([1]).toString('base64');
  assert.throws(()=>compareNativeRustWitnesses(a,b),/canonical complete Wasm function body hashes/);
  b.fixtures.std=fixture('std',[[0,11]]);
  assert.throws(()=>compareNativeRustWitnesses(b,a),/Linux x64 producer/);
});
