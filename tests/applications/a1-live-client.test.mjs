import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeLiveFrame, pointerToNative, startLiveClient } from '../../examples/note-preview/live-client.mjs';
import { livePage, liveStyles } from '../../examples/note-preview/live-page.mjs';

const token = 'a'.repeat(64);
const tokenKey = 'grunkalunka:a1:session';
const rgb = Buffer.alloc(320 * 160 * 3);
for (let pixel = 0; pixel < 320 * 160; pixel++) {
  rgb[pixel * 3] = pixel % 256;
  rgb[pixel * 3 + 1] = 82;
  rgb[pixel * 3 + 2] = 166;
}
const pixels = rgb.toString('base64');

function frame(overrides = {}) {
  return { schemaVersion: 1, width: 320, height: 160, sequence: 0,
    expanded: 1, visibleCount: 2, canActivate: true, remainingEvents: 256,
    pixels, ...overrides };
}

function response(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status, headers: { 'content-type': 'application/json; charset=utf-8' },
  });
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

const settle = () => new Promise(resolve => setImmediate(resolve));

// This harness checks browser adapter logic. Real native frame production and
// actual browser-engine input/paint are separate integration evidence.
function browser({ hash = '#' + token, stored, storageThrows = false } = {}) {
  const attributes = new Map();
  const listeners = new Map();
  const painted = [];
  const status = { textContent: '' };
  let focused = 0;
  const canvas = {
    setAttribute: (name, value) => attributes.set(name, value),
    removeAttribute: name => attributes.delete(name),
    getBoundingClientRect: () => ({ left: 30, top: 50, width: 640, height: 320 }),
    getContext: () => ({
      createImageData: (width, height) => ({ width, height,
        data: new Uint8ClampedArray(width * height * 4) }),
      putImageData: (value, x, y) => painted.push({ ...value, x, y }),
    }),
    addEventListener: (name, fn) => listeners.set(name, fn),
    removeEventListener: (name, fn) => {
      if (listeners.get(name) === fn) listeners.delete(name);
    },
    focus: () => { focused++; },
  };
  const data = new Map(stored === undefined ? [] : [[tokenKey, stored]]);
  const location = { hash, pathname: '/', search: '' };
  const window = {
    location,
    history: { replaceState: (_state, _title, path) => {
      assert.equal(path, '/');
      location.hash = '';
    } },
    get sessionStorage() {
      if (storageThrows) throw new Error('Storage disabled');
      return { getItem: key => data.get(key) ?? null,
        setItem: (key, value) => data.set(key, value), removeItem: key => data.delete(key) };
    },
  };
  const requests = [];
  const replies = [];
  const fetch = async (path, options) => {
    requests.push({ path, ...options });
    const reply = replies.shift();
    assert.ok(reply, 'unexpected browser request');
    return reply;
  };
  const document = { getElementById: id => id === 'note-frame' ? canvas : status };
  const click = (overrides = {}) => listeners.get('click')?.({
    clientX: 78, clientY: 82, button: 0, isPrimary: true, ...overrides,
  });
  function key(key, overrides = {}) {
    let prevented = false;
    listeners.get('keydown')?.({ key, repeat: false,
      preventDefault: () => { prevented = true; }, ...overrides });
    return prevented;
  }
  return { window, document, fetch, attributes, listeners, painted, status,
    requests, replies, data, click, key, focused: () => focused };
}

test('pointer mapping uses the current CSS rectangle and excludes every outside edge', () => {
  const rect = { left: 30, top: 50, width: 640, height: 320 };
  assert.deepEqual(pointerToNative({ clientX: 78, clientY: 82 }, rect), { x: 24, y: 16 });
  assert.deepEqual(pointerToNative({ clientX: 30, clientY: 50 }, rect), { x: 0, y: 0 });
  assert.deepEqual(pointerToNative({ clientX: 669.99, clientY: 369.99 }, rect), { x: 319, y: 159 });
  assert.deepEqual(pointerToNative({ clientX: 42, clientY: 58 },
    { ...rect, width: 160, height: 80 }), { x: 24, y: 16 });
  for (const point of [
    { clientX: 29.99, clientY: 70 }, { clientX: 670, clientY: 70 },
    { clientX: 50, clientY: 49.99 }, { clientX: 50, clientY: 370 },
    { clientX: Infinity, clientY: 70 }, { clientX: 50, clientY: NaN },
  ]) assert.equal(pointerToNative(point, rect), null);
  for (const invalid of [null, { ...rect, width: 0 }, { ...rect, height: -1 },
    { ...rect, left: Infinity }, { ...rect, width: NaN }]) {
    assert.equal(pointerToNative({ clientX: 50, clientY: 70 }, invalid), null);
  }
});

