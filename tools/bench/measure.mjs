import { performance } from 'node:perf_hooks';

// Fixed goldens are checked outside timed loops, independently of either engine.
const cases = [[1071, 462, 21], [84, 30, 6], [-84, 30, 6], [0, 0, 0],
  [0, -9, 9], [13, 17, 1], [81, 27, 27], [-55, -34, 1]];

export function configuration({ samples = 9, iterations = 100_000, warmup = 5000 } = {}) {
  for (const [name, value, min, max] of [
    ['samples', samples, 2, 50], ['iterations', iterations, 1, 1_000_000], ['warmup', warmup, 0, 1_000_000],
  ]) {
    if (!Number.isInteger(value) || value < min || value > max) throw new RangeError(`Invalid ${name}`);
  }
  return { samples, iterations, warmup };
}

export function statistics(samplesMs) {
  if (!Array.isArray(samplesMs) || samplesMs.length === 0 ||
      samplesMs.some(value => !Number.isFinite(value) || value < 0)) throw new RangeError('Invalid timings');
  const sorted = [...samplesMs].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const medianMs = sorted.length % 2 ? sorted[middle] : sorted[middle - 1] / 2 + sorted[middle] / 2;
  return { samplesMs: [...samplesMs], minMs: sorted[0], medianMs, maxMs: sorted.at(-1) };
}

function expectedChecksum(iterations) {
  const cycle = cases.reduce((sum, entry) => sum + entry[2], 0);
  return Math.floor(iterations / cases.length) * cycle +
    cases.slice(0, iterations % cases.length).reduce((sum, entry) => sum + entry[2], 0);
}
function batch(fn, count) {
  let checksum = 0;
  for (let i = 0; i < count; i++) {
    const [a, b] = cases[i % cases.length];
    checksum += fn(a, b);
  }
  return checksum;
}
function observe(fn) {
  const start = performance.now();
  const value = fn();
  return { value, ms: performance.now() - start };
}
function requireEqual(actual, expected) {
  if (!Object.is(actual, expected)) throw new Error('GCD result/checksum mismatch');
}

export async function measure(wasm, source, options) {
  const config = configuration(options);
  const js = (await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`)).run;
  const compiled = observe(() => new WebAssembly.Module(wasm));
  if (WebAssembly.Module.imports(compiled.value).length) throw new Error('Probe grants no imports');
  const instantiated = observe(() => new WebAssembly.Instance(compiled.value));
  const guest = instantiated.value.exports.run;
  const firstJs = observe(() => js(cases[0][0], cases[0][1]));
  const firstWasm = observe(() => guest(cases[0][0], cases[0][1]));
  requireEqual(firstJs.value, cases[0][2]);
  requireEqual(firstWasm.value, cases[0][2]);
  for (const [a, b, expected] of cases) {
    requireEqual(js(a, b), expected);
    requireEqual(guest(a, b), expected);
  }
  for (const fn of [js, guest]) requireEqual(batch(fn, config.warmup), expectedChecksum(config.warmup));
  const timings = { js: [], wasm: [] };
  const expected = expectedChecksum(config.iterations);
  for (let i = 0; i < config.samples; i++) {
    for (const name of i % 2 ? ['wasm', 'js'] : ['js', 'wasm']) {
      const observed = observe(() => batch(name === 'js' ? js : guest, config.iterations));
      requireEqual(observed.value, expected); // Deliberately outside the measured region.
      timings[name].push(observed.ms);
    }
  }
  return {
    config, cases, checksum: expected,
    startup: { moduleConstructionMs: compiled.ms, instantiationMs: instantiated.ms,
      firstJsCallMs: firstJs.ms, firstWasmCallMs: firstWasm.ms },
    batches: { order: 'alternating, JS first in sample 0', unit: 'milliseconds per batch',
      js: statistics(timings.js), wasm: statistics(timings.wasm) },
  };
}
