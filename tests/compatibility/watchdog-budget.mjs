// Finite engineering watchdog allocation, not a measured performance contract.
export function watchdogBudget(counts) {
  const dimensions = ['paired', 'rejected', 'curated'];
  if (counts === null || typeof counts !== 'object' || Array.isArray(counts)
    || Object.keys(counts).length !== dimensions.length
    || !dimensions.every(key => Object.hasOwn(counts, key)
      && Number.isSafeInteger(counts[key]) && counts[key] >= 0)) {
    throw new TypeError('invalid watchdog counts: expected safe nonnegative paired, rejected and curated integers');
  }
  if (counts.curated > counts.rejected) throw new RangeError('curated watchdog count exceeds rejected count');
  const allowances = [200 * counts.paired, 25 * counts.rejected, 100 * counts.curated];
  let allocation = 30_000;
  for (const allowance of allowances) {
    if (!Number.isSafeInteger(allowance) || !Number.isSafeInteger(allocation + allowance)) {
      throw new RangeError('unsafe watchdog arithmetic');
    }
    allocation += allowance;
  }
  const innerMs = Math.min(600_000, Math.max(180_000, allocation));
  return { innerMs, outerMs: innerMs + 60_000 };
}

export function corpusWatchdogCounts(programs, rejected) {
  if (!Array.isArray(programs) || !Array.isArray(rejected)
    || !programs.every(Array.isArray) || !rejected.every(Array.isArray)) {
    throw new TypeError('invalid watchdog inventory: expected paired and rejection rows');
  }
  return { paired: programs.length, rejected: rejected.length, curated: rejected.filter(row => row[4]).length };
}

export function assertWithinWatchdog(elapsedMs, innerMs) {
  if (elapsedMs > innerMs) throw new Error(`experiment exceeded ${innerMs}ms compatibility watchdog budget`);
}
