// Run after the offline workspace build. The report covers only this profile.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { valuePrograms, valueRejected } from './primitive-values.mjs';
import { rejected as numericRejected } from './corpus.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const compiler = path.join(root, 'target/debug/grunkalunka' + (process.platform === 'win32' ? '.exe' : ''));
const runs = path.join(root, 'target/value-compatibility-runs');
fs.mkdirSync(runs, {recursive: true});
const run = fs.mkdtempSync(path.join(runs, 'run-'));
process.on('uncaughtExceptionMonitor', () => console.error('Partial primitive profile artifacts: ' + run));
const started = performance.now(), sha = bytes => createHash('sha256').update(bytes).digest('hex');
function invoke(args, input = '', success = true) {
  assert.ok(performance.now() - started < 60_000, 'primitive profile total watchdog');
  const result = spawnSync(compiler, args, {
    cwd: root, input, encoding: 'utf8', shell: false, timeout: 10_000, maxBuffer: 1024 * 1024,
  });
  assert.ifError(result.error); assert.equal(result.signal, null);
  if (success) assert.equal(result.status, 0, result.stderr);
  else { assert.equal(result.status, 1, 'explicit error exit'); assert.match(result.stderr, /^ERROR:/); }
  report.invocations.push({args: args.map(arg => arg.startsWith(root) ? path.relative(root,arg) : arg),
    inputBytes: Buffer.byteLength(input), inputSha256: sha(input), status: result.status,
    stdoutSha256: sha(result.stdout), stderr: result.stderr});
  return result;
}
const buffer = new ArrayBuffer(8), view = new DataView(buffer);
function bits(x) { view.setFloat64(0, x); return view.getBigUint64(0).toString(16).padStart(16, '0'); }
function fromBits(hex) { view.setBigUint64(0, BigInt('0x' + hex)); return view.getFloat64(0); }
function observe(x) {
  if (x === undefined) return 'undefined';
  if (x === null) return 'null';
  if (typeof x === 'boolean') return 'boolean:' + Number(x);
  assert.equal(typeof x, 'number');
  return Number.isNaN(x) ? 'number:nan' : 'number:' + bits(x);
}
function decode([tag, payload]) {
  if (tag === 0 || tag === 1) { assert.ok(Object.is(payload, 0)); return tag === 0 ? undefined : null; }
  if (tag === 2) { assert.ok(Object.is(payload, 0) || payload === 1); return payload === 1; }
  assert.equal(tag, 3); return payload;
}
function observeLine(line) {
  if (['undefined', 'null', 'boolean:0', 'boolean:1'].includes(line)) return line;
  assert.match(line, /^number:[0-9a-f]{16}$/);
  return observe(fromBits(line.slice(7)));
}
function save(name, bytes) {
  fs.writeFileSync(path.join(run, name), bytes, {flag: 'wx'});
  return {path: name, sha256: sha(bytes)};
}
const vectors = [
  [0, 0], [-0, -0], [1, 7], [-1, 2], [2, -3], [NaN, 1], [1, NaN],
  [Infinity, 2], [-Infinity, Infinity], [Number.MAX_VALUE, -Number.MAX_VALUE],
  [Number.MIN_VALUE, 2 ** -1022], [-Number.MIN_VALUE, -0], [0.5, 0.25], [4294967297, -3],
];
const input = vectors.map(([a,b]) => bits(a) + ' ' + bits(b) + '\n').join('');
const report = {
  schemaVersion: 1, profile: 'primitive-return-v1', scope: 'finite selected cases; not full JS/TS conformance',
  compiler: {binarySha256: sha(fs.readFileSync(compiler)), cargoLockSha256: sha(fs.readFileSync(path.join(root, 'Cargo.lock')))},
  host: {node: process.version, platform: process.platform, arch: process.arch},
  fixtureSha256: sha(fs.readFileSync(new URL('./primitive-values.mjs', import.meta.url))),
  vectors: vectors.map(([a,b]) => [bits(a), bits(b)]),
  pairedPrograms: valuePrograms.length, rejectedPairs: valueRejected.length,
  comparisons: 0, fixtures: [], rejections: [], controls: [], invocations: [],
  numericOriginCases: 0,
};
const ids = new Set();
for (const fixture of valuePrograms) {
  const {id, sourceJS, sourceTS, standards} = fixture;
  assert.ok(!ids.has(id)); ids.add(id);
  assert.ok(standards.length > 0 && standards.every(url => url.startsWith('https://tc39.es/ecma262/')));
  const original = Function('"use strict";' + sourceJS.replace('export function', 'function') + ';return run;')();
  const reference = vectors.map(([a,b]) => observe(original(a,b)));
  const numericOrigin = fixture.numericOrigin === null ? null : numericRejected.find(row => row[0] === fixture.numericOrigin);
  if(fixture.numericOrigin !== null) { assert.ok(numericOrigin); assert.equal(sourceJS,numericOrigin[1]); report.numericOriginCases++; }
  const record = {id, standards, numericOrigin: fixture.numericOrigin, oracle: save(id + '.oracle.js', sourceJS), reference, variants: []};
  for (const [extension, source] of [['js', sourceJS], ['ts', sourceTS]]) {
    const name = id + '.' + extension, output = name + '.wasm';
    const sourceArtifact = save(name, source);
    const compiled = invoke(['compile-value', path.join(run, name), path.join(run, output)]);
    assert.match(compiled.stdout, /^COMPILED \d+ bytes:/);
    const wasm = fs.readFileSync(path.join(run, output));
    const wat = invoke(['wat-value', path.join(run, name)]).stdout;
    assert.match(wat, /\(result i32 f64\)/);
    const watArtifact = save(name + '.wat', wat);
    const instance = new WebAssembly.Instance(new WebAssembly.Module(wasm));
    assert.deepEqual(WebAssembly.Module.imports(new WebAssembly.Module(wasm)), []);
    const node = vectors.map(([a,b]) => observe(decode(instance.exports.run(a,b))));
    const raw = invoke(['eval-value', path.join(run, output)], input).stdout;
    assert.ok(raw.endsWith('\n'), 'one terminated line per result');
    const wasmtime = raw.slice(0,-1).split('\n').map(line => observeLine(line.replace(/\r$/, '')));
    assert.deepEqual(node, reference, id + ' ' + extension + ' Node-Wasm');
    assert.deepEqual(wasmtime, reference, id + ' ' + extension + ' Wasmtime');
    record.variants.push({extension, source: sourceArtifact, wat: watArtifact,
      wasm: {path: output, sha256: sha(wasm)}, raw: save(name + '.stdout', raw), node, wasmtime});
    report.comparisons += reference.length * 2;
  }
  if(numericOrigin) {
    const numericOutput = path.join(run,id+'.numeric-rejected.wasm');
    record.numericDiagnostic = invoke(['compile',path.join(run,id+'.js'),numericOutput],'',false).stderr.trimEnd();
    assert.ok(!fs.existsSync(numericOutput));
  }
  report.fixtures.push(record);
}
for (const {id, sourceJS, sourceTS, diagnostic, standards} of valueRejected) {
  for (const [extension, source] of [['js', sourceJS], ['ts', sourceTS]]) {
    const name = 'reject-' + id + '.' + extension, output = name + '.wasm';
    const artifact = save(name, source);
    const result = invoke(['compile-value', path.join(run, name), path.join(run, output)], '', false);
    assert.ok(result.stderr.includes(diagnostic), result.stderr);
    assert.equal(result.stdout, ''); assert.ok(!fs.existsSync(path.join(run, output)));
    report.rejections.push({id, extension, standards, source: artifact, diagnostic: result.stderr.trimEnd(), artifactProduced: false});
  }
}

