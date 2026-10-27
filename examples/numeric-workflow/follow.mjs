// Internal single-file polling loop; build and report are awaited serially.
import fs from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

async function snapshot(path) {
  try { return { bytes: await fs.readFile(path) }; }
  catch (error) { return { error }; }
}
function same(a, b) {
  if (!a) return false;
  if (a.error || b.error) return a.error?.code === b.error?.code && a.error?.message === b.error?.message;
  return a.bytes.equals(b.bytes);
}

export async function followSource(path, build, report, { signal, interval = 250 } = {}) {
  if (!Number.isInteger(interval) || interval < 25 || interval > 60_000) {
    throw new RangeError('interval must be an integer from 25 through 60000');
  }
  if (signal !== undefined && !(signal instanceof AbortSignal)) throw new TypeError('signal must be an AbortSignal');
  let reported;
  while (!signal?.aborted) {
    const input = await snapshot(path);
    if (signal?.aborted) break;
    if (!same(reported, input)) {
      let outcome;
      if (input.error) outcome = { error: input.error };
      else {
        try { outcome = { value: await build(input.bytes, signal) }; }
        catch (error) { outcome = { error }; }
      }
      if (signal?.aborted) break;
      // Recheck after a slow build. Never report a result for observed old bytes.
      const current = await snapshot(path);
      if (signal?.aborted) break;
      if (!same(input, current)) continue;
      await report(outcome);
      // Discarded builds must not suppress a later reversion to the same bytes.
      reported = input;
    }
    try { await delay(interval, undefined, { signal }); }
    catch (error) { if (!signal?.aborted) throw error; }
  }
}