test('frame decoding preserves supplied native RGB and rejects malformed or unbounded state', () => {
  const decoded = decodeLiveFrame(frame());
  assert.equal(decoded.rgba.length, 320 * 160 * 4);
  assert.deepEqual([...decoded.rgba.subarray(0, 8)], [0, 82, 166, 255, 1, 82, 166, 255]);
  assert.deepEqual([...decoded.rgba.subarray(-4)], [255, 82, 166, 255]);
  decoded.rgba.fill(0);
  assert.equal(decodeLiveFrame(frame()).rgba[1], 82);
  assert.doesNotThrow(() => decodeLiveFrame(frame({ sequence: 256, remainingEvents: 0 })));
  assert.doesNotThrow(() => decodeLiveFrame(frame({ canActivate: false })));
  for (const invalid of [null, [], frame({ schemaVersion: 2 }), frame({ width: 640 }),
    frame({ height: 159 }), frame({ sequence: -1 }), frame({ sequence: '0' }),
    frame({ sequence: 257, remainingEvents: 0 }), frame({ expanded: 2 }),
    frame({ expanded: 0 }), frame({ visibleCount: 0 }), frame({ visibleCount: 65 }),
    frame({ visibleCount: 0.5 }), frame({ canActivate: 1 }),
    frame({ remainingEvents: -1 }), frame({ remainingEvents: 257 }),
    frame({ remainingEvents: 0 }), frame({ pixels: pixels.slice(1) }),
    frame({ pixels: pixels + 'AAAA' }), frame({ pixels: '*' + pixels.slice(1) }),
    frame({ pixels: pixels.slice(0, -1) + '=' })]) {
    assert.throws(() => decodeLiveFrame(invalid), /Invalid A1 live/);
  }
});

test('client consumes a launch capability, paints native pixels, and serializes scaled pointer and keyboard inputs', async t => {
  const b = browser({ stored: 'b'.repeat(64) });
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  assert.equal(b.window.location.hash, '');
  assert.equal(b.data.get(tokenKey), token);
  await client.ready;
  assert.equal(b.requests[0].path, '/frame');
  assert.deepEqual(b.requests[0].headers, { 'x-a1-session': token, accept: 'application/json' });
  assert.equal(b.requests[0].credentials, 'omit');
  assert.equal(b.requests[0].mode, 'same-origin');
  assert.equal(b.requests[0].redirect, 'error');
  assert.equal(b.painted.length, 1);
  assert.deepEqual([...b.painted[0].data.subarray(0, 4)], [0, 82, 166, 255]);
  assert.equal(b.attributes.get('aria-expanded'), 'true');
  assert.equal(b.attributes.get('aria-label'), 'Hide outline');

  const input = deferred();
  b.replies.push(input.promise);
  b.click();
  b.click();
  assert.equal(b.requests.length, 2, 'a busy canvas does not queue more input');
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.equal(b.attributes.get('aria-busy'), 'true');
  assert.equal(b.requests[1].path, '/input');
  assert.equal(b.requests[1].method, 'POST');
  assert.equal(b.requests[1].body, '{"x":24,"y":16}');
  assert.equal(b.requests[1].headers['content-type'], 'application/json');
  input.resolve(response(frame({ sequence: 1, expanded: 0, visibleCount: 0, remainingEvents: 255 })));
  await settle();
  assert.equal(b.painted.length, 2);
  assert.equal(b.attributes.get('aria-expanded'), 'false');
  assert.equal(b.attributes.get('aria-label'), 'Show outline');
  assert.equal(b.attributes.get('aria-disabled'), 'false');
  assert.equal(b.status.textContent, 'Outline collapsed.');

  b.replies.push(response(frame({ sequence: 2, remainingEvents: 254 })));
  assert.equal(b.key('Enter'), true);
  await settle();
  assert.equal(b.requests[2].body, '{"x":24,"y":16}');
  assert.equal(b.attributes.get('aria-expanded'), 'true');
  b.replies.push(response(frame({ sequence: 3, expanded: 0, visibleCount: 0, remainingEvents: 253 })));
  assert.equal(b.key(' '), true);
  await settle();
  assert.equal(b.attributes.get('aria-expanded'), 'false');
  const count = b.requests.length;
  b.click({ button: 2 });
  b.click({ isPrimary: false });
  b.click({ clientX: 670 });
  b.click({ clientY: 370 });
  b.key('Enter', { repeat: true });
  b.key('Escape');
  assert.equal(b.requests.length, count);
  assert.ok(b.focused() > 0);
});

test('sessionStorage supports refresh, invalid launch tokens replace old authority, and blocked storage keeps launch working', async t => {
  for (const options of [{ hash: '', stored: token }, { storageThrows: true }]) {
    const b = browser(options);
    b.replies.push(response(frame()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.requests.length, 1);
    assert.equal(b.requests[0].headers['x-a1-session'], token);
    assert.equal(b.painted.length, 1);
  }
  for (const options of [{ hash: '', stored: undefined }, { hash: '#bad', stored: token }]) {
    const b = browser(options);
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.requests.length, 0);
    assert.equal(b.data.size, 0);
    assert.match(b.status.textContent, /Open the session link/);
    assert.equal(b.attributes.get('aria-disabled'), 'true');
  }
});

