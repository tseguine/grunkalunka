import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash } from 'node:crypto';
import {
  parseCargoScriptEvents, compareScriptOutputs,
} from '../native-cargo-build-script.mjs';

const sha=s=>createHash('sha256').update(s).digest('hex');
const event={
  reason:'build-script-executed',
  package_id:'registry+https://github.com/rust-lang/crates.io-index#pulldown-cmark@0.13.5',
  linked_libs:['static=math'],
  linked_paths:['native=/tmp/source/build'],
  cfgs:['has_rust_1_99','feature="simd"'],
  env:[['GENERATED_MODE','enabled']],
  out_dir:'/tmp/host-specific-target/build/pulldown-cmark/out',
};
const messages=(...events)=>events.map(x=>JSON.stringify(x)).join('\n');
const success={reason:'build-finished',success:true};
const record=(file='generated.rs',digest=sha('generated'))=>({
  package:'pulldown-cmark@0.13.5',
  totalBytes:9,
  files:[{path:file,bytes:9,sha256:digest}],
  cfgHashes:[sha('has_rust_1_99'),sha('feature="simd"')].sort(),
  env:[{key:'GENERATED_MODE',valueSha256:sha('enabled')}],
  linkLibraryHashes:[sha('static=math')],
  linkPathHashes:[sha('native=/tmp/source/build')],
});
function receipt(os='linux',arch='x64'){
  return{
    schemaVersion:1,workload:'p1-native-cargo-build-script-outputs',
    revision:'d'.repeat(40),
    environment:{
      os,arch,node:'v24.21.0',
      rustc:'rustc 1.99.1 (synthetic)',rustcCommit:'e'.repeat(40),
      rustcHost:os==='linux'?'x86_64-unknown-linux-gnu':
        os==='win32'?'x86_64-pc-windows-msvc':'aarch64-apple-darwin',
      llvm:'23.1.1',
      buildCommand:['cargo','build','--message-format=json','--locked','--offline',
        '-j','4','-p','grunkalunka-markdown','--example','outline_guest',
        '--target','wasm32-unknown-unknown'],
    },
    guest:{sha256:'b'.repeat(64),bytes:735367},
    scripts:[record()],
  };
}

test('parses documented Cargo build-script JSON and hashes values not paths',()=>{
  const parsed=parseCargoScriptEvents(messages(
    {reason:'compiler-artifact',package_id:'unused'},event,success));
  assert.equal(parsed.length,1);
  assert.equal(parsed[0].packageId,event.package_id);
  assert.equal(parsed[0].outDir,event.out_dir);
  assert.deepEqual(parsed[0].cfgHashes,[sha('has_rust_1_99'),sha('feature="simd"')].sort());
  assert.deepEqual(parsed[0].env,[{key:'GENERATED_MODE',valueSha256:sha('enabled')}]);
  assert.deepEqual(parsed[0].linkPathHashes,[sha('native=/tmp/source/build')]);
  assert.ok(!JSON.stringify(parsed[0].cfgHashes).includes('simd'));
});

test('unrelated compiler messages do not replace completed build scripts',()=>{
  assert.throws(()=>parseCargoScriptEvents(messages(success)),
    /finished actual Cargo build with build-script output events/);
  assert.throws(()=>parseCargoScriptEvents(messages(event)),
    /finished actual Cargo build with build-script output events/);
  assert.throws(()=>parseCargoScriptEvents(messages(event,
    {reason:'build-finished',success:false})),/successful Cargo build/);
});

test('unchanged generated file hashes and cfg/env inputs compare exactly',()=>{
  const comparison=compareScriptOutputs(receipt(),receipt('win32'));
  assert.equal(comparison.workload,'p1-native-cargo-build-script-comparison');
  assert.equal(comparison.findings.length,1);
  assert.equal(comparison.findings[0].sameGeneratedFiles,true);
  assert.equal(comparison.findings[0].sameCfgHashes,true);
  assert.equal(comparison.findings[0].sameEnvHashes,true);
  assert.deepEqual(comparison.findings[0].changedFiles,[]);
});

test('a different generated file hash is an observation rather than a failure',()=>{
  const linux=receipt(),mac=receipt('darwin','arm64');
  mac.scripts[0].files[0].sha256='f'.repeat(64);
  const finding=compareScriptOutputs(linux,mac).findings[0];
  assert.equal(finding.sameGeneratedFiles,false);
  assert.deepEqual(finding.generatedBytes,[9,9]);
  assert.deepEqual(finding.changedFiles,[{
    path:'generated.rs',
    producer:{path:'generated.rs',bytes:9,sha256:sha('generated')},
    receiver:{path:'generated.rs',bytes:9,sha256:'f'.repeat(64)},
  }]);
});

test('script cfg and hashed environment differences remain distinct',()=>{
  const linux=receipt(),win=receipt('win32');
  win.scripts[0].cfgHashes=[sha('different')];
  win.scripts[0].env[0].valueSha256=sha('disabled');
  const finding=compareScriptOutputs(linux,win).findings[0];
  assert.equal(finding.sameGeneratedFiles,true);
  assert.equal(finding.sameCfgHashes,false);
  assert.equal(finding.sameEnvHashes,false);
});

test('missing build scripts are reported without inventing file equality',()=>{
  const linux=receipt(),mac=receipt('darwin','arm64');
  mac.scripts[0].package='other@1.0.0';
  const result=compareScriptOutputs(linux,mac);
  assert.equal(result.findings.length,2);
  assert.deepEqual(result.findings.map(x=>[x.package,x.missingOn]),
    [['other@1.0.0','producer'],['pulldown-cmark@0.13.5','receiver']]);
});

test('rejects unsafe output paths, invalid generated hashes and inconsistent counts',()=>{
  const linux=receipt(),other=receipt('win32');
  other.scripts[0].files[0].path='../private';
  assert.throws(()=>compareScriptOutputs(linux,other),/bounded generated file identity/);
  other.scripts[0].files[0].path='generated.rs';
  other.scripts[0].files[0].sha256='broken';
  assert.throws(()=>compareScriptOutputs(linux,other),/bounded generated file identity/);
  other.scripts[0].files[0].sha256=sha('generated');
  other.scripts[0].totalBytes++;
  assert.throws(()=>compareScriptOutputs(linux,other),/generated file inventory byte total/);
});

test('rejects cross-revision or compiler-version comparisons and malformed event env',()=>{
  const linux=receipt(),other=receipt('win32');
  other.revision='f'.repeat(40);
  assert.throws(()=>compareScriptOutputs(linux,other),
    /same tested revision and native compiler release/);
  other.revision=linux.revision;
  other.environment.rustcCommit='f'.repeat(40);
  assert.throws(()=>compareScriptOutputs(linux,other),
    /same tested revision and native compiler release/);
  const corrupted={...event,env:[['../BAD','value']]};
  assert.throws(()=>parseCargoScriptEvents(messages(corrupted,success)),
    /bounded Cargo build-script environment/);
});
