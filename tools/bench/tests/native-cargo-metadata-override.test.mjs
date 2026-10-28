import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import {
  parseMetadataOverrideLog, compareNativeMetadataOverrides,
} from '../native-cargo-metadata-override.mjs';

const TARGET='wasm32-unknown-unknown';
const names={
  build_script_build:['host','p1script'],
  unicase:[TARGET,'p1unicase'],
  pulldown_cmark:[TARGET,'p1pulldown'],
  grunkalunka_markdown:[TARGET,'p1markdown'],
  outline_guest:[TARGET,'p1guest'],
};
const paths=[
  'Cargo.toml','Cargo.lock','rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
  'tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs',
];
const src=path=>({
  path,gitSha256:'a'.repeat(64),checkoutSha256:'a'.repeat(64),
  gitBytes:12,checkoutBytes:12,checkoutMatchesGit:true,
});
const hex=key=>(key*17).toString(16).padStart(16,'0');
function row(crate,mode,kind='linux'){
  const [target,effective]=names[crate];
  const index=Object.keys(names).sort().indexOf(crate);
  const original=kind==='linux'?hex(index+2):hex(index+22);
  const changed=mode==='all'||(mode==='target'&&target===TARGET);
  return {crate,target,original,effective:changed?effective:original,changed};
}
const metadata=(mode,host)=>Object.keys(names).sort().map(crate=>row(crate,mode,host));
const magic=Buffer.from([0,97,115,109,1,0,0,0]);
function wasm(bytes){
  const body=Buffer.from(bytes);
  const payload=Buffer.from([1,body.length,...body]);
  return Buffer.concat([magic,Buffer.from([10,payload.length]),payload]);
}
function sectionReceipt(mode,os,arch,body=[0,11]){
  const bytes=wasm(body);
  return {
    schemaVersion:1,workload:'p1-markdown-guest-wasm-sections',
    revision:'e'.repeat(40),
    environment:{
      os,arch,node:'v24.21.0',rustc:'rustc 1.99.1 (synthetic)',
      rustcHost:os==='linux'?'x86_64-unknown-linux-gnu':
        os==='win32'?'x86_64-pc-windows-msvc':'aarch64-apple-darwin',
      target:TARGET,profile:'dev',rustflags:'',cargoEncodedRustflags:'',
    },
    inputs:[
      'Cargo.lock','crates/grunkalunka-markdown/src/lib.rs',
      'crates/grunkalunka-markdown/examples/outline_guest.rs',
    ].map(src),
    ...inspectWasmSections(bytes,{codeBodyDigests:true}),
  };
}
function receipt(os='linux',arch='x64',body=[0,11]){
  return {
    schemaVersion:1,workload:'p1-native-cargo-metadata-override',
    revision:'e'.repeat(40),
    environment:{
      os,arch,node:'v24.21.0',rustc:'rustc 1.99.1 (synthetic)',
      rustcHost:os==='linux'?'x86_64-unknown-linux-gnu':
        os==='win32'?'x86_64-pc-windows-msvc':'aarch64-apple-darwin',
      rustcCommit:'f'.repeat(40),llvm:'23.1.1',target:TARGET,
      cargoIncremental:'0',
      cargoCommand:['cargo','build','--locked','--offline','-j','1',
        '-p','grunkalunka-markdown','--example','outline_guest','--target',TARGET],
      wrapperSourceSha256:'a'.repeat(64),
      privateWrapperPolicy:
        'one fixed wrapper executable and absolute target directory, complete clean between variants',
    },
    inputs:paths.map(src),
    variants:Object.fromEntries(['passthrough','target','all'].map(mode=>
      [mode,{mode,actualMetadata:metadata(mode,os),
        report:sectionReceipt(mode,os,arch,body)}])),
  };
}

test('observed compiler wrapper modes require exact five-crate argument inventory',()=>{
  const log=metadata('passthrough','linux').reverse()
    .map(x=>[x.crate,x.target,x.original,x.effective].join('\t')).join('\n')+'\n';
  const rows=parseMetadataOverrideLog(log,'passthrough');
  assert.deepEqual(rows,metadata('passthrough','linux'));
  assert.equal(rows.length,5);
  assert.ok(rows.every(x=>!x.changed));
});

test('explicit target-only overrides four crates and preserves native build-script hash',()=>{
  const log=metadata('target','linux')
    .map(x=>[x.crate,x.target,x.original,x.effective].join('\t')).join('\n');
  const rows=parseMetadataOverrideLog(log,'target');
  assert.equal(rows.filter(x=>x.changed).length,4);
  assert.equal(rows.find(x=>x.crate==='build_script_build').changed,false);
  assert.equal(rows.find(x=>x.crate==='outline_guest').effective,'p1guest');
});

