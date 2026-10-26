// Internal entry point. All guest compilation, initialization and execution stays here.
import { parentPort, workerData } from 'node:worker_threads';

const { wasm, a, b, profile } = workerData;
const label = profile === 'primitive' ? 'Primitive' : 'Numeric';
const module = new WebAssembly.Module(wasm);
if (WebAssembly.Module.imports(module).length !== 0) {
  throw new TypeError(`${label} execution does not grant imports`);
}
const instance = new WebAssembly.Instance(module);
if (typeof instance.exports.run !== 'function') {
  throw new TypeError(`${label} module must export a run function`);
}
const value = instance.exports.run(a, b);
if (profile !== 'primitive' && typeof value !== 'number') throw new TypeError('Numeric run must return one Number');
parentPort.postMessage(value);
