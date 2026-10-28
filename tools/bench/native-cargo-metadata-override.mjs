// P1: native Cargo rustc wrapper control over generated -C metadata.
// All modes run with the same wrapper path and original source/target directory.
// Only that generated metadata argument is substituted; Cargo's
// -C extra-filename, --extern, target and all other flags remain unchanged.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import {
  observeMarkdownGuest, compareMarkdownGuests,
} from './wasm-section-fingerprint.mjs';

const root=fileURLToPath(new URL('../../',import.meta.url));
const TARGET='wasm32-unknown-unknown';
const SHA=/^[a-f0-9]{64}$/;
const GIT=/^[a-f0-9]{40}$/;
const MAX_WASM=16*1024*1024;
const MAX_RECEIPT=768*1024;
const BUILD=[
  'build','--locked','--offline','-j','1',
  '-p','grunkalunka-markdown','--example','outline_guest',
  '--target',TARGET,
];
const WORKLOAD='p1-native-cargo-metadata-override';
const VARIANTS=['passthrough','target','all'];
const NAMES=Object.freeze({
  build_script_build:{target:'host',effective:'p1script'},
  unicase:{target:TARGET,effective:'p1unicase'},
  pulldown_cmark:{target:TARGET,effective:'p1pulldown'},
  grunkalunka_markdown:{target:TARGET,effective:'p1markdown'},
  outline_guest:{target:TARGET,effective:'p1guest'},
});
const SOURCES=[
  'Cargo.toml','Cargo.lock','rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
  'tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs',
];
const RUST_WRAPPER='tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs';
const check=(condition,label)=>{
  if(!condition)throw new TypeError('Invalid '+label);
};
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');

function run(command,args,env=process.env,binary=false) {
  const response=spawnSync(command,args,{
    cwd:root,env,windowsHide:true,shell:false,
    encoding:binary?null:'utf8',timeout:300_000,
    maxBuffer:4*1024*1024,stdio:['ignore','pipe','pipe'],
  });
  if(response.error||response.signal||response.status!==0){
    throw new Error('Failed '+command+' '+args.join(' ')+': '+
      (response.error?.message??
        (response.stderr?.toString('utf8').trim()||
         response.stdout?.toString('utf8').slice(-3500))??
        response.signal??response.status));
  }
  return response.stdout;
}

async function snapshot() {
  const inputs=[];
  for(const path of SOURCES){
    const original=run('git',['show','HEAD:'+path],process.env,true);
    const checkout=await readFile(resolve(root,path));
    check(original.length>0&&original.length<=8*1024*1024,
      'bounded original Git source '+path);
    inputs.push({path,gitSha256:digest(original),gitBytes:original.length,
      checkoutSha256:digest(checkout),checkoutBytes:checkout.length,
      checkoutMatchesGit:original.equals(checkout)});
  }
  return inputs;
}

function effective(mode,name,original){
  const key=NAMES[name];
  check(key,'expected native/wasm32 Rust crate '+name);
  return mode==='all'||(mode==='target'&&key.target===TARGET)?
    key.effective:original;
}

export function parseMetadataOverrideLog(raw,mode) {
  check(VARIANTS.includes(mode)&&typeof raw==='string'&&raw.length>0&&
    raw.length<=16384,'bounded rustc wrapper metadata log');
  const rows=[];
  for(const line of raw.trim().split(/\r?\n/)){
    const fields=line.split('\t');
    check(fields.length===4,'four rustc wrapper metadata log fields');
    const [crate,target,original,observed]=fields;
    const selected=Object.hasOwn(NAMES,crate);
    check(/^[A-Za-z_][A-Za-z0-9_]*$/.test(crate)&&
      (target===TARGET||target==='host')&&
      (!selected||target===NAMES[crate].target)&&
      /^[0-9a-f]{16}$/.test(original)&&
      /^[a-z0-9]+$/.test(observed)&&
      observed===(selected?effective(mode,crate,original):original),
    'actual selected/untouched compiler metadata for '+crate);
    rows.push({crate,target,original,effective:observed,changed:original!==observed});
  }
  rows.sort((a,b)=>{
    const left=a.crate+'|'+a.target+'|'+a.original;
    const right=b.crate+'|'+b.target+'|'+b.original;
    return left<right?-1:left>right?1:0;
  });
  const selected=rows.filter(x=>Object.hasOwn(NAMES,x.crate));
  const names=Object.keys(NAMES).sort();
  check(rows.length>=names.length&&rows.length<=128&&
    selected.length===names.length&&
    selected.every((row,i)=>row.crate===names[i]),
  'one real rustc invocation per expected crate');
  return rows;
}

