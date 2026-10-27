// Whole-Node-process memory observations; do not attribute them to Wasm alone.
export const NODE_MEMORY_PHASES = Object.freeze([
  'beforeModule', 'afterModule', 'afterInstantiation', 'afterWarmup', 'afterBatches',
]);
const FIELDS = Object.freeze([
  ['rssBytes', 'rss'], ['heapTotalBytes', 'heapTotal'],
  ['heapUsedBytes', 'heapUsed'], ['externalBytes', 'external'],
  ['arrayBuffersBytes', 'arrayBuffers'],
]);

export function memorySnapshot() {
  const observation = process.memoryUsage();
  return Object.fromEntries(FIELDS.map(([name, field]) => [name, observation[field]]));
}

export function validateMemorySnapshots(memory) {
  if (!memory || typeof memory !== 'object' || memory.unit !== 'bytes' ||
      memory.scope !== 'entire-node-process' || !memory.snapshots ||
      typeof memory.snapshots !== 'object') {
    throw new TypeError('Invalid Node memory report envelope');
  }
  for (const phase of NODE_MEMORY_PHASES) {
    const snapshot = memory.snapshots[phase];
    if (!snapshot || typeof snapshot !== 'object') {
      throw new TypeError('Missing Node memory phase: ' + phase);
    }
    for (const [name] of FIELDS) {
      const value = snapshot[name];
      if (!Number.isSafeInteger(value) || value < 0) {
        throw new TypeError('Invalid Node memory value: ' + phase + '.' + name);
      }
    }
    if (snapshot.heapUsedBytes > snapshot.heapTotalBytes) {
      throw new TypeError('Node heap usage exceeds its reserved heap');
    }
  }
  return memory;
}
