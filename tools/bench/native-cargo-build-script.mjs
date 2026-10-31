// P1: observe Cargo build-script outputs across native hosts, not raw logs
// or absolute OUT_DIR values. Source/dependency archive bytes are not changed.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstat, mkdtemp, readFile, readdir, rm, stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { arch, platform, tmpdir } from 'node:os';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const root=fileURLToPath(new URL('../../',import.meta.url));
const SHA=/^[a-f0-9]{64}$/;
const COMMIT=/^[a-f0-9]{40}$/;
const MAX_STDOUT=16*1024*1024;
const MAX_FILE_BYTES=4*1024*1024;
const MAX_TOTAL_BYTES=8*1024*1024;
const MAX_FILES=512;
const MAX_SCRIPTS=16;
const MAX_RECEIPT=192*1024;
const GUEST='wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const BUILD=['build','--message-format=json','--locked','--offline','-j','4',
  '-p','grunkalunka-markdown','--example','outline_guest',
  '--target','wasm32-unknown-unknown'];
const required=(ok,label)=>{if(!ok)throw new TypeError('Invalid '+label)};
const sha256=bytes=>createHash('sha256').update(bytes).digest('hex');

function run(cmd,args,env=process.env){
  const p=spawnSync(cmd,args,{
    cwd:root,env,windowsHide:true,shell:false,encoding:'utf8',
    timeout:300_000,maxBuffer:MAX_STDOUT,stdio:['ignore','pipe','pipe'],
  });
  if(p.error||p.signal||p.status!==0){
    throw new Error('Failed '+cmd+' '+args.join(' ')+': '+
      (p.error?.message??p.stderr?.slice(-2000)??p.signal??p.status));
  }
  return p.stdout;
}
function safePath(path){
  return typeof path==='string'&&path.length>0&&
    !path.startsWith('/')&&!path.includes('\\')&&
    !path.split('/').some(x=>x===''||x==='.'||x==='..');
}
function shaList(values){
  required(Array.isArray(values)&&values.length<=128&&
    values.every(x=>typeof x==='string'&&x.length<=2048),
  'bounded Cargo build-script value list');
  return values.map(s=>sha256(Buffer.from(s))).sort();
}

// Extract structured Cargo JSON, never raw process output or OUT_DIR paths.
export function parseCargoScriptEvents(output){
  required(typeof output==='string'&&output.length>0&&
    output.length<=MAX_STDOUT,'bounded Cargo JSON message output');
  const messages=[];
  let finished=false;
  for(const line of output.split(/\r?\n/)){
    if(!line.startsWith('{'))continue;
    const message=JSON.parse(line);
    if(message.reason==='build-script-executed'){
      required(typeof message.package_id==='string'&&message.package_id.length>0&&
        typeof message.out_dir==='string'&&message.out_dir.length>0&&
        Array.isArray(message.cfgs)&&Array.isArray(message.env)&&
        Array.isArray(message.linked_libs)&&Array.isArray(message.linked_paths),
      'Cargo build-script-executed payload');
      required(message.env.length<=128&&
        message.env.every(pair=>Array.isArray(pair)&&pair.length===2&&
          typeof pair[0]==='string'&&pair[0].length<=128&&
          /^[A-Za-z_][A-Za-z0-9_]*$/.test(pair[0])&&
          typeof pair[1]==='string'&&pair[1].length<=4096),
      'bounded Cargo build-script environment');
      messages.push({
        packageId:message.package_id,outDir:message.out_dir,
        cfgHashes:shaList(message.cfgs),
        env:message.env.map(([key,value])=>({key,valueSha256:sha256(Buffer.from(value))}))
          .sort((a,b)=>a.key.localeCompare(b.key,'en')),
        linkLibraryHashes:shaList(message.linked_libs),
        linkPathHashes:shaList(message.linked_paths),
      });
      required(messages.length<=MAX_SCRIPTS,'bounded number of Cargo build scripts');
    }else if(message.reason==='build-finished'){
      required(message.success===true,'successful Cargo build');
      finished=true;
    }
  }
  required(finished&&messages.length>0,
    'finished actual Cargo build with build-script output events');
  return messages;
}
async function fileFingerprints(folder){
  const records=[];
  let total=0;
  // Keep the exact recursive DFS order (including quota failures), but own
  // the directory continuation on the heap rather than the JS call stack.
  const pending=[{dir:folder,depth:0,entries:null,index:0}];
  while(pending.length){
    const frame=pending[pending.length-1];
    if(frame.entries===null){
      required(frame.depth<=6,'bounded build-script output directory depth');
      frame.entries=await readdir(frame.dir,{withFileTypes:true});
      required(frame.entries.length<=MAX_FILES,'bounded build-script directory entries');
    }
    if(frame.index===frame.entries.length){
      pending.pop();
      continue;
    }
    const item=frame.entries[frame.index++];
    const full=join(frame.dir,item.name);
    const info=await lstat(full);
    required(!info.isSymbolicLink(),'no symlink generated build-script files');
    if(info.isDirectory()){
      pending.push({dir:full,depth:frame.depth+1,entries:null,index:0});
    }else if(info.isFile()){
      const path=relative(folder,full).split(sep).join('/');
      required(safePath(path)&&info.size<=MAX_FILE_BYTES&&
        total+info.size<=MAX_TOTAL_BYTES&&records.length<MAX_FILES,
      'bounded build-script file bytes/path');
      const digest=createHash('sha256');
      let length=0;
      for await(const chunk of createReadStream(full)){
        length+=chunk.length;
        required(length<=MAX_FILE_BYTES,'bounded generated data stream');
        digest.update(chunk);
      }
      required(length===info.size,'stable generated file size');
      records.push({path,bytes:length,sha256:digest.digest('hex')});
      total+=length;
    }
  }
  records.sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);
  return {files:records,totalBytes:total};
}