function validateSources(inputs){
  check(Array.isArray(inputs)&&inputs.length===SOURCES.length,
    'complete observed Git source files');
  for(let i=0;i<inputs.length;i++){
    const v=inputs[i];
    check(v&&v.path===SOURCES[i]&&SHA.test(v.gitSha256)&&
      SHA.test(v.checkoutSha256)&&Number.isSafeInteger(v.gitBytes)&&
      v.gitBytes>0&&Number.isSafeInteger(v.checkoutBytes)&&v.checkoutBytes>0&&
      v.checkoutMatchesGit===
        (v.gitSha256===v.checkoutSha256&&v.gitBytes===v.checkoutBytes),
    'tracked Git source identity '+SOURCES[i]);
  }
}
function validateHost(report){
  check(report&&report.schemaVersion===1&&report.workload===WORKLOAD&&
    GIT.test(report.revision),'native Cargo wrapper report');
  check(report.environment&&['linux','win32','darwin'].includes(report.environment.os)&&
    ['x64','arm64'].includes(report.environment.arch)&&
    typeof report.environment.node==='string'&&
    /^rustc \d+\.\d+\.\d+/.test(report.environment.rustc)&&
    GIT.test(report.environment.rustcCommit)&&
    typeof report.environment.llvm==='string'&&
    report.environment.llvm.length>0&&
    typeof report.environment.rustcHost==='string'&&
    report.environment.target===TARGET&&
    report.environment.cargoIncremental==='0'&&
    report.environment.wrapperSourceSha256===report.inputs[6]?.gitSha256&&
    report.environment.cargoCommand?.join('\0')===['cargo',...BUILD].join('\0')&&
    report.environment.privateWrapperPolicy==='one fixed wrapper executable and absolute target directory, complete clean between variants',
  'native Rust compiler and wrapper controls');
  validateSources(report.inputs);
  check(report.variants&&
    Object.keys(report.variants).sort().join(',')==='all,passthrough,target',
  'all three independent wrapper modes');
  const expectedInputs=new Map(report.inputs.map(x=>[x.path,x]));
  const originalByCrate=new Map();
  for(const mode of VARIANTS){
    const record=report.variants[mode];
    check(record&&record.mode===mode&&
      Array.isArray(record.actualMetadata)&&
      record.actualMetadata.length>=Object.keys(NAMES).length&&
      record.actualMetadata.length<=128&&
      record.report&&record.report.schemaVersion===1&&
      record.report.workload==='p1-markdown-guest-wasm-sections'&&
      record.report.revision===report.revision&&
      record.report.artifact&&SHA.test(record.report.artifact.sha256)&&
      Number.isSafeInteger(record.report.artifact.bytes)&&
      record.report.artifact.bytes>=8&&
      record.report.artifact.bytes<=MAX_WASM&&
      record.report.environment?.os===report.environment.os&&
      record.report.environment?.arch===report.environment.arch&&
      record.report.environment?.rustc===report.environment.rustc&&
      record.report.environment?.node===report.environment.node&&
      record.report.environment?.target===TARGET&&
      record.report.environment?.rustflags===''&&
      record.report.environment?.cargoEncodedRustflags===''&&
      Array.isArray(record.report.inputs)&&
      record.report.inputs.every(x=>
        isDeepStrictEqual(x,expectedInputs.get(x.path))),
    'complete observed Wasm receipt for '+mode);
    const names=Object.keys(NAMES).sort();
    const selected=record.actualMetadata.filter(x=>Object.hasOwn(NAMES,x.crate));
    check(selected.length===names.length&&
      selected.every((row,i)=>row.crate===names[i]),
    'all five selected compiler metadata records in '+mode);
    for(const row of record.actualMetadata){
      const crate=row.crate;
      const selectedCrate=Object.hasOwn(NAMES,crate);
      const isSelectedTarget=selectedCrate&&row.target===NAMES[crate].target;
      check(/^[A-Za-z_][A-Za-z0-9_]*$/.test(crate)&&
        (row.target==='host'||row.target===TARGET)&&
        (!selectedCrate||isSelectedTarget)&&
        /^[0-9a-f]{16}$/.test(row.original)&&
        row.effective===(selectedCrate?
          effective(mode,crate,row.original):row.original)&&
        row.changed===(row.original!==row.effective),
      'complete observed generated metadata for '+mode+'/'+crate);
      if(selectedCrate){
        if(originalByCrate.has(crate)){
          check(originalByCrate.get(crate)===row.original,
            'unchanged Cargo-generated metadata across wrapper modes for '+crate);
        }else originalByCrate.set(crate,row.original);
      }
    }
    // Existing inspector validates bounded data/section and full body SHA-256
    // vectors. Self-compare requires no host identity assumption.
    const identity=compareMarkdownGuests(record.report,record.report);
    check(identity.sameArtifactBytes&&
      identity.codeBodyComparison!==null,
    'self-consistent actual core Wasm fingerprint for '+mode);
  }
  return report;
}

