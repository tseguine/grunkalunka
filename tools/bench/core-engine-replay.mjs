// Replay the exact core Wasm bytes supplied on stdin; no compiler or JS oracle runs here.
import { createHash } from 'node:crypto';
import { arch, cpus, platform } from 'node:os';
import { performance } from 'node:perf_hooks';
import { memorySnapshot, validateMemorySnapshots } from './node-memory.mjs';

const cases = [[1071, 462, 21], [84, 30, 6], [-84, 30, 6], [0, 0, 0],
  [0, -9, 9], [13, 17, 1], [81, 27, 27], [-55, -34, 1]];

function controls(values) {
  if (values.length !== 3) throw new Error('Expected samples iterations warmup');
  const numbers = values.map(value => {
    if (!/^\d+$/.test(value)) throw new Error('Invalid integer');
    return Number(value);
  });
  const [samples, iterations, warmup] = numbers;
  for (const [value, min, max] of [[samples, 2, 20], [iterations, 1, 10000],
    [warmup, 0, 10000]]) {
    if (!Number.isSafeInteger(value) || value < min || value > max) {
      throw new Error('Cost replay option is out of bounds');
    }
  }
  return { samples, iterations, warmup };
}

async function input() {
  const chunks = [];
  let length = 0;
  for await (const chunk of process.stdin) {
    length += chunk.length;
    if (length > 1024 * 1024) throw new Error('Wasm input exceeds 1 MiB');
    chunks.push(chunk);
  }
  if (length === 0) throw new Error('Missing core Wasm input');
  return Buffer.concat(chunks, length);
}

function checksum(iterations) {
  const full = Math.floor(iterations / cases.length);
  const part = iterations % cases.length;
  return full * cases.reduce((sum, entry) => sum + entry[2], 0) +
    cases.slice(0, part).reduce((sum, entry) => sum + entry[2], 0);
}

function batch(fn, iterations) {
  let total = 0;
  for (let i = 0; i < iterations; i++) {
    const [a, b] = cases[i % cases.length];
    total += fn(a, b);
  }
  return total;
}

function statistics(samplesMs) {
  const sorted = [...samplesMs].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return {
    samplesMs,
    minMs: sorted[0],
    medianMs: sorted.length % 2 ? sorted[middle] :
      sorted[middle - 1] / 2 + sorted[middle] / 2,
    maxMs: sorted.at(-1),
  };
}

// An independent whole-process observation uses fresh Node/V8 state and
// exactly the same supplied compiler-emitted Wasm as the normal replay.
async function freshProcess() {
  const wasm = await input();
  const module = new WebAssembly.Module(wasm);
  if (WebAssembly.Module.imports(module).length !== 0) {
    throw new Error('Fresh-process replay must not grant imports');
  }
  const run = new WebAssembly.Instance(module).exports.run;
  if (typeof run !== 'function') throw new Error('Missing fresh-process run export');
  for (const [a, b, expected] of cases) {
    if (!Object.is(run(a, b), expected)) {
      throw new Error('Incorrect fresh-process independent golden');
    }
  }
  process.stdout.write(JSON.stringify({
    mode: 'fresh-process',
    sha256: createHash('sha256').update(wasm).digest('hex'),
    wasmBytes: wasm.length,
    goldenCases: cases.length,
  }) + '\n');
}

async function main() {
  if (process.argv.length === 3 && process.argv[2] === '--fresh-process') {
    await freshProcess();
    return;
  }
  const config = controls(process.argv.slice(2));
  const wasm = await input();
  const memorySnapshots = { beforeModule: memorySnapshot() };
  const startedCompile = performance.now();
  const module = new WebAssembly.Module(wasm);
  const moduleCompileMs = performance.now() - startedCompile;
  if (WebAssembly.Module.imports(module).length !== 0) {
    throw new Error('Core cost replay must not grant imports');
  }
  memorySnapshots.afterModule = memorySnapshot();
  const startedInstantiate = performance.now();
  const instance = new WebAssembly.Instance(module);
  const instantiationMs = performance.now() - startedInstantiate;
  memorySnapshots.afterInstantiation = memorySnapshot();
  const run = instance.exports.run;
  if (typeof run !== 'function') throw new Error('Missing run export');
  const startedFirst = performance.now();
  const first = run(cases[0][0], cases[0][1]);
  const firstCallMs = performance.now() - startedFirst;
  if (!Object.is(first, cases[0][2])) throw new Error('Incorrect first result');
  for (const [a, b, expected] of cases) {
    if (!Object.is(run(a, b), expected)) throw new Error('Incorrect fixed golden');
  }
  if (batch(run, config.warmup) !== checksum(config.warmup)) {
    throw new Error('Incorrect warmup checksum');
  }
  memorySnapshots.afterWarmup = memorySnapshot();
  const samplesMs = [];
  for (let i = 0; i < config.samples; i++) {
    const started = performance.now();
    const value = batch(run, config.iterations);
    samplesMs.push(performance.now() - started);
    if (value !== checksum(config.iterations)) {
      throw new Error('Incorrect measured checksum');
    }
  }
  memorySnapshots.afterBatches = memorySnapshot();
  // Keep the five existing process-memory observations unchanged. Reuse
  // the already compiled Module, but make an independent Instance for each
  // bounded sample; ordinary goldens are checked outside the timing window.
  const freshInstantiations = [], freshFirstCalls = [];
  for (let sample = 0; sample < config.samples; sample++) {
    const startedInstantiateFresh = performance.now();
    const freshInstance = new WebAssembly.Instance(module);
    freshInstantiations.push(performance.now() - startedInstantiateFresh);
    const freshRun = freshInstance.exports.run;
    if (typeof freshRun !== 'function') throw new Error('Missing fresh-instance run export');
    const startedFirstFresh = performance.now();
    const freshFirst = freshRun(cases[0][0], cases[0][1]);
    freshFirstCalls.push(performance.now() - startedFirstFresh);
    if (!Object.is(freshFirst, cases[0][2])) {
      throw new Error('Incorrect fresh-instance first result');
    }
    for (const [a, b, expected] of cases) {
      if (!Object.is(freshRun(a, b), expected)) {
        throw new Error('Incorrect fresh-instance golden');
      }
    }
  }
  const memory = validateMemorySnapshots({
    unit: 'bytes', scope: 'entire-node-process', snapshots: memorySnapshots,
  });
  const report = {
    schemaVersion: 1,
    engine: 'node-v8',
    environment: {
      node: process.version, v8: process.versions.v8,
      os: platform(), arch: arch(), cpuModel: cpus()[0]?.model ?? null,
      nodeOptions: process.env.NODE_OPTIONS ?? '', execArgv: process.execArgv,
    },
    artifact: {
      bytes: wasm.length, sha256: createHash('sha256').update(wasm).digest('hex'),
      imports: 0,
    },
    configuration: config, goldenCases: cases.length, checksum: checksum(config.iterations),
    startup: { moduleCompileMs, instantiationMs, firstCallMs },
    batches: { unit: 'milliseconds per batch', ...statistics(samplesMs) },
    freshInstances: {
      count: config.samples,
      instantiation: statistics(freshInstantiations),
      firstCall: statistics(freshFirstCalls),
    },
    memory,
  };
  process.stdout.write(JSON.stringify(report) + '\n');
}

try {
  await main();
} catch (error) {
  console.error(error.stack ?? String(error));
  process.exitCode = 1;
}
