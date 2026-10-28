import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import { extractNamedBodyHashes } from '../wasm-named-bodies.mjs';
import { MODE_CONFIG, summarizeMetadataScopeControl } from '../markdown-metadata-scope.mjs';

const paths = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const shorter = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const source = path => ({
  path, gitSha256:'a'.repeat(64), checkoutSha256:'a'.repeat(64),
  gitBytes:16, checkoutBytes:16, checkoutMatchesGit:true,
});
const magic = Buffer.from([0,97,115,109,1,0,0,0]);
const part = (id,payload) => {
  assert.ok(payload.length<128);
  return Buffer.from([id,payload.length,...payload]);
};
function wasm(bodies,names) {
  const nameEntries=names.flatMap((name,index)=>{
    const bytes=Buffer.from(name);
    return [index,bytes.length,...bytes];
  });
  const nameMap=[names.length,...nameEntries];
  return Buffer.concat([
    magic,part(1,[1,0x60,0,0]),
    part(3,[bodies.length,...bodies.map(()=>0)]),
    part(10,[bodies.length,...bodies.flatMap(body=>[body.length,...body])]),
    part(0,[4,...Buffer.from('name'),1,nameMap.length,...nameMap]),
  ]);
}
function variant(mode,bodies=[[0,11],[0,1,11]],names=['one','two']) {
  const bytes=wasm(bodies,names);
  const observed=inspectWasmSections(bytes,{codeBodyDigests:true});
  return {
    schemaVersion:1,workload:'p1-markdown-guest-wasm-sections',
    revision:'d'.repeat(40),
    environment:{
      os:'linux',arch:'x64',node:'v24.21.0',
      rustc:'rustc 1.99.1 (synthetic)',
      rustcHost:'x86_64-unknown-linux-gnu',
      rustflags:MODE_CONFIG[mode].rustflags,cargoEncodedRustflags:'',
      target:'wasm32-unknown-unknown',profile:'dev',
    },
    inputs:shorter.map(source),
    ...observed,
    namedBodies:extractNamedBodyHashes(bytes,observed.codeBodies.count),
  };
}
function matrix() {
  return {default:variant('default'),guestOnly:variant('guestOnly'),global:variant('global')};
}
const summarize = variants => summarizeMetadataScopeControl(variants,paths.map(source));

test('all three output identities and exact command scopes are independently recorded',()=>{
  const result=summarize(matrix());
  assert.equal(result.workload,'p1-markdown-metadata-scope-control');
  assert.equal(result.environment.modes.default.command,'cargo build');
  assert.equal(result.environment.modes.guestOnly.command,'cargo rustc');
  assert.deepEqual(result.environment.modes.guestOnly.rustcExtras,
    ['--','-C','metadata=p1guestcontrol']);
  assert.equal(result.environment.modes.global.rustflags,
    '-C metadata=p1guestcontrol');
  for(const cmp of Object.values(result.comparisons)){
    assert.equal(cmp.sameWasmBytes,true);
    assert.deepEqual(cmp.changedSections,[]);
    assert.equal(cmp.nameOccurrences,2);
    assert.equal(cmp.nameWithExactBodyOccurrences,2);
    assert.equal(cmp.bodiesAtSameOrdinal,2);
    assert.equal(cmp.bodiesIgnoringOrder,2);
  }
});

test('guest-only symbol changes are not conflated with global code-body changes',()=>{
  const variants=matrix();
  variants.guestOnly=variant('guestOnly',[[0,11],[0,1,11]],['changed','two']);
  variants.global=variant('global',[[0,1,11],[0,1,11]],['changed','two']);
  const result=summarize(variants);
  assert.deepEqual(result.comparisons.defaultToGuestOnly.changedSections,['custom:name']);
  assert.equal(result.comparisons.defaultToGuestOnly.nameOccurrences,1);
  assert.equal(result.comparisons.defaultToGuestOnly.bodiesIgnoringOrder,2);
  assert.ok(result.comparisons.defaultToGlobal.changedSections.includes('section:10'));
  assert.equal(result.comparisons.defaultToGlobal.bodiesIgnoringOrder,1);
  assert.equal(result.comparisons.guestOnlyToGlobal.nameOccurrences,2);
});

test('different function counts explicitly prevent ordinal body alignment',()=>{
  const variants=matrix();
  variants.global=variant('global',[[0,11]],['one']);
  const result=summarize(variants);
  assert.deepEqual(result.comparisons.defaultToGlobal.definedFunctions,[2,1]);
  assert.equal(result.comparisons.defaultToGlobal.bodiesAtSameOrdinal,null);
  assert.equal(result.comparisons.defaultToGlobal.bodiesIgnoringOrder,1);
});

test('rejects missing matrix modes or guest-only flag forged as global RUSTFLAGS',()=>{
  const variants=matrix();
  delete variants.global;
  assert.throws(()=>summarize(variants),/complete default\/guestOnly\/global matrix/);
  variants.global=variant('global');
  variants.guestOnly.environment.rustflags='-C metadata=p1guestcontrol';
  assert.throws(()=>summarize(variants),/controlled source, toolchain and Rust flags in guestOnly/);
});

test('rejects distinct source, Git revision or native Rust compiler within a matrix',()=>{
  const variants=matrix();
  variants.global.inputs[0].gitSha256='f'.repeat(64);
  variants.global.inputs[0].checkoutSha256='f'.repeat(64);
  assert.throws(()=>summarize(variants),/controlled source, toolchain and Rust flags in global/);
  variants.global=variant('global');
  variants.guestOnly.revision='e'.repeat(40);
  assert.throws(()=>summarize(variants),/controlled source, toolchain and Rust flags in guestOnly/);
  variants.guestOnly=variant('guestOnly');
  variants.global.environment.rustc='rustc 1.99.2';
  assert.throws(()=>summarize(variants),/controlled source, toolchain and Rust flags in global/);
});

test('rejects hidden encoded flags, invalid name digests and invalid body hashes',()=>{
  const variants=matrix();
  variants.global.environment.cargoEncodedRustflags='-Cmetadata=other';
  assert.throws(()=>summarize(variants),/controlled source, toolchain and Rust flags in global/);
  variants.global=variant('global');
  variants.guestOnly.namedBodies.nameDigestsBase64='invalid!';
  assert.throws(()=>summarize(variants),/bounded named function vector/);
  variants.guestOnly=variant('guestOnly');
  variants.global.codeBodies.bodyDigestsBase64=Buffer.from([1]).toString('base64');
  assert.throws(()=>summarize(variants),/canonical complete SHA-256 body vector/);
});

test('rejects inconsistent reported source checksums or forged target',()=>{
  const variants=matrix();
  const inventory=paths.map(source);
  inventory[0].checkoutMatchesGit=false;
  assert.throws(()=>summarizeMetadataScopeControl(variants,inventory),
    /tracked source metadata/);
  variants.global.environment.target='wasm32-wasip1';
  assert.throws(()=>summarize(variants),
    /controlled source, toolchain and Rust flags in global/);
});