function summary(a,b){
  const result=compareMarkdownGuests(a,b);
  const bodies=result.codeBodyComparison;
  check(bodies!==null,'complete native Wasm code-body vectors');
  return {
    sameArtifactBytes:result.sameArtifactBytes,
    changedSections:result.changedSections.map(x=>x.section),
    functions:[bodies.producerCount,bodies.receiverCount],
    matchingAtOrdinal:bodies.matchingAtSameOrdinal,
    matchingIgnoringOrder:bodies.matchingBodyOccurrencesIgnoringOrder,
    unmatched:[bodies.producerUnmatchedBodyOccurrences,bodies.receiverUnmatchedBodyOccurrences],
  };
}
export function compareNativeMetadataOverrides(linux,receiver) {
  validateHost(linux);validateHost(receiver);
  check(linux.environment.os==='linux'&&linux.environment.arch==='x64'&&
    receiver.environment.os!=='linux',
  'Linux x64 producer and independent native receiver');
  check(linux.revision===receiver.revision&&
    isDeepStrictEqual(
      linux.inputs.map(x=>[x.path,x.gitSha256,x.gitBytes]),
      receiver.inputs.map(x=>[x.path,x.gitSha256,x.gitBytes])),
  'identical tested revision and canonical Git bytes');
  check(linux.environment.rustc===receiver.environment.rustc&&
    linux.environment.rustcCommit===receiver.environment.rustcCommit&&
    linux.environment.llvm===receiver.environment.llvm&&
    linux.environment.node===receiver.environment.node,
  'same Rust/LLVM/Node versions');
  const metadata={};
  const crossHost={};
  for(const mode of VARIANTS){
    const left=linux.variants[mode],right=receiver.variants[mode];
    const selectedLeft=left.actualMetadata.filter(x=>Object.hasOwn(NAMES,x.crate));
    const selectedRight=right.actualMetadata.filter(x=>Object.hasOwn(NAMES,x.crate));
    let generatedSame=0,effectiveSame=0;
    const mismatches=[];
    for(let i=0;i<selectedLeft.length;i++){
      const l=selectedLeft[i],r=selectedRight[i];
      check(l.crate===r.crate&&l.target===r.target,
        'comparable crate targets at the same ordinal');
      if(l.original===r.original)generatedSame++;
      if(l.effective===r.effective)effectiveSame++;
      else mismatches.push(l.crate);
    }
    metadata[mode]={
      generatedSame,effectiveSame,totalCrates:selectedLeft.length,
      remainingMetadataMismatches:mismatches,
      producerModifiedCrates:selectedLeft.filter(x=>x.changed).map(x=>x.crate),
      receiverModifiedCrates:selectedRight.filter(x=>x.changed).map(x=>x.crate),
      producerUnselectedCrates:left.actualMetadata.filter(x=>!Object.hasOwn(NAMES,x.crate)).map(x=>x.crate),
      receiverUnselectedCrates:right.actualMetadata.filter(x=>!Object.hasOwn(NAMES,x.crate)).map(x=>x.crate),
    };
    crossHost[mode]=summary(left.report,right.report);
  }
  return {
    schemaVersion:1,workload:'p1-native-cargo-metadata-override-comparison',
    revision:linux.revision,
    producer:{...linux.environment},receiver:{...receiver.environment},
    metadata,
    crossHost,
    withinHost:{
      producer:{
        passthroughToTarget:summary(linux.variants.passthrough.report,linux.variants.target.report),
        targetToAll:summary(linux.variants.target.report,linux.variants.all.report),
      },
      receiver:{
        passthroughToTarget:summary(receiver.variants.passthrough.report,receiver.variants.target.report),
        targetToAll:summary(receiver.variants.target.report,receiver.variants.all.report),
      },
    },
    artifacts:{
      producer:Object.fromEntries(VARIANTS.map(mode=>[mode,{...linux.variants[mode].report.artifact}])),
      receiver:Object.fromEntries(VARIANTS.map(mode=>[mode,{...receiver.variants[mode].report.artifact}])),
    },
    interpretation:'selected five Cargo-generated metadata groups overridden under one wrapper; other actual compiled dependencies, extra-filename and all other Cargo inputs remain host-dependent',
  };
}

