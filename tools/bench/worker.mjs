// Internal benchmark worker: its parent owns the whole-probe deadline.
import { parentPort, workerData } from 'node:worker_threads';
import { measure } from './measure.mjs';

parentPort.postMessage(await measure(workerData.wasm, workerData.source, workerData.config));