export function compareScriptOutputs(producer,receiver){
  const validate=report=>{
    required(report&&report.schemaVersion===1&&
      report.workload==='p1-native-cargo-build-script-outputs'&&
      COMMIT.test(report.revision)&&report.environment&&
      ['linux','win32','darwin'].includes(report.environment.os)&&
      ['x64','arm64'].includes(report.environment.arch)&&
      /^rustc \d+\.\d+\.\d+/.test(report.environment.rustc)&&
      COMMIT.test(report.environment.rustcCommit)&&
      typeof report.environment.llvm==='string'&&report.environment.llvm.length>0&&
      report.environment.buildCommand?.join('\0')===['cargo',...BUILD].join('\0')&&
      report.guest&&SHA.test(report.guest.sha256)&&
      Number.isSafeInteger(report.guest.bytes)&&report.guest.bytes>8,
    'native Cargo build-script evidence receipt');
    required(Array.isArray(report.scripts)&&report.scripts.length>0&&
      report.scripts.length<=MAX_SCRIPTS,
    'bounded build-script receipt count');
    let last='';
    for(const script of report.scripts){
      required(script&&typeof script.package==='string'&&
        /^[A-Za-z0-9_-]+@[A-Za-z0-9.+_-]+$/.test(script.package)&&
        script.package>last&&
        Array.isArray(script.files)&&script.files.length<=MAX_FILES&&
        Number.isSafeInteger(script.totalBytes)&&
        script.totalBytes>=0&&script.totalBytes<=MAX_TOTAL_BYTES&&
        Array.isArray(script.cfgHashes)&&Array.isArray(script.env)&&
        Array.isArray(script.linkLibraryHashes)&&
        Array.isArray(script.linkPathHashes),
      'sorted package build-script inventory');
      let size=0,prior='';
      for(const file of script.files){
        required(file&&safePath(file.path)&&file.path>prior&&
          Number.isSafeInteger(file.bytes)&&file.bytes>=0&&file.bytes<=MAX_FILE_BYTES&&
          SHA.test(file.sha256),'bounded generated file identity');
        prior=file.path;
        size+=file.bytes;
      }
      required(size===script.totalBytes,'generated file inventory byte total');
      for(const hashes of [script.cfgHashes,script.linkLibraryHashes,script.linkPathHashes]){
        required(hashes.length<=128&&hashes.every(x=>SHA.test(x)),
          'bounded build-script cfg/link hashes');
      }
      required(script.env.length<=128&&script.env.every(x=>
        /^[A-Za-z_][A-Za-z0-9_]*$/.test(x.key)&&SHA.test(x.valueSha256)),
      'bounded build-script env value hashes');
      last=script.package;
    }
    return report;
  };
  validate(producer);validate(receiver);
  required(producer.environment.os==='linux'&&producer.environment.arch==='x64'&&
    receiver.environment.os!=='linux','native Linux producer/receiver');
  required(producer.revision===receiver.revision&&
    producer.environment.rustc===receiver.environment.rustc&&
    producer.environment.rustcCommit===receiver.environment.rustcCommit&&
    producer.environment.llvm===receiver.environment.llvm,
  'same tested revision and native compiler release');
  const a=new Map(producer.scripts.map(x=>[x.package,x]));
  const b=new Map(receiver.scripts.map(x=>[x.package,x]));
  const packages=[...new Set([...a.keys(),...b.keys()])].sort();
  const findings=packages.map(packageName=>{
    const left=a.get(packageName),right=b.get(packageName);
    if(!left||!right)return{package:packageName,missingOn:!left?'producer':'receiver'};
    const first=new Map(left.files.map(x=>[x.path,x]));
    const second=new Map(right.files.map(x=>[x.path,x]));
    const changedFiles=[...new Set([...first.keys(),...second.keys()])].sort()
      .filter(path=>!isDeepStrictEqual(first.get(path)??null,second.get(path)??null))
      .map(path=>({path,producer:first.get(path)??null,receiver:second.get(path)??null}));
    return {
      package:packageName,
      sameGeneratedFiles:changedFiles.length===0,
      generatedFiles:[left.files.length,right.files.length],
      generatedBytes:[left.totalBytes,right.totalBytes],
      changedFiles,
      sameCfgHashes:isDeepStrictEqual(left.cfgHashes,right.cfgHashes),
      sameEnvHashes:isDeepStrictEqual(left.env,right.env),
      sameLinkLibraryHashes:isDeepStrictEqual(left.linkLibraryHashes,right.linkLibraryHashes),
      sameLinkPathHashes:isDeepStrictEqual(left.linkPathHashes,right.linkPathHashes),
    };
  });
  return {
    schemaVersion:1,workload:'p1-native-cargo-build-script-comparison',
    revision:producer.revision,
    producer:{...producer.environment,guest:producer.guest},
    receiver:{...receiver.environment,guest:receiver.guest},
    findings,
    interpretation:'Cargo JSON build-script outputs and generated file hashes; neither complete compiler inputs nor proof of cause',
  };
}