test('all-crate mode replaces generated metadata on all five crate commands',()=>{
  const log=metadata('all','win32')
    .map(x=>[x.crate,x.target,x.original,x.effective].join('\t')).join('\n');
  const rows=parseMetadataOverrideLog(log,'all');
  assert.equal(rows.filter(x=>x.changed).length,5);
  assert.equal(rows.find(x=>x.crate==='build_script_build').effective,'p1script');
});

test('rejects forged values, unknown crates, duplicate rows and malformed generated metadata',()=>{
  const good=metadata('all','linux')
    .map(x=>[x.crate,x.target,x.original,x.effective].join('\t'));
  for(const replacement of [
    'other\twasm32-unknown-unknown\t0123456789abcdef\tp1guest',
    'outline_guest\twasm32-wasip1\t0123456789abcdef\tp1guest',
    'outline_guest\twasm32-unknown-unknown\tinvalid\tp1guest',
    'outline_guest\twasm32-unknown-unknown\t0123456789abcdef\twrong',
  ]){
    const altered=good.slice();
    altered[2]=replacement;
    assert.throws(()=>parseMetadataOverrideLog(altered.join('\n'),'all'),/Invalid/);
  }
  assert.throws(()=>parseMetadataOverrideLog(good.slice(0,4).join('\n'),'all'),
    /one real rustc invocation/);
  assert.throws(()=>parseMetadataOverrideLog(good.concat(good[0]).join('\n'),'all'),
    /one real rustc invocation/);
  assert.throws(()=>parseMetadataOverrideLog(good.join('\n'),'other'),/Invalid/);
});

test('same source and native Rust versions permit comparing different generated hashes',()=>{
  const linux=receipt();
  const windows=receipt('win32','x64',[0,1,11]);
  const result=compareNativeMetadataOverrides(linux,windows);
  assert.equal(result.schemaVersion,1);
  assert.equal(result.workload,'p1-native-cargo-metadata-override-comparison');
  assert.equal(result.metadata.passthrough.generatedSame,0);
  assert.equal(result.metadata.passthrough.effectiveSame,0);
  assert.equal(result.metadata.target.effectiveSame,4);
  assert.deepEqual(result.metadata.target.remainingMetadataMismatches,
    ['build_script_build']);
  assert.equal(result.metadata.all.effectiveSame,5);
  assert.deepEqual(result.metadata.all.remainingMetadataMismatches,[]);
  assert.equal(result.crossHost.all.sameArtifactBytes,false);
  assert.equal(result.crossHost.all.matchingIgnoringOrder,0);
  assert.equal(result.withinHost.producer.passthroughToTarget.sameArtifactBytes,true);
});

test('output equality and metadata-input equality are independent observations',()=>{
  const a=receipt(),b=receipt('darwin','arm64');
  const comparison=compareNativeMetadataOverrides(a,b);
  assert.equal(comparison.crossHost.passthrough.sameArtifactBytes,true);
  assert.equal(comparison.metadata.passthrough.generatedSame,0);
  assert.equal(comparison.metadata.all.effectiveSame,5);
  assert.equal(comparison.crossHost.all.matchingAtOrdinal,1);
});

test('rejects changing the original Cargo-generated hash between modes on one host',()=>{
  const a=receipt(),b=receipt('win32');
  b.variants.target.actualMetadata[3].original='1'.repeat(16);
  assert.throws(()=>compareNativeMetadataOverrides(a,b),
    /unchanged Cargo-generated metadata across wrapper modes/);
});

test('rejects wrong target, unexpected host role and forged compiler/toolchain',()=>{
  const a=receipt(),b=receipt('win32');
  b.variants.all.actualMetadata[0].target=TARGET;
  assert.throws(()=>compareNativeMetadataOverrides(a,b),
    /complete observed generated metadata/);
  b.variants.all.actualMetadata[0].target='host';
  assert.throws(()=>compareNativeMetadataOverrides(b,a),/Linux x64 producer/);
  b.environment.rustcCommit='1'.repeat(40);
  assert.throws(()=>compareNativeMetadataOverrides(a,b),/same Rust\/LLVM\/Node versions/);
});

test('rejects incomplete Wasm function vectors and altered canonical Git sources',()=>{
  const a=receipt(),b=receipt('win32');
  b.variants.all.report.codeBodies.bodyDigestsBase64='broken!';
  assert.throws(()=>compareNativeMetadataOverrides(a,b),/reported code body fingerprints/);
  b.variants.all.report=sectionReceipt('all','win32','x64');
  b.inputs[0].gitSha256='b'.repeat(64);
  b.inputs[0].checkoutSha256='b'.repeat(64);
  assert.throws(()=>compareNativeMetadataOverrides(a,b),/identical tested revision and canonical Git bytes/);
});
