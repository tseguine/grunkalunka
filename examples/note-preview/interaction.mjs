// Private A1 headless input adapter: actual native hit testing triggers a
// source-compiled selector re-evaluation and a new native-painted frame.
// This is not a DOM event, an exposed WIT UI service or a window system.
import { spawnSync } from 'node:child_process';
import { prepareNoteScene } from './scene.mjs';

const PPM_HEADER = Buffer.from('P6\n320 160\n255\n');
const MAX_PPM = PPM_HEADER.length + 320 * 160 * 3;

function callNative(executable, args, input, maxBuffer) {
  const result = spawnSync(executable, args, {
    input, shell: false, encoding: null, timeout: 30_000,
    killSignal: 'SIGKILL', maxBuffer,
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('A1 native scene failed: ' +
      (result.stderr?.toString('utf8').trim() || result.error?.message ||
       result.signal || result.status));
  }
  return result.stdout;
}

function paint(executable, scene) {
  const ppm = callNative(executable, ['--ppm'], scene.input, MAX_PPM + 1024);
  if (ppm.length !== MAX_PPM || !ppm.subarray(0, PPM_HEADER.length).equals(PPM_HEADER)) {
    throw new Error('Invalid native A1 frame');
  }
  return ppm;
}

export async function activateNoteHeading({
  executable, wasm, document, expanded, x, y,
}) {
  if (typeof executable !== 'string' || !executable ||
      !Number.isSafeInteger(x) || !Number.isSafeInteger(y) ||
      x < 0 || y < 0 || x > 0xffffffff || y > 0xffffffff ||
      ![0, 1].includes(expanded)) {
    throw new TypeError('Invalid A1 activation request');
  }
  const before = await prepareNoteScene({ ...document, wasm, expanded });
  const previousPixels = paint(executable, before);
  const raw = callNative(executable, ['--hit', String(x), String(y)], before.input, 4096)
    .toString('utf8');
  if (raw === 'A1HIT1\tnone\n') {
    return { activated: false, heading: null, before, after: before,
      previousPixels, pixels: previousPixels };
  }
  const match = /^A1HIT1\t(0|[1-9][0-9]*)\t(0|[1-9][0-9]*)\t(0|[1-9][0-9]*)\t([1-6])\n$/.exec(raw);
  if (!match) throw new Error('Malformed native A1 hit response');
  const [index, start, end, level] = match.slice(1).map(Number);
  const heading = document.headings?.[index];
  if (document.profile !== 'granted' || index >= before.visibleCount ||
      !heading || heading.start !== start || heading.end !== end ||
      heading.level !== level) {
    throw new Error('Native A1 hit disagrees with authorized WIT headings');
  }
  // Selected heading activation collapses the outline. This is a deliberately
  // minimal host-managed transition until source JS can call a WIT UI service.
  const after = await prepareNoteScene({ ...document, wasm, expanded: 0 });
  const pixels = paint(executable, after);
  return { activated: true, heading: { index, start, end, level },
    before, after, previousPixels, pixels };
}
