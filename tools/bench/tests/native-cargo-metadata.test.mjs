import assert from 'node:assert/strict';
import test from 'node:test';
import {
  parseVerboseRustcInvocations, compareNativeCargoMetadata,
} from '../native-cargo-metadata.mjs';

const PATHS=[
  'Cargo.toml','Cargo.lock','rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const source=path=>({
  path,gitSha256:'a'.repeat(64),checkoutSha256:'a'.repeat(64),
  gitBytes:32,checkoutBytes:32,checkoutMatchesGit:true,
});
const build=[
  'cargo','build','-vv','--locked','--offline','-j','4',
  '-p','grunkalunka-markdown','--example','outline_guest',
  '--target','wasm32-unknown-unknown',
];
const entry=(crate,auto,extra,variant='default')=>({
  crate,target:'wasm32-unknown-unknown',crateType:crate==='outline_guest'?'bin':'lib',
  metadataValues:variant==='fixed'?[auto,'p1crosshostfixed']:[auto],
  extraFilenameValues:[extra],
});
function report(os='linux',arch='x64'){
  const host=os==='linux'?'x86_64-unknown-linux-gnu':
    os==='win32'?'x86_64-pc-windows-msvc':'aarch64-apple-darwin';
  return{
    schemaVersion:1,workload:'p1-native-cargo-metadata',
    revision:'d'.repeat(40),
    environment:{
      os,arch,node:'v24.21.0',
      rustc:'rustc 1.99.1 (synthetic)',
      rustcCommit:'e'.repeat(40),rustcHost:host,llvm:'23.1.1',
      target:'wasm32-unknown-unknown',cargoIncremental:'0',
      buildCommand:[...build],
    },
    inputs:PATHS.map(source),
    modes:{
      default:{
        requestedRustflags:'',
        artifact:{bytes:700,sha256:'b'.repeat(64)},
        invocations:[
          entry('outline_guest','123abc','-123'),
          entry('pulldown_cmark','456def','-456'),
        ],
      },
      fixed:{
        requestedRustflags:'-C metadata=p1crosshostfixed',
        artifact:{bytes:704,sha256:'c'.repeat(64)},
        invocations:[
          entry('outline_guest','123abc','-123','fixed'),
          entry('pulldown_cmark','456def','-456','fixed'),
        ],
      },
    },
  };
}
const sampleLog = [
  '   Compiling pulldown-cmark v0.13.5',
  '     Running \x60/home/runner/.rustup/toolchains/stable/bin/rustc --crate-name pulldown_cmark --edition=2021 /tmp/lib.rs --crate-type lib --target wasm32-unknown-unknown -C metadata=456def -C extra-filename=-456\x60',
  '     Running \x60/home/runner/.rustup/toolchains/stable/bin/rustc --crate-name outline_guest --edition=2024 /tmp/guest.rs --crate-type bin --target wasm32-unknown-unknown -C metadata=123abc -C extra-filename=-123\x60',
].join('\n');

test('parses actual Cargo verbose Rust commands without retaining source paths',()=>{
  const entries=parseVerboseRustcInvocations(sampleLog);
  assert.deepEqual(entries,report().modes.default.invocations);
  const encoded=JSON.stringify(entries);
  assert.ok(!encoded.includes('/home/runner'));
  assert.ok(!encoded.includes('/tmp/guest.rs'));
});

test('parses Windows quoted rustc executable and multiple metadata values',()=>{
  const windows=[
    'Running \x60"C:\\\\rust\\\\bin\\\\rustc.exe" --crate-name pulldown_cmark --target wasm32-unknown-unknown --crate-type lib -C metadata=456def -C metadata=p1crosshostfixed -C extra-filename=-456\x60',
    'Running \x60"C:\\\\rust\\\\bin\\\\rustc.exe" --crate-name outline_guest --target wasm32-unknown-unknown --crate-type bin -C metadata=123abc -C metadata=p1crosshostfixed -C extra-filename=-123\x60',
  ].join('\n');
  assert.deepEqual(parseVerboseRustcInvocations(windows),
    report().modes.fixed.invocations);
});

