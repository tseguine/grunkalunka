// Preserve the P1 runner-budget cleanup: canonical tests may select new
// regressions, but new experimental P1 workflow files must not silently
// restore independently triggered PR/push jobs. Manual-only controls remain
// possible under AGENTS.md's documented exception policy.
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const workflows=fileURLToPath(new URL('../../../.github/workflows/',import.meta.url));

// Intentionally accept only the current plain YAML event mapping. New YAML
// spellings must be consciously reviewed, not treated as a safe no-op.
function declaredEvents(yaml){
  const lines=yaml.replace(/\r\n/g,'\n').split('\n');
  const starts=lines.flatMap((line,i)=>/^on:\s*(?:#.*)?$/.test(line)?[i]:[]);
  assert.equal(starts.length,1,'explicit unique top-level on: mapping required');
  const events=[];
  for(let i=starts[0]+1;i<lines.length;i++){
    const line=lines[i];
    if(!line.trim()||/^\s*#/.test(line))continue;
    if(!/^\s/.test(line))break;
    if(/^  \S/.test(line)){
      const match=/^  ([a-z][a-z0-9_-]*):(?:\s.*)?$/.exec(line);
      assert.ok(match,'unsupported workflow event syntax');
      events.push(match[1]);
    }
  }
  assert.ok(events.length>0,'nonempty workflow event mapping required');
  return events.sort();
}
function assertManualOnly(yaml,name){
  assert.deepEqual(declaredEvents(yaml),['workflow_dispatch'],
    name+' must remain manual-only; P1 unit tests belong in canonical CI');
}

test('P1 workflows cannot silently restore automatic independent CI fanout',()=>{
  for(const name of readdirSync(workflows).filter(x=>/^p1-.*\.ya?ml$/.test(x))){
    assertManualOnly(readFileSync(join(workflows,name),'utf8'),name);
  }
});

test('workflow guard rejects PR/push/schedule and unsupported inline triggers',()=>{
  for(const on of [
    'on:\n  pull_request:\n',
    'on:\n  push:\n',
    'on:\n  workflow_dispatch:\n  pull_request:\n',
    'on:\n  workflow_dispatch:\n  schedule:\n    - cron: \'0 0 * * *\'\n',
    'on: [workflow_dispatch, pull_request]\n',
  ])assert.throws(()=>assertManualOnly('name: test\n'+on+'jobs:\n  check:\n', 'synthetic'));
  assert.doesNotThrow(()=>assertManualOnly(
    'name: manual\non:\n  workflow_dispatch:\n    inputs:\n      profile:\n        required: true\njobs:\n  check:\n',
    'synthetic'));
});
