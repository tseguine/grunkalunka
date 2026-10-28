// P1: exact rustc -C metadata/extra-filename flags from actual Cargo -vv.
// Keep native paths, compiler commands, and raw verbose logs out of reports.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const root = fileURLToPath(new URL('../../', import.meta.url));
const SHA = /^[a-f0-9]{64}$/;
const COMMIT = /^[a-f0-9]{40}$/;
const MAX_WASM = 16 * 1024 * 1024;
const MAX_LOG = 32 * 1024 * 1024;
const MAX_ENTRIES = 160;
const MAX_RECEIPT = 192 * 1024;
const MODES = Object.freeze({default:'',fixed:'-C metadata=p1crosshostfixed'});
const BUILD = [
  'build','-vv','--locked','--offline','-j','4',
  '-p','grunkalunka-markdown','--example','outline_guest',
  '--target','wasm32-unknown-unknown',
];
const PATHS = [
  'Cargo.toml','Cargo.lock','rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const GUEST = 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const check=(ok,label)=>{if(!ok)throw new TypeError('Invalid '+label);};
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');

function run(cmd,args,env=process.env,binary=false) {
  const r=spawnSync(cmd,args,{
    cwd:root,env,shell:false,windowsHide:true,timeout:300_000,
    encoding:binary?null:'utf8',maxBuffer:MAX_LOG,
    stdio:['ignore','pipe','pipe'],
  });
  if(r.error||r.signal||r.status!==0){
    throw new Error('Failed '+cmd+' '+args.join(' ')+': '+
      (r.error?.message??r.stderr?.toString('utf8').slice(-2500)??r.signal??r.status));
  }
  return r;
}

// Parse only the actual single-line Cargo verbose Running command spans.
export function parseVerboseRustcInvocations(log) {
  check(typeof log==='string'&&log.length>0&&log.length<=MAX_LOG,
    'bounded Cargo verbose log');
  const invocations=[];
  for(const item of log.matchAll(/Running \x60([^\x60\r\n]+)\x60/g)){
    const cmd=item[1];
    if(!cmd.includes('rustc')||!cmd.includes('--crate-name'))continue;
    const crate=cmd.match(/--crate-name\s+["']?([A-Za-z_][A-Za-z0-9_]*)/)?.[1];
    if(!crate)continue;
    const target=cmd.match(/--target\s+["']?([a-z0-9_-]+)/)?.[1]??'host';
    const crateType=cmd.match(/--crate-type\s+["']?([A-Za-z0-9_,]+)/)?.[1]??'unspecified';
    const metadataValues=[...cmd.matchAll(/-C\s+["']?metadata=([A-Za-z0-9_-]+)/g)]
      .map(x=>x[1]);
    const extraFilenameValues=[...cmd.matchAll(/-C\s+["']?extra-filename=([-A-Za-z0-9_]+)/g)]
      .map(x=>x[1]);
    check(metadataValues.length>=1&&metadataValues.length<=4&&
      extraFilenameValues.length<=4,'rustc metadata flags');
    invocations.push({crate,target,crateType,metadataValues,extraFilenameValues});
    check(invocations.length<=MAX_ENTRIES,'maximum rustc commands');
  }
  check(invocations.some(x=>x.crate==='outline_guest'&&
    x.target==='wasm32-unknown-unknown')&&
    invocations.some(x=>x.crate==='pulldown_cmark'&&
      x.target==='wasm32-unknown-unknown'),
  'actual target rustc commands for Markdown guest and parser');
  invocations.sort((a,b)=>{
    const x=JSON.stringify(a),y=JSON.stringify(b);
    return x<y?-1:x>y?1:0;
  });
  return invocations;
}

function validateSources(inputs){
  check(Array.isArray(inputs)&&inputs.length===PATHS.length,
    'canonical source inventory');
  for(let i=0;i<PATHS.length;i++){
    const x=inputs[i];
    check(x&&x.path===PATHS[i]&&SHA.test(x.gitSha256)&&
      SHA.test(x.checkoutSha256)&&Number.isSafeInteger(x.gitBytes)&&
      x.gitBytes>0&&Number.isSafeInteger(x.checkoutBytes)&&x.checkoutBytes>0&&
      x.checkoutMatchesGit===
        (x.gitSha256===x.checkoutSha256&&x.gitBytes===x.checkoutBytes),
    'tracked source '+PATHS[i]);
  }
}
function validateReport(r){
  check(r&&r.schemaVersion===1&&r.workload==='p1-native-cargo-metadata'&&
    COMMIT.test(r.revision),'Cargo rustc metadata receipt');
  check(r.environment&&['linux','win32','darwin'].includes(r.environment.os)&&
    ['x64','arm64'].includes(r.environment.arch)&&
    typeof r.environment.node==='string'&&
    /^rustc \d+\.\d+\.\d+/.test(r.environment.rustc)&&
    COMMIT.test(r.environment.rustcCommit)&&
    typeof r.environment.llvm==='string'&&r.environment.llvm.length>0&&
    typeof r.environment.rustcHost==='string'&&
    r.environment.target==='wasm32-unknown-unknown'&&
    r.environment.cargoIncremental==='0'&&
    r.environment.buildCommand?.join('\0')===['cargo',...BUILD].join('\0'),
  'native Rust and Cargo build conditions');
  validateSources(r.inputs);
  check(r.modes&&Object.keys(r.modes).sort().join(',')==='default,fixed',
    'default and fixed metadata inventories');
  for(const [mode,flags] of Object.entries(MODES)){
    const v=r.modes[mode];
    check(v&&v.requestedRustflags===flags&&v.artifact&&
      SHA.test(v.artifact.sha256)&&
      Number.isSafeInteger(v.artifact.bytes)&&v.artifact.bytes>=8&&
      v.artifact.bytes<=MAX_WASM&&
      Array.isArray(v.invocations)&&v.invocations.length>=2&&
      v.invocations.length<=MAX_ENTRIES,'native '+mode+' metadata mode');
    let prior='',guest=false,parser=false,fixed=false;
    for(const x of v.invocations){
      const entry=JSON.stringify(x);
      check(x&&/^[A-Za-z_][A-Za-z0-9_]*$/.test(x.crate)&&
        /^[a-z0-9_-]+$/.test(x.target)&&
        /^[A-Za-z0-9_,]+$/.test(x.crateType)&&
        Array.isArray(x.metadataValues)&&x.metadataValues.length>=1&&
        x.metadataValues.length<=4&&
        x.metadataValues.every(y=>/^[A-Za-z0-9_-]+$/.test(y))&&
        Array.isArray(x.extraFilenameValues)&&
        x.extraFilenameValues.length<=4&&
        x.extraFilenameValues.every(y=>/^[-A-Za-z0-9_]+$/.test(y))&&
        entry>=prior,'sorted bounded actual rustc arguments');
      prior=entry;
      if(x.target==='wasm32-unknown-unknown'){
        if(x.crate==='outline_guest')guest=true;
        if(x.crate==='pulldown_cmark')parser=true;
        if(x.metadataValues.includes('p1crosshostfixed'))fixed=true;
      }
    }
    check(guest&&parser&&(mode==='fixed'?fixed:!fixed),
      'actual Markdown rustc commands with expected metadata control');
  }
  return r;
}

function grouped(entries){
  const result=new Map();
  for(const x of entries){
    const key=x.crate+'|'+x.target+'|'+x.crateType;
    if(!result.has(key))result.set(key,[]);
    result.get(key).push({metadataValues:x.metadataValues,
      extraFilenameValues:x.extraFilenameValues});
  }
  for(const list of result.values()){
    list.sort((a,b)=>{
      const x=JSON.stringify(a),y=JSON.stringify(b);
      return x<y?-1:x>y?1:0;
    });
  }
  return result;
}
function compareInvocations(a,b){
  const first=grouped(a),second=grouped(b);
  const keys=[...new Set([...first.keys(),...second.keys()])].sort();
  const differing=[];
  let matching=0;
  for(const key of keys){
    const left=first.get(key)??[],right=second.get(key)??[];
    if(isDeepStrictEqual(left,right))matching++;
    else differing.push({crateTarget:key,producer:left,receiver:right});
  }
  return {matchingGroups:matching,differingGroups:differing,totalGroups:keys.length};
}

export function compareNativeCargoMetadata(linux,receiver){
  validateReport(linux);validateReport(receiver);
  check(linux.environment.os==='linux'&&linux.environment.arch==='x64'&&
    receiver.environment.os!=='linux','native Linux producer and receiver');
  check(linux.revision===receiver.revision,'same exact tested Git revision');
  check(isDeepStrictEqual(
    linux.inputs.map(x=>[x.path,x.gitSha256,x.gitBytes]),
    receiver.inputs.map(x=>[x.path,x.gitSha256,x.gitBytes])),
  'identical canonical Git source bytes');
  check(linux.environment.rustc===receiver.environment.rustc&&
    linux.environment.rustcCommit===receiver.environment.rustcCommit&&
    linux.environment.llvm===receiver.environment.llvm&&
    linux.environment.node===receiver.environment.node,
  'same Rust compiler/LLVM/Node versions');
  return {
    schemaVersion:1,workload:'p1-native-cargo-metadata-comparison',
    revision:linux.revision,
    producer:{...linux.environment},receiver:{...receiver.environment},
    modes:Object.fromEntries(Object.keys(MODES).map(name=>[name,{
      ...compareInvocations(linux.modes[name].invocations,receiver.modes[name].invocations),
      producerArtifact:linux.modes[name].artifact,
      receiverArtifact:receiver.modes[name].artifact,
      artifactDigestAndSizeEqual:isDeepStrictEqual(
        linux.modes[name].artifact,receiver.modes[name].artifact),
    }])),
    interpretation:'Cargo verbose rustc metadata invocation evidence; does not prove whole toolchain or semantic equivalence',
  };
}

async function snapshot(){
  const inputs=[];
  for(const path of PATHS){
    const blob=run('git',['show','HEAD:'+path],process.env,true).stdout;
    const checkout=await readFile(resolve(root,path));
    check(blob.length>0&&blob.length<=8*1024*1024,'bounded Git blob');
    inputs.push({path,gitSha256:hash(blob),gitBytes:blob.length,
      checkoutSha256:hash(checkout),checkoutBytes:checkout.length,
      checkoutMatchesGit:blob.equals(checkout)});
  }
  return inputs;
}
export async function collectNativeCargoMetadata(){
  check(!process.env.RUSTFLAGS&&!process.env.CARGO_ENCODED_RUSTFLAGS,
    'no inherited Rust compiler flags');
  const revision=run('git',['rev-parse','HEAD']).stdout.trim();
  check(COMMIT.test(revision),'Git revision');
  const rustc=run('rustc',['-vV']).stdout.trim();
  const field=name=>{
    const m=rustc.match(new RegExp('^'+name+': ([^\\r\\n]+)$','m'));
    check(m,'Rust metadata '+name);
    return m[1];
  };
  const initial=await snapshot();
  const directory=await mkdtemp(join(tmpdir(),'grunkalunka-p1-cargo-args-'));
  const target=join(directory,'target');
  const guest=join(target,GUEST);
  const modes={};
  try{
    for(const [mode,flags] of Object.entries(MODES)){
      const env={...process.env,CARGO_TARGET_DIR:target,
        CARGO_INCREMENTAL:'0',CARGO_TERM_COLOR:'never',RUSTFLAGS:flags};
      delete env.CARGO_ENCODED_RUSTFLAGS;
      if(mode==='fixed'){
        run('cargo',['clean','--target','wasm32-unknown-unknown'],env);
        let removed=false;
        try{await stat(guest)}catch(e){
          if(e.code!=='ENOENT')throw e;
          removed=true;
        }
        check(removed,'clean removed previous Markdown Wasm');
      }
      const result=run('cargo',BUILD,env);
      const entries=parseVerboseRustcInvocations(result.stderr.toString('utf8'));
      const bytes=await readFile(guest);
      check(bytes.length>=8&&bytes.length<=MAX_WASM&&
        WebAssembly.validate(bytes),'actual valid bounded Wasm');
      modes[mode]={requestedRustflags:flags,
        artifact:{bytes:bytes.length,sha256:hash(bytes)},
        invocations:entries};
      check(isDeepStrictEqual(await snapshot(),initial),
        'canonical checkout stayed unchanged');
    }
    check(run('rustc',['-vV']).stdout.trim()===rustc,
      'stable native toolchain');
    return validateReport({
      schemaVersion:1,workload:'p1-native-cargo-metadata',revision,
      environment:{os:platform(),arch:arch(),node:process.version,
        rustc:rustc.split(/\r?\n/)[0],rustcHost:field('host'),
        rustcCommit:field('commit-hash'),llvm:field('LLVM version'),
        target:'wasm32-unknown-unknown',cargoIncremental:'0',
        buildCommand:['cargo',...BUILD]},
      inputs:initial,modes,
    });
  }finally{
    await rm(directory,{recursive:true,force:true});
  }
}

if(process.argv[1]&&pathToFileURL(resolve(process.argv[1])).href===import.meta.url){
  try{
    const [command,...args]=process.argv.slice(2);
    let report;
    if(command==='collect'&&args.length===0)report=await collectNativeCargoMetadata();
    else if(command==='compare'&&args.length===2){
      const reports=await Promise.all(args.map(async path=>{
        const s=await stat(resolve(path));
        check(s.isFile()&&s.size>0&&s.size<=MAX_RECEIPT,
          'bounded native Cargo metadata receipt');
        return JSON.parse(await readFile(resolve(path),'utf8'));
      }));
      report=compareNativeCargoMetadata(...reports);
    }else throw new TypeError('usage: native-cargo-metadata.mjs collect | compare LINUX.json RECEIVER.json');
    console.log(JSON.stringify(report,null,2));
  }catch(error){console.error(error.stack??String(error));process.exitCode=1;}
}