test('rejects logs without actual Rust guest and Markdown parser invocations',()=>{
  assert.throws(()=>parseVerboseRustcInvocations('Compiling outline_guest'),
    /actual target rustc commands/);
  const missing=sampleLog.split('\n').slice(0,2).join('\n');
  assert.throws(()=>parseVerboseRustcInvocations(missing),
    /actual target rustc commands/);
});

test('same Cargo metadata arguments on different hosts produce no group differences',()=>{
  const a=report(),b=report('win32');
  const compared=compareNativeCargoMetadata(a,b);
  assert.equal(compared.workload,'p1-native-cargo-metadata-comparison');
  assert.equal(compared.modes.default.matchingGroups,2);
  assert.equal(compared.modes.default.totalGroups,2);
  assert.deepEqual(compared.modes.default.differingGroups,[]);
  assert.equal(compared.modes.fixed.artifactDigestAndSizeEqual,true);
});

test('different per-crate Cargo generated metadata stays visible under one fixed requested flag',()=>{
  const linux=report(),mac=report('darwin','arm64');
  mac.modes.default.invocations[1].metadataValues[0]='999abc';
  mac.modes.fixed.invocations[1].metadataValues[0]='999abc';
  const diff=compareNativeCargoMetadata(linux,mac);
  for(const name of ['default','fixed']){
    const mode=diff.modes[name];
    assert.equal(mode.matchingGroups,1);
    assert.equal(mode.differingGroups.length,1);
    assert.equal(mode.differingGroups[0].crateTarget,
      'pulldown_cmark|wasm32-unknown-unknown|lib');
    assert.deepEqual(mode.differingGroups[0].producer[0].metadataValues,
      name==='fixed'?['456def','p1crosshostfixed']:['456def']);
  }
});

test('rejects inconsistent Rust source/version/roles and forged metadata flags',()=>{
  const linux=report(),windows=report('win32');
  windows.inputs[0].gitSha256='f'.repeat(64);
  windows.inputs[0].checkoutSha256='f'.repeat(64);
  assert.throws(()=>compareNativeCargoMetadata(linux,windows),
    /identical canonical Git source bytes/);
  windows.inputs[0]=source(PATHS[0]);
  windows.environment.rustcCommit='f'.repeat(40);
  assert.throws(()=>compareNativeCargoMetadata(linux,windows),
    /same Rust compiler\/LLVM\/Node versions/);
  windows.environment.rustcCommit='e'.repeat(40);
  windows.modes.fixed.invocations[1].metadataValues.pop();
  assert.throws(()=>compareNativeCargoMetadata(linux,windows),
    /real guest\/parser rustc and expected explicit metadata flags/);
  windows.modes.fixed.invocations[1].metadataValues.push('p1crosshostfixed');
  assert.throws(()=>compareNativeCargoMetadata(windows,linux),
    /native Linux producer and receiver/);
});

test('rejects unsorted, invalid or missing actual compiler metadata entries',()=>{
  const a=report(),b=report('win32');
  b.modes.default.invocations.reverse();
  assert.throws(()=>compareNativeCargoMetadata(a,b),
    /sorted bounded actual rustc arguments/);
  b.modes.default.invocations.reverse();
  b.modes.default.invocations[0].metadataValues=[];
  assert.throws(()=>compareNativeCargoMetadata(a,b),
    /sorted bounded actual rustc arguments/);
  b.modes.default.invocations=report().modes.default.invocations;
  b.modes.fixed.invocations[0].extraFilenameValues=['../unsafe'];
  assert.throws(()=>compareNativeCargoMetadata(a,b),
    /sorted bounded actual rustc arguments/);
});

test('default and fixed metadata modes cannot be confused or dropped',()=>{
  const a=report(),b=report('win32');
  b.modes.fixed.requestedRustflags='';
  assert.throws(()=>compareNativeCargoMetadata(a,b),
    /native fixed metadata mode/);
  b.modes.fixed.requestedRustflags='-C metadata=p1crosshostfixed';
  delete b.modes.default;
  assert.throws(()=>compareNativeCargoMetadata(a,b),
    /default and fixed metadata inventories/);
});