export async function collectNativeBuildScriptOutputs(){
  required(!process.env.RUSTFLAGS&&!process.env.CARGO_ENCODED_RUSTFLAGS,
    'no inherited compiler flags');
  const revision=run('git',['rev-parse','HEAD']).trim();
  required(COMMIT.test(revision),'tested Git revision');
  const rustc=run('rustc',['-vV']).trim();
  const field=name=>{
    const m=rustc.match(new RegExp('^'+name+': ([^\\r\\n]+)$','m'));
    required(m,'native Rust version '+name);
    return m[1];
  };
  const metadata=JSON.parse(run('cargo',['metadata','--format-version','1','--offline','--locked']));
  required(Array.isArray(metadata.packages),'resolved Cargo metadata package list');
  const packages=new Map(metadata.packages.map(x=>[x.id,x.name+'@'+x.version]));
  const directory=await mkdtemp(join(tmpdir(),'grunkalunka-p1-build-script-'));
  const target=join(directory,'target');
  const env={...process.env,CARGO_TARGET_DIR:target,
    CARGO_INCREMENTAL:'0',CARGO_TERM_COLOR:'never',RUSTFLAGS:''};
  delete env.CARGO_ENCODED_RUSTFLAGS;
  try{
    const events=parseCargoScriptEvents(run('cargo',BUILD,env));
    const scripts=[];
    for(const event of events){
      const packageName=packages.get(event.packageId);
      required(packageName,'resolved Cargo package ID for executed build script');
      const output=resolve(event.outDir);
      const rel=relative(target,output);
      required(rel&&rel!=='.'&&!rel.startsWith('..')&&
        !isAbsolute(rel),'generated Cargo OUT_DIR inside private target');
      const info=await stat(output);
      required(info.isDirectory(),'actual Cargo generated output directory');
      const files=await fileFingerprints(output);
      scripts.push({package:packageName,
        ...files,cfgHashes:event.cfgHashes,env:event.env,
        linkLibraryHashes:event.linkLibraryHashes,
        linkPathHashes:event.linkPathHashes});
    }
    scripts.sort((a,b)=>a.package<b.package?-1:a.package>b.package?1:0);
    for(let i=1;i<scripts.length;i++)
      required(scripts[i].package!==scripts[i-1].package,
        'unambiguous package build-script output source');
    const artifact=await readFile(join(target,GUEST));
    required(artifact.length>8&&artifact.length<=16*1024*1024&&
      WebAssembly.validate(artifact),'actual valid compiled Markdown Wasm');
    required(run('rustc',['-vV']).trim()===rustc,'stable native Rust release');
    return{
      schemaVersion:1,workload:'p1-native-cargo-build-script-outputs',revision,
      environment:{os:platform(),arch:arch(),node:process.version,
        rustc:rustc.split(/\r?\n/)[0],rustcHost:field('host'),
        rustcCommit:field('commit-hash'),llvm:field('LLVM version'),
        buildCommand:['cargo',...BUILD]},
      guest:{bytes:artifact.length,sha256:sha256(artifact)},scripts,
    };
  }finally{
    await rm(directory,{recursive:true,force:true});
  }
}

if(process.argv[1]&&pathToFileURL(resolve(process.argv[1])).href===import.meta.url){
  try{
    const [mode,...args]=process.argv.slice(2);
    let result;
    if(mode==='collect'&&args.length===0)result=await collectNativeBuildScriptOutputs();
    else if(mode==='compare'&&args.length===2){
      const receipts=await Promise.all(args.map(async path=>{
        const info=await stat(resolve(path));
        required(info.isFile()&&info.size>0&&info.size<=MAX_RECEIPT,
          'bounded Cargo script receipt');
        return JSON.parse(await readFile(resolve(path),'utf8'));
      }));
      result=compareScriptOutputs(...receipts);
    }else throw new TypeError('usage: native-cargo-build-script.mjs collect | compare LINUX.json RECEIVER.json');
    console.log(JSON.stringify(result,null,2));
  }catch(error){console.error(error.stack??String(error));process.exitCode=1;}
}
