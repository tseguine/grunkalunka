// The browser is a transport and pixel surface. Native hit testing and the
// compiled service select every committed view, including keyboard activation.
const WIDTH = 320;
const HEIGHT = 160;
const RGB_BYTES = WIDTH * HEIGHT * 3;
const BASE64_LENGTH = RGB_BYTES / 3 * 4;
const MAX_REPLY_BYTES = 220_000;
const TOKEN_KEY = 'grunkalunka:a1:session';
const validToken = value => typeof value === 'string' && /^[A-Za-z0-9_-]{32,128}$/.test(value);

export function pointerToNative({ clientX, clientY }, rect) {
  if (!rect || ![clientX, clientY, rect.left, rect.top, rect.width, rect.height]
    .every(Number.isFinite) || rect.width <= 0 || rect.height <= 0) return null;
  const left = clientX - rect.left;
  const top = clientY - rect.top;
  if (left < 0 || top < 0 || left >= rect.width || top >= rect.height) return null;
  const x = Math.floor(left / rect.width * WIDTH);
  const y = Math.floor(top / rect.height * HEIGHT);
  // Reject round-off at the far boundary instead of turning an outside click
  // into an actionable edge pixel. The canvas itself has no CSS border/padding.
  return x >= 0 && y >= 0 && x < WIDTH && y < HEIGHT ? { x, y } : null;
}

export function decodeLiveFrame(frame) {
  if (frame === null || typeof frame !== 'object' || Array.isArray(frame) ||
      frame.schemaVersion !== 1 || frame.width !== WIDTH || frame.height !== HEIGHT ||
      !Number.isSafeInteger(frame.sequence) || frame.sequence < 0 || frame.sequence > 256 ||
      ![0, 1].includes(frame.expanded) ||
      !Number.isSafeInteger(frame.visibleCount) || frame.visibleCount < 0 || frame.visibleCount > 64 ||
      (frame.expanded === 0 && frame.visibleCount !== 0) ||
      (frame.expanded === 1 && frame.visibleCount === 0) ||
      typeof frame.canActivate !== 'boolean' ||
      !Number.isSafeInteger(frame.remainingEvents) || frame.remainingEvents < 0 ||
      frame.sequence + frame.remainingEvents < 1 || frame.sequence + frame.remainingEvents > 256 ||
      typeof frame.pixels !== 'string' || frame.pixels.length !== BASE64_LENGTH ||
      /[^A-Za-z0-9+/]/.test(frame.pixels)) {
    throw new TypeError('Invalid A1 live frame');
  }
  const rgb = atob(frame.pixels);
  if (rgb.length !== RGB_BYTES) throw new TypeError('Invalid A1 live RGB bytes');
  const rgba = new Uint8ClampedArray(WIDTH * HEIGHT * 4);
  for (let pixel = 0; pixel < WIDTH * HEIGHT; pixel++) {
    rgba[pixel * 4] = rgb.charCodeAt(pixel * 3);
    rgba[pixel * 4 + 1] = rgb.charCodeAt(pixel * 3 + 1);
    rgba[pixel * 4 + 2] = rgb.charCodeAt(pixel * 3 + 2);
    rgba[pixel * 4 + 3] = 255;
  }
  return { sequence: frame.sequence, expanded: frame.expanded,
    visibleCount: frame.visibleCount, canActivate: frame.canActivate,
    remainingEvents: frame.remainingEvents, rgba };
}

function forgetToken(storage) {
  try { storage?.removeItem(TOKEN_KEY); } catch { /* Storage may be disabled. */ }
}

function sessionToken(window) {
  let storage;
  try { storage = window.sessionStorage; } catch { /* Keep a launch token in memory. */ }
  const fragment = window.location.hash.slice(1);
  // Remove the launch capability before any request. Refreshes use tab-scoped,
  // origin-scoped sessionStorage; it is never copied to a query or localStorage.
  window.history.replaceState(null, '', window.location.pathname + window.location.search);
  if (fragment) {
    forgetToken(storage);
    if (!validToken(fragment)) return { token: null, storage };
    try { storage?.setItem(TOKEN_KEY, fragment); } catch { /* Memory-only is sufficient. */ }
    return { token: fragment, storage };
  }
  let stored;
  try { stored = storage?.getItem(TOKEN_KEY); } catch { /* A launch link can reconnect. */ }
  if (!validToken(stored)) {
    forgetToken(storage);
    return { token: null, storage };
  }
  return { token: stored, storage };
}

async function readReply(response) {
  if (!/^application\/json(?:\s*;|$)/i.test(response.headers.get('content-type') ?? '')) {
    throw new TypeError('Invalid A1 live response type');
  }
  const reader = response.body?.getReader();
  if (!reader) throw new TypeError('Missing A1 live response');
  const bytes = new Uint8Array(MAX_REPLY_BYTES);
  let length = 0;
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      if (value.byteLength > MAX_REPLY_BYTES - length) {
        throw new RangeError('A1 live response exceeds its limit');
      }
      bytes.set(value, length);
      length += value.byteLength;
    }
    return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(0, length)));
  } catch (error) {
    await reader.cancel().catch(() => {});
    throw error;
  } finally {
    reader.releaseLock();
  }
}