// CLI controls are separate from language feature/rejection counts.
const numberSource = path.join(run, 'number.js'), numberWasm = path.join(run, 'numeric-control.wasm');
invoke(['compile', numberSource, numberWasm]);
assert.equal(invoke(['eval', numberWasm], '8000000000000000 0000000000000000\n').stdout, '8000000000000000\n');
invoke(['eval-value', numberWasm], '', false);
invoke(['eval', path.join(run, 'null.js.wasm')], '', false);
assert.equal(invoke(['compile'], '', false).stderr,
  'ERROR: usage: grunkalunka compile SOURCE.{js,ts} OUT.wasm | wat SOURCE | eval OUT.wasm < input-bits.txt\n');
assert.match(invoke(['wat-value'], '', false).stderr, /usage: grunkalunka compile-value/);
report.controls.push('numeric protocol, usage and signature separation');
for (const line of ['', '0 0', '0000000000000000', 'zzzzzzzzzzzzzzzz 0000000000000000',
  '0000000000000000 0000000000000000 0000000000000000']) {
  const result = invoke(['eval-value', path.join(run, 'null.js.wasm')], line + '\n', false);
  assert.equal(result.stdout, '');
}
report.controls.push('malformed input lines');
const zero = '0000000000000000 0000000000000000\n', nullModule = path.join(run, 'null.js.wasm');
assert.equal(invoke(['eval-value', nullModule], zero.repeat(4096)).stdout, 'null\n'.repeat(4096));
const limited = invoke(['eval-value', nullModule], zero.repeat(4097), false);
assert.equal(limited.stdout, 'null\n'.repeat(4096)); assert.match(limited.stderr, /at most 4096 input pairs/);
report.controls.push('4096 repeated calls and 4097th input limit');
const existing = path.join(run, 'existing.wasm'); fs.writeFileSync(existing, 'sentinel');
invoke(['compile-value', numberSource, existing], '', false);
assert.equal(fs.readFileSync(existing, 'utf8'), 'sentinel');
const invalid = path.join(run, 'invalid.js'); fs.writeFileSync(invalid, 'export function run(a,b) {return null;} @');
const failed = path.join(run, 'invalid.wasm');
assert.match(invoke(['compile-value', invalid, failed], '', false).stderr, /parse diagnostics/);
assert.ok(!fs.existsSync(failed));
const extension = path.join(run, 'invalid.txt'); fs.writeFileSync(extension, 'ignored');
assert.match(invoke(['compile-value', extension, failed], '', false).stderr, /source extension/);
assert.ok(!fs.existsSync(failed));
report.controls.push('no overwrite and no artifact after failure');
const fuelSource = path.join(run, 'fuel.js'), fuelWasm = path.join(run, 'fuel.wasm');
fs.writeFileSync(fuelSource, 'export function run(a,b) {while(a) {a+=0;} return null;}');
invoke(['compile-value', fuelSource, fuelWasm]);
const fuel = invoke(['eval-value', fuelWasm], '3ff0000000000000 0000000000000000\n', false);
assert.equal(fuel.stdout, ''); assert.match(fuel.stderr, /fuel/);
report.controls.push('fuel exhaustion');
assert.equal(report.pairedPrograms, 21); assert.equal(report.numericOriginCases,4); assert.equal(report.rejections.length, 12);
assert.equal(report.comparisons, 1176);
report.artifacts = fs.readdirSync(run).sort().map(name => {const bytes=fs.readFileSync(path.join(run,name)); return {path:name,bytes:bytes.length,sha256:sha(bytes)};});
save('report.json', JSON.stringify(report, null, 2) + '\n');
console.log(`PASS primitive-return-v1: ${report.pairedPrograms} pairs, ${report.rejectedPairs} rejected pairs, ${report.comparisons} comparisons, ${report.controls.length} CLI controls`);
console.log('Primitive profile report: ' + run);