test('authorization failures erase stored authority and leave the view disabled', async t => {
  for (const code of [401, 403]) {
    const b = browser();
    b.replies.push(response({ error: 'Access denied' }, code));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.data.size, 0);
    assert.match(b.status.textContent, /Session access expired/);
    assert.equal(b.painted.length, 0);
    b.click();
    assert.equal(b.requests.length, 1);
  }
});

test('a failed action may publish its retained frame, but an older reply never replaces a newer view', async t => {
  const b = browser();
  b.replies.push(response(frame({ sequence: 1, remainingEvents: 255 })));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const message = '<b>Action failed; the preceding view is retained.</b>';
  b.replies.push(response({ error: message,
    frame: frame({ sequence: 2, remainingEvents: 254 }) }, 500));
  b.click();
  await settle();
  assert.equal(b.painted.length, 2);
  assert.equal(b.status.textContent, message, 'errors are assigned as text, not parsed HTML');
  assert.equal(b.attributes.get('aria-disabled'), 'false');
  b.replies.push(response(frame({ sequence: 1, expanded: 0, visibleCount: 0, remainingEvents: 255 })));
  b.click();
  await settle();
  assert.equal(b.painted.length, 2);
  assert.equal(b.attributes.get('aria-expanded'), 'true');
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.match(b.status.textContent, /Reopen the session link/);
});

test('uncertain, oversized or closed responses preserve the last frame and stop further input', async t => {
  for (const next of [
    () => Promise.reject(new Error('connection lost after sending input')),
    () => response({ error: 'Closed' }, 503),
    () => response(frame({ pixels: 'A'.repeat(220_001) })),
    () => new Response('not JSON', { headers: { 'content-type': 'text/plain' } }),
    () => response(frame({ width: 400 })),
  ]) {
    const b = browser();
    b.replies.push(response(frame()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    b.replies.push(next());
    b.click();
    await settle();
    assert.equal(b.painted.length, 1);
    assert.equal(b.attributes.get('aria-disabled'), 'true');
    assert.equal(b.attributes.get('aria-busy'), 'false');
    b.click();
    b.key('Enter');
    assert.equal(b.requests.length, 2, 'do not blindly repeat an uncertain action');
  }
});

test('a terminal response applies its newest retained snapshot before disabling input', async t => {
  const b = browser();
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  b.replies.push(response({ error: 'Session closed',
    frame: frame({ sequence: 1, remainingEvents: 255, canActivate: false }) }, 503));
  b.click();
  await settle();
  assert.equal(b.painted.length, 2, 'the terminal snapshot is applied despite unchanged pixels');
  assert.deepEqual(b.painted[1].data, b.painted[0].data);
  assert.equal(b.attributes.get('role'), 'img', 'control state comes from the final snapshot');
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.equal(b.attributes.get('aria-busy'), 'false');
  assert.match(b.status.textContent, /Connection interrupted.*Reopen the session link/);
  b.click();
  b.key('Enter');
  assert.equal(b.requests.length, 2);
});

test('denied and exhausted sessions retain their supplied view without forwarding activation', async t => {
  for (const state of [frame({ canActivate: false }),
    frame({ sequence: 256, remainingEvents: 0 })]) {
    const b = browser();
    b.replies.push(response(state));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.painted.length, 1);
    assert.equal(b.attributes.get('aria-disabled'), 'true');
    b.click();
    b.key('Enter');
    b.key(' ');
    assert.equal(b.requests.length, 1);
    if (!state.canActivate) assert.equal(b.attributes.get('role'), 'img');
    else assert.match(b.status.textContent, /input limit reached/);
  }
});

test('disposing the adapter aborts a pending request and removes both input handlers', async () => {
  const b = browser();
  const pending = deferred();
  b.replies.push(pending.promise);
  const client = startLiveClient(b);
  client.dispose();
  assert.equal(b.requests[0].signal.aborted, true);
  assert.equal(b.listeners.size, 0);
  pending.resolve(response(frame()));
  await client.ready;
  assert.equal(b.painted.length, 0);
});

test('fixed page assets use external scripts/styles and preserve a borderless native coordinate surface', () => {
  assert.match(livePage, /<script type="module" src="\/client\.mjs"><\/script>/);
  assert.match(livePage, /<link rel="stylesheet" href="\/style\.css">/);
  assert.match(livePage, /id="note-frame" width="320" height="160"/);
  assert.match(livePage, /tabindex="0"/);
  assert.ok(!livePage.includes('<style') && !livePage.includes('onclick='));
  assert.match(liveStyles, /canvas \{[^}]*padding: 0; border: 0;/);
  assert.match(liveStyles, /image-rendering: pixelated/);
});