export function startLiveClient({ window, document, fetch = window.fetch.bind(window) }) {
  const canvas = document.getElementById('note-frame');
  const status = document.getElementById('session-status');
  const context = canvas.getContext('2d', { alpha: false });
  let current = null;
  let busy = true;
  let connected = false;
  let disposed = false;
  let requestAbort = null;
  const { token, storage } = sessionToken(window);

  function updateControl() {
    const enabled = connected && !busy && current?.canActivate && current.remainingEvents > 0;
    canvas.setAttribute('aria-busy', String(busy));
    canvas.setAttribute('aria-disabled', String(!enabled));
    canvas.setAttribute('role', current?.canActivate ? 'button' : 'img');
    canvas.setAttribute('aria-label', current?.canActivate
      ? (current.expanded ? 'Hide outline' : 'Show outline') : 'Note outline view');
    if (current?.canActivate) canvas.setAttribute('aria-expanded', String(current.expanded === 1));
    else canvas.removeAttribute('aria-expanded');
  }

  function paint(value) {
    const next = decodeLiveFrame(value);
    if (current && next.sequence < current.sequence) throw new Error('Stale A1 live frame');
    const pixels = context.createImageData(WIDTH, HEIGHT);
    pixels.data.set(next.rgba);
    context.putImageData(pixels, 0, 0);
    current = next;
  }

  function describeFrame() {
    if (current.remainingEvents === 0) return 'Session input limit reached. Start a new session to continue.';
    if (!current.canActivate) return 'View only. Outline controls are unavailable in this session.';
    if (!current.expanded) return 'Outline collapsed.';
    return 'Outline expanded. ' + current.visibleCount +
      (current.visibleCount === 1 ? ' heading.' : ' headings.');
  }

  async function request(path, input) {
    requestAbort = new AbortController();
    const response = await fetch(path, {
      method: input === undefined ? 'GET' : 'POST',
      headers: { 'x-a1-session': token, accept: 'application/json',
        ...(input === undefined ? {} : { 'content-type': 'application/json' }) },
      ...(input === undefined ? {} : { body: JSON.stringify(input) }),
      credentials: 'omit', mode: 'same-origin', cache: 'no-store', redirect: 'error',
      referrerPolicy: 'no-referrer', signal: requestAbort.signal,
    });
    if (response.status === 401 || response.status === 403) {
      forgetToken(storage);
      await response.body?.cancel().catch(() => {});
      throw new Error('Session access expired. Reopen the session link to reconnect.');
    }
    const reply = await readReply(response);
    if (disposed) return null;
    if (!response.ok) {
      if (reply === null || typeof reply !== 'object' || !reply.frame) {
        throw new Error('Connection interrupted. Reopen the session link to reconnect.');
      }
      paint(reply.frame);
      if (response.status === 503) {
        throw new Error('Connection interrupted. Reopen the session link to reconnect.');
      }
      return typeof reply.error === 'string' && reply.error.length <= 256
        ? reply.error : 'The action failed. The last completed view is shown.';
    }
    paint(reply);
    return null;
  }

  function failed(error) {
    connected = false;
    status.textContent = error.message === 'Session access expired. Reopen the session link to reconnect.'
      ? error.message : 'Connection interrupted. Reopen the session link to reconnect.';
  }

  async function activate(input) {
    if (disposed || busy || !connected || !current?.canActivate || current.remainingEvents === 0) return;
    busy = true;
    status.textContent = 'Updating outline…';
    updateControl();
    try {
      const message = await request('/input', input);
      if (!disposed) status.textContent = message ?? describeFrame();
    } catch (error) {
      if (!disposed) failed(error);
    } finally {
      requestAbort = null;
      busy = false;
      if (!disposed) updateControl();
    }
  }

  function pointer(event) {
    if (event.button !== 0 || event.isPrimary === false) return;
    const input = pointerToNative(event, canvas.getBoundingClientRect());
    if (!input) return;
    canvas.focus();
    void activate(input);
  }

  function keyboard(event) {
    if (!['Enter', ' '].includes(event.key) || !current?.canActivate || current.remainingEvents === 0) return;
    event.preventDefault();
    if (event.repeat) return;
    // This point lies inside the native title control in both expansion states.
    // Native hit testing still decides whether the input has any action target.
    void activate({ x: 24, y: 16 });
  }

  canvas.addEventListener('click', pointer);
  canvas.addEventListener('keydown', keyboard);
  updateControl();
  const ready = (async () => {
    if (!token || !context) {
      busy = false;
      status.textContent = token ? 'This browser cannot display the note view.'
        : 'Open the session link to connect to your note.';
      updateControl();
      return;
    }
    try {
      await request('/frame');
      if (disposed) return;
      connected = true;
      if (!disposed) status.textContent = describeFrame();
    } catch (error) {
      if (!disposed) failed(error);
    } finally {
      requestAbort = null;
      busy = false;
      if (!disposed) updateControl();
    }
  })();

  function dispose() {
    disposed = true;
    connected = false;
    requestAbort?.abort();
    canvas.removeEventListener('click', pointer);
    canvas.removeEventListener('keydown', keyboard);
  }

  return Object.freeze({ ready, dispose });
}

if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  startLiveClient({ window, document });
}