export async function collectNativeMetadataOverrides() {
  check(!process.env.RUSTFLAGS&&!process.env.CARGO_ENCODED_RUSTFLAGS&&
    !process.env.RUSTC_WRAPPER&&
    !process.env.RUSTC_WORKSPACE_WRAPPER,
  'no preexisting Rust compiler flags or wrappers');
  const revision=run('git',['rev-parse','HEAD']).trim();
  check(GIT.test(revision),'exact Git revision');
  const rustc=run('rustc',['-vV']).trim();
  const field=name=>{
    const m=rustc.match(new RegExp('^'+name+': ([^\\r\\n]+)$','m'));
    check(m,'native Rust metadata '+name);
    return m[1];
  };
  const initial=await snapshot();
  const directory=await mkdtemp(join(tmpdir(),'grunkalunka-p1-rustc-metadata-wrapper-'));
  const target=join(directory,'target');
  const log=join(directory,'wrapper-records.tsv');
  const exe=join(directory,platform()==='win32'?'p1-wrapper.exe':'p1-wrapper');
  const guest=join(target,'wasm32-unknown-unknown/debug/examples/outline_guest.wasm');
  const variants={};
  try{
    run('rustc',['--edition=2024','-C','opt-level=0',
      '--crate-name','p1_wrapper','-o',exe,resolve(root,RUST_WRAPPER)]);
    run('rustc',['--test','--edition=2024','--crate-name',
      'p1_wrapper_tests','-o',
      join(directory,platform()==='win32'?'p1-wrapper-tests.exe':'p1-wrapper-tests'),
      resolve(root,RUST_WRAPPER)]);
    run(join(directory,platform()==='win32'?'p1-wrapper-tests.exe':'p1-wrapper-tests'),[]);
    for(const mode of VARIANTS){
      const env={...process.env,
        CARGO_TARGET_DIR:target,CARGO_INCREMENTAL:'0',CARGO_TERM_COLOR:'never',
        RUSTFLAGS:'',RUSTC_WRAPPER:exe,
        P1_WRAPPER_MODE:mode,P1_WRAPPER_LOG:log,
      };
      delete env.CARGO_ENCODED_RUSTFLAGS;
      if(mode!=='passthrough'){
        // Fully remove private host and Wasm products, including the native
        // build script; a --target-only clean would not rebuild that script.
        run('cargo',['clean'],env);
        let missing=false;
        try{await stat(guest)}catch(e){
          if(e.code!=='ENOENT')throw e;
          missing=true;
        }
        check(missing,'cleaned private Cargo target before '+mode);
      }
      await writeFile(log,'');
      run('cargo',BUILD,env);
      const source=await readFile(log,'utf8');
      const actualMetadata=parseMetadataOverrideLog(source,mode);
      const compiled=await readFile(guest);
      check(compiled.length>=8&&compiled.length<=MAX_WASM&&
        WebAssembly.validate(compiled),'bounded actual Wasm after '+mode);
      const report=await observeMarkdownGuest(guest);
      check(report.revision===revision&&
        report.artifact.sha256===digest(compiled)&&
        report.artifact.bytes===compiled.length&&
        report.environment.rustc===rustc.split(/\r?\n/)[0],
      'actual compiled Wasm and native Rust toolchain');
      report.environment.rustflags='';
      report.environment.cargoEncodedRustflags='';
      variants[mode]={mode,actualMetadata,report};
      check(isDeepStrictEqual(await snapshot(),initial),
        'tracked source not modified during '+mode);
    }
    check(run('rustc',['-vV']).trim()===rustc,'stable native toolchain across modes');
    return validateHost({
      schemaVersion:1,workload:WORKLOAD,revision,
      environment:{
        os:platform(),arch:arch(),node:process.version,
        rustc:rustc.split(/\r?\n/)[0],rustcCommit:field('commit-hash'),
        rustcHost:field('host'),llvm:field('LLVM version'),target:TARGET,
        cargoIncremental:'0',cargoCommand:['cargo',...BUILD],
        wrapperSourceSha256:initial[6].gitSha256,
        privateWrapperPolicy:'one fixed wrapper executable and absolute target directory, complete clean between variants',
      },
      inputs:initial,variants,
    });
  }finally{
    await rm(directory,{recursive:true,force:true});
  }
}

if(process.argv[1]&&pathToFileURL(resolve(process.argv[1])).href===import.meta.url){
  try{
    const [mode,...args]=process.argv.slice(2);
    let data;
    if(mode==='collect'&&args.length===0)data=await collectNativeMetadataOverrides();
    else if(mode==='compare'&&args.length===2){
      const reports=await Promise.all(args.map(async file=>{
        const info=await stat(resolve(file));
        check(info.isFile()&&info.size>0&&info.size<=MAX_RECEIPT,
          'bounded native metadata override JSON');
        return JSON.parse(await readFile(resolve(file),'utf8'));
      }));
      data=compareNativeMetadataOverrides(...reports);
    }else throw new TypeError('usage: native-cargo-metadata-override.mjs collect | compare LINUX.json RECEIVER.json');
    console.log(JSON.stringify(data,null,2));
  }catch(error){
    console.error(error.stack??String(error));
    process.exitCode=1;
  }
}
