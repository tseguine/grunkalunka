import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeLiveFrame, pointerToNative, wheelDeltaToNative,
  startLiveClient } from '../../examples/note-preview/live-client.mjs';
import { livePage, liveStyles } from '../../examples/note-preview/live-page.mjs';

const token = 'a'.repeat(64);
const tokenKey = 'grunkalunka:a1:session';
const scrollControlIds = ['scroll-start', 'scroll-up', 'scroll-down', 'scroll-end'];
const rgb = Buffer.alloc(320 * 160 * 3);
for (let pixel = 0; pixel < 320 * 160; pixel++) {
  rgb[pixel * 3] = pixel % 256;
  rgb[pixel * 3 + 1] = 82;
  rgb[pixel * 3 + 2] = 166;
}
const pixels = rgb.toString('base64');

function frame(overrides = {}) {
  const sequence = overrides.sequence ?? 0;
  const remainingEvents = overrides.remainingEvents ?? 256 - sequence;
  return { schemaVersion: 4, width: 320, height: 160, sequence, inputGeneration: 1,
    expanded: 1, visibleCount: 2, canActivate: remainingEvents > 0, remainingEvents,
    scrollY: 0, maxScrollY: 0, canScroll: false, controlPoint: { x: 24, y: 16 },
    canReload: remainingEvents > 0, noteChoices: [{ id: 0, label: 'note.md' }],
    selectedNoteId: 0, canSelectNote: false, pixels, ...overrides };
}

function emptyFrame(overrides = {}) {
  return frame({ expanded: 0, visibleCount: 0, canActivate: false,
    controlPoint: null, ...overrides });
}

function scrollingFrame(overrides = {}) {
  const scrollY = overrides.scrollY ?? 0;
  const remainingEvents = overrides.remainingEvents ?? 256 - (overrides.sequence ?? 0);
  return frame({ visibleCount: 16, scrollY, maxScrollY: 400,
    canScroll: remainingEvents > 0, canActivate: remainingEvents > 0 && scrollY === 0,
    controlPoint: scrollY === 0 ? { x: 24, y: 16 } : null, ...overrides });
}

function selectionFrame(overrides = {}) {
  const remainingEvents = overrides.remainingEvents ?? 256 - (overrides.sequence ?? 0);
  return frame({ noteChoices: [{ id: 0, label: 'First note' }, { id: 1, label: '<b>Second note</b>' }],
    selectedNoteId: 0, canSelectNote: remainingEvents > 0, ...overrides });
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

function deadlineClock() {
  let now = 0;
  let nextId = 0;
  const pending = new Map();
  const delays = [];
  return {
    pending, delays,
    setTimeout(callback, delay) {
      delays.push(delay);
      const id = ++nextId;
      pending.set(id, { at: now + delay, callback });
      return id;
    },
    clearTimeout(id) { pending.delete(id); },
    advance(milliseconds) {
      now += milliseconds;
      for (const [id, task] of [...pending]) {
        if (task.at <= now && pending.delete(id)) task.callback();
      }
    },
  };
}

// Deliberately ignores transport abort and never settles cancellation. A client
// deadline must retire its own wait without relying on either transport promise.
function stalledBody(status = 200) {
  const first = deferred();
  let reads = 0;
  let cancellations = 0;
  let releases = 0;
  const cancel = () => { cancellations++; return new Promise(() => {}); };
  const reader = {
    read: () => ++reads === 1 ? first.promise : Promise.resolve({ done: true }),
    cancel,
    releaseLock: () => { releases++; },
  };
  return {
    response: { status, ok: status >= 200 && status < 300,
      headers: new Headers({ 'content-type': 'application/json' }),
      body: { getReader: () => reader, cancel } },
    complete: value => first.resolve({ done: false,
      value: new TextEncoder().encode(JSON.stringify(value)) }),
    reads: () => reads, cancellations: () => cancellations, releases: () => releases,
  };
}

async function alreadySettled(promise) {
  let completed = false;
  promise.then(() => { completed = true; });
  await settle();
  assert.equal(completed, true, 'the client settles independently of the stalled transport');
  await promise;
}

// This harness checks browser adapter logic. Real native frame production and
// actual browser-engine input/paint are separate integration evidence.
function browser({ hash = '#' + token, stored, storageThrows = false } = {}) {
  const attributes = new Map();
  const listeners = new Map();
  const listenerOptions = new Map();
  const reloadListeners = new Map();
  const reloadAttributes = new Map();
  const reconnectListeners = new Map();
  const reconnectAttributes = new Map();
  const scrollListeners = new Map();
  const scrollButtons = Object.fromEntries(scrollControlIds.map(id => [id, {
    disabled: true,
    addEventListener: (name, fn) => {
      assert.equal(name, 'click');
      scrollListeners.set(id, fn);
    },
    removeEventListener: (name, fn) => {
      assert.equal(name, 'click');
      if (scrollListeners.get(id) === fn) scrollListeners.delete(id);
    },
  }]));
  const choiceListeners = new Map();
  const choiceAttributes = new Map();
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
    addEventListener: (name, fn, options) => {
      listeners.set(name, fn);
      listenerOptions.set(name, options);
    },
    removeEventListener: (name, fn) => {
      if (listeners.get(name) === fn) listeners.delete(name);
    },
    focus: () => { focused++; },
  };
  const reloadButton = {
    disabled: true,
    setAttribute: (name, value) => reloadAttributes.set(name, value),
    addEventListener: (name, fn) => reloadListeners.set(name, fn),
    removeEventListener: (name, fn) => {
      if (reloadListeners.get(name) === fn) reloadListeners.delete(name);
    },
  };
  const noteChoice = {
    disabled: true, value: '', options: [],
    setAttribute: (name, value) => choiceAttributes.set(name, value),
    addEventListener: (name, fn) => choiceListeners.set(name, fn),
    removeEventListener: (name, fn) => {
      if (choiceListeners.get(name) === fn) choiceListeners.delete(name);
    },
    replaceChildren(...options) { this.options = options; this.value = options[0]?.value ?? ''; },
    set innerHTML(_value) { throw new Error('Note choices must use text nodes'); },
  };
  const reconnectButton = {
    disabled: true, hidden: true,
    setAttribute: (name, value) => reconnectAttributes.set(name, value),
    addEventListener: (name, fn) => reconnectListeners.set(name, fn),
    removeEventListener: (name, fn) => {
      if (reconnectListeners.get(name) === fn) reconnectListeners.delete(name);
    },
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
  const document = { getElementById: id => ({
    'note-frame': canvas, 'session-status': status, 'reload-note': reloadButton,
    'reconnect-view': reconnectButton, 'note-choice': noteChoice,
    ...scrollButtons,
  })[id], createElement: name => {
    assert.equal(name, 'option');
    return { value: '', textContent: '',
      set innerHTML(_value) { throw new Error('Note labels must use textContent'); } };
  } };
  const click = (overrides = {}) => listeners.get('click')?.({
    clientX: 78, clientY: 82, button: 0, isPrimary: false, ...overrides,
  });
  function key(key, overrides = {}) {
    let prevented = false;
    listeners.get('keydown')?.({ key, repeat: false,
      preventDefault: () => { prevented = true; }, ...overrides });
    return prevented;
  }
  function wheel(overrides = {}) {
    let prevented = false;
    listeners.get('wheel')?.({ deltaY: 40, deltaMode: 0, ctrlKey: false,
      preventDefault: () => { prevented = true; }, ...overrides });
    return prevented;
  }
  return { window, document, fetch, attributes, listeners, listenerOptions, painted, status,
    reloadButton, reloadListeners, reloadAttributes, reload: () => reloadListeners.get('click')?.(),
    reconnectButton, reconnectListeners, reconnectAttributes,
    reconnect: () => reconnectListeners.get('click')?.(),
    scrollButtons, scrollListeners, tapScroll: id => scrollListeners.get(id)?.(),
    noteChoice, choiceListeners, choiceAttributes, select: id => {
      noteChoice.value = String(id);
      return choiceListeners.get('change')?.();
    },
    requests, replies, data, click, key, wheel, focused: () => focused };
}

function scrollDisabled(b) {
  return scrollControlIds.map(id => b.scrollButtons[id].disabled);
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

test('wheel units map to bounded native movement without intercepting zoom or invalid input', () => {
  const rect = { height: 320 };
  assert.equal(wheelDeltaToNative({ deltaY: 80, deltaMode: 0 }, rect), 40);
  assert.equal(wheelDeltaToNative({ deltaY: -80, deltaMode: 0 }, rect), -40);
  assert.equal(wheelDeltaToNative({ deltaY: 0.5, deltaMode: 0 }, rect), 0.25);
  assert.equal(wheelDeltaToNative({ deltaY: 3, deltaMode: 1 }, rect), 48);
  assert.equal(wheelDeltaToNative({ deltaY: -1, deltaMode: 2 }, rect), -160);
  assert.equal(wheelDeltaToNative({ deltaY: Number.MAX_VALUE, deltaMode: 2 }, rect), 2560);
  assert.equal(wheelDeltaToNative({ deltaY: -Number.MAX_VALUE, deltaMode: 0 }, rect), -2048);
  for (const patch of [{ deltaY: 0 }, { deltaY: NaN }, { deltaY: Infinity },
    { deltaY: -Infinity }, { deltaY: '4' }, { deltaMode: 3 }, { deltaMode: '0' },
    { ctrlKey: true }]) {
    assert.equal(wheelDeltaToNative({ deltaY: 40, deltaMode: 0, ...patch }, rect), null);
  }
  for (const badRect of [null, {}, { height: 0 }, { height: -1 }, { height: NaN },
    { height: Infinity }]) {
    assert.equal(wheelDeltaToNative({ deltaY: 40, deltaMode: 0 }, badRect), null);
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
  assert.doesNotThrow(() => decodeLiveFrame(scrollingFrame({ scrollY: 400 })));
  assert.doesNotThrow(() => decodeLiveFrame(emptyFrame()));
  assert.equal(decodeLiveFrame(emptyFrame()).canReload, true);
  const supplied = frame({ controlPoint: { x: 85, y: 0 } });
  decodeLiveFrame(supplied).controlPoint.x = 0;
  assert.deepEqual(supplied.controlPoint, { x: 85, y: 0 });
  for (const invalid of [null, [], frame({ schemaVersion: 1 }), frame({ schemaVersion: 2 }),
    frame({ schemaVersion: 3 }),
    ...[undefined, null, -1, 0.5, '1', Number.MAX_SAFE_INTEGER + 1]
      .map(inputGeneration => frame({ inputGeneration })),
    frame({ width: 640 }),
    frame({ height: 159 }), frame({ sequence: -1 }), frame({ sequence: '0' }),
    frame({ sequence: 257, remainingEvents: 0 }), frame({ expanded: 2 }),
    frame({ expanded: 0 }), frame({ visibleCount: 0 }), frame({ visibleCount: 65 }),
    frame({ visibleCount: 0.5 }), frame({ canActivate: 1 }),
    frame({ remainingEvents: -1 }), frame({ remainingEvents: 257 }),
    frame({ remainingEvents: 0 }), frame({ scrollY: -1 }), frame({ scrollY: 1 }),
    frame({ maxScrollY: -1 }), frame({ maxScrollY: 0x100000000 }),
    frame({ scrollY: 0.5, maxScrollY: 1 }), frame({ maxScrollY: '0' }),
    frame({ canScroll: 1 }), frame({ canScroll: true }),
    frame({ canReload: 1 }), frame({ canReload: undefined }),
    frame({ sequence: 256, remainingEvents: 0, canReload: true }),
    frame({ sequence: 256, remainingEvents: 0, canActivate: true }),
    scrollingFrame({ sequence: 256, canScroll: true }),
    frame({ controlPoint: null }), frame({ controlPoint: undefined }),
    frame({ controlPoint: [] }), frame({ controlPoint: { x: 1 } }),
    frame({ controlPoint: { x: 1, y: 2, extra: 3 } }),
    frame({ controlPoint: { x: -1, y: 0 } }), frame({ controlPoint: { x: 320, y: 0 } }),
    frame({ controlPoint: { x: 0, y: 160 } }), frame({ controlPoint: { x: 0, y: 0.5 } }),
    frame({ pixels: pixels.slice(1) }),
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
  assert.equal(b.requests[0].path, '/sync');
  assert.equal(b.requests[0].method, 'POST');
  assert.equal(b.requests[0].body, '{}');
  assert.deepEqual(b.requests[0].headers, { 'x-a1-session': token, accept: 'application/json',
    'content-type': 'application/json' });
  assert.equal(b.requests[0].credentials, 'omit');
  assert.equal(b.requests[0].mode, 'same-origin');
  assert.equal(b.requests[0].redirect, 'error');
  assert.equal(b.requests[0].referrerPolicy, 'same-origin');
  assert.equal(b.requests[0].referrer, '');
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
  assert.equal(b.requests[1].headers['x-a1-input-generation'], '1');
  assert.equal(b.requests[1].referrerPolicy, 'same-origin');
  assert.equal(b.requests[1].referrer, '');
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
  b.click({ button: 1 });
  b.click({ clientX: 670 });
  b.click({ clientY: 370 });
  b.key('Enter', { repeat: true });
  b.key('Escape');
  assert.equal(b.requests.length, count);
  assert.ok(b.focused() > 0);
});

test('click activation accepts default pointer fields and legacy mouse events while excluding auxiliary buttons', async t => {
  const b = browser();
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  for (const [sequence, isPrimary] of [[1, false], [2, undefined], [3, true]]) {
    b.replies.push(response(frame({ sequence })));
    b.click({ isPrimary });
    await settle();
    assert.equal(b.requests.length, sequence + 1);
    assert.equal(b.requests.at(-1).path, '/input');
    assert.equal(b.requests.at(-1).body, '{"x":24,"y":16}');
  }
  for (const button of [1, 2]) b.click({ button });
  assert.equal(b.requests.length, 4);
});

test('keyboard activation uses the latest native control point, including a clipped title', async t => {
  const b = browser();
  b.replies.push(response(frame({ controlPoint: { x: 85, y: 0 } })));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  b.replies.push(response(frame({ sequence: 1, expanded: 0, visibleCount: 0,
    controlPoint: { x: 103, y: 4 } })));
  assert.equal(b.key('Enter'), true);
  assert.equal(b.requests[1].body, '{"x":85,"y":0}');
  await settle();
  b.replies.push(response(frame({ sequence: 2 })));
  assert.equal(b.key(' '), true);
  assert.equal(b.requests[2].body, '{"x":103,"y":4}');
  await settle();
});

test('wheel bursts coalesce one pending target and keyboard scrolling works with the title offscreen', async t => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  assert.deepEqual(b.listenerOptions.get('wheel'), { passive: false });
  const first = deferred();
  const second = deferred();
  b.replies.push(first.promise, second.promise);
  assert.equal(b.wheel({ deltaY: 80 }), true);
  assert.equal(b.requests[1].path, '/scroll');
  assert.equal(b.requests[1].body, '{"y":40}');
  for (let index = 0; index < 100; index++) assert.equal(b.wheel({ deltaY: 80 }), true);
  assert.equal(b.requests.length, 2, 'the burst retains only a target while the first request runs');
  first.resolve(response(scrollingFrame({ sequence: 1, scrollY: 40 })));
  await settle();
  assert.equal(b.requests.length, 3);
  assert.equal(b.requests[2].body, '{"y":400}', 'the one pending target is clamped');
  assert.equal(b.key('Enter'), false, 'an offscreen title has no keyboard activation point');
  b.click();
  assert.equal(b.requests.length, 3);
  b.wheel({ deltaY: 80 });
  second.resolve(response(scrollingFrame({ sequence: 2, scrollY: 400 })));
  await settle();
  assert.equal(b.requests.length, 3, 'a pending target already reached needs no extra request');
  assert.equal(b.attributes.get('aria-disabled'), 'false', 'scrolling remains available offscreen');
  assert.equal(b.attributes.get('aria-label'), 'Scrollable note outline');
  assert.match(b.status.textContent, /At the end/);

  let sequence = 2;
  for (const [key, y] of [['Home', 0], ['ArrowDown', 16], ['PageDown', 176],
    ['End', 400], ['PageUp', 240], ['ArrowUp', 224]]) {
    b.replies.push(response(scrollingFrame({ sequence: ++sequence, scrollY: y })));
    assert.equal(b.key(key), true);
    assert.equal(b.requests.at(-1).path, '/scroll');
    assert.equal(b.requests.at(-1).body, JSON.stringify({ y }));
    await settle();
  }
  b.replies.push(response(scrollingFrame({ sequence: ++sequence, scrollY: 208 })));
  assert.equal(b.key('ArrowUp', { repeat: true }), true);
  await settle();
  assert.equal(b.requests.at(-1).body, '{"y":208}', 'held arrows remain bounded user input');
  const count = b.requests.length;
  assert.equal(b.key('PageDown', { ctrlKey: true }), false);
  assert.equal(b.wheel({ ctrlKey: true }), false);
  assert.equal(b.wheel({ deltaY: Infinity }), false);
  assert.equal(b.requests.length, count);
});

test('visible scroll controls use native page steps and stop at both boundaries without moving focus', async t => {
  const b = browser();
  const initial = deferred();
  b.replies.push(initial.promise);
  const client = startLiveClient(b);
  t.after(client.dispose);
  assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
  for (const id of scrollControlIds) b.tapScroll(id);
  assert.equal(b.requests.length, 1);
  initial.resolve(response(scrollingFrame()));
  await client.ready;
  assert.deepEqual(scrollDisabled(b), [true, true, false, false]);
  b.tapScroll('scroll-start');
  b.tapScroll('scroll-up');
  assert.equal(b.requests.length, 1, 'top-boundary taps do not consume native attempts');

  let sequence = 0;
  for (const [id, y] of [['scroll-down', 160], ['scroll-down', 320],
    ['scroll-down', 400], ['scroll-up', 240], ['scroll-start', 0], ['scroll-end', 400]]) {
    b.replies.push(response(scrollingFrame({ sequence: ++sequence, scrollY: y })));
    b.tapScroll(id);
    assert.equal(b.requests.at(-1).path, '/scroll');
    assert.equal(b.requests.at(-1).body, JSON.stringify({ y }));
    await settle();
    assert.deepEqual(scrollDisabled(b), [y === 0, y === 0, y === 400, y === 400]);
  }
  b.tapScroll('scroll-down');
  b.tapScroll('scroll-end');
  assert.equal(b.requests.length, 7, 'bottom-boundary taps also avoid zero-distance requests');
  assert.equal(b.focused(), 0, 'button activation keeps focus on the native HTML button');
});

test('scroll button bursts coalesce behind the active target and allow reversing pending movement', async t => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const first = deferred();
  const second = deferred();
  const third = deferred();
  b.replies.push(first.promise, second.promise, third.promise);
  b.tapScroll('scroll-down');
  assert.equal(b.requests[1].body, '{"y":160}');
  assert.deepEqual(scrollDisabled(b), [false, false, false, false]);
  b.tapScroll('scroll-down');
  b.tapScroll('scroll-end');
  assert.deepEqual(scrollDisabled(b), [false, false, true, true]);
  assert.equal(b.requests.length, 2, 'taps retain one target while a scroll is running');
  first.resolve(response(scrollingFrame({ sequence: 1, scrollY: 160 })));
  await settle();
  assert.equal(b.requests[2].body, '{"y":400}');
  b.tapScroll('scroll-up');
  assert.deepEqual(scrollDisabled(b), [false, false, false, false]);
  second.resolve(response(scrollingFrame({ sequence: 2, scrollY: 400 })));
  await settle();
  assert.equal(b.requests[3].body, '{"y":240}');
  third.resolve(response(scrollingFrame({ sequence: 3, scrollY: 240 })));
  await settle();
  assert.equal(b.requests.length, 4);
  assert.deepEqual(scrollDisabled(b), [false, false, false, false]);
});

test('scroll buttons queue behind pointer input only while the resulting outline can still scroll', async t => {
  for (const collapsed of [false, true]) await t.test(String(collapsed), async t => {
    const b = browser();
    b.replies.push(response(scrollingFrame()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    const pointer = deferred();
    b.replies.push(pointer.promise);
    if (!collapsed) b.replies.push(response(scrollingFrame({ sequence: 2, scrollY: 160 })));
    b.click();
    assert.equal(b.requests[1].path, '/input');
    assert.deepEqual(scrollDisabled(b), [true, true, false, false]);
    b.tapScroll('scroll-down');
    assert.equal(b.requests.length, 2);
    pointer.resolve(response(collapsed ? frame({ sequence: 1, expanded: 0, visibleCount: 0 })
      : scrollingFrame({ sequence: 1 })));
    await settle();
    if (collapsed) {
      assert.equal(b.requests.length, 2, 'collapse discards a target outside the new outline');
      assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
    } else {
      assert.equal(b.requests.length, 3);
      assert.equal(b.requests[2].body, '{"y":160}');
      assert.deepEqual(scrollDisabled(b), [false, false, false, false]);
    }
  });
});

test('document replacement disables scroll buttons and never queues old-document taps', async t => {
  for (const kind of ['reload', 'select']) await t.test(kind, async t => {
    const b = browser();
    const scrolling = overrides => selectionFrame({ maxScrollY: 400, canScroll: true, ...overrides });
    b.replies.push(response(scrolling()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    const replacement = deferred();
    b.replies.push(replacement.promise);
    const replacing = kind === 'reload' ? b.reload() : b.select(1);
    assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
    for (const id of scrollControlIds) b.tapScroll(id);
    assert.equal(b.requests.length, 2);
    replacement.resolve(response(scrolling({ sequence: 1, selectedNoteId: kind === 'select' ? 1 : 0 })));
    await replacing;
    assert.equal(b.requests.length, 2, 'document replacement does not drain ignored taps');
    assert.deepEqual(scrollDisabled(b), [true, true, false, false]);
  });
});

test('scroll controls stay disabled without scroll authority, overflow or remaining budget', async t => {
  const states = [frame(), emptyFrame(), scrollingFrame({ canScroll: false, canActivate: false,
    canReload: false, controlPoint: null }), scrollingFrame({ sequence: 256, remainingEvents: 0 })];
  for (const state of states) {
    const b = browser();
    b.replies.push(response(state));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
    for (const id of scrollControlIds) b.tapScroll(id);
    assert.equal(b.requests.length, 1);
  }
});

test('uncertain scroll taps are discarded through disconnect and synchronization', async t => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const lost = deferred();
  b.replies.push(lost.promise);
  b.tapScroll('scroll-down');
  b.tapScroll('scroll-end');
  lost.reject(new Error('scroll reply lost'));
  await settle();
  assert.equal(b.painted.length, 1, 'the last completed native frame stays visible');
  assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
  for (const id of scrollControlIds) b.tapScroll(id);
  assert.equal(b.requests.length, 2);

  const sync = deferred();
  b.replies.push(sync.promise);
  const reconnecting = b.reconnect();
  assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
  for (const id of scrollControlIds) b.tapScroll(id);
  sync.resolve(response(scrollingFrame({ sequence: 1, inputGeneration: 2, scrollY: 160 })));
  await reconnecting;
  assert.deepEqual(b.requests.map(request => request.path), ['/sync', '/scroll', '/sync']);
  assert.deepEqual(scrollDisabled(b), [false, false, false, false]);
  b.replies.push(response(scrollingFrame({ sequence: 2, inputGeneration: 2, scrollY: 320 })));
  b.tapScroll('scroll-down');
  assert.equal(b.requests[3].body, '{"y":320}', 'a new tap uses the reconciled native position');
  await settle();
});

test('disposal removes scroll button listeners and prevents queued or late scroll work', async () => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  await client.ready;
  const pending = deferred();
  b.replies.push(pending.promise);
  const staleListener = b.scrollListeners.get('scroll-down');
  b.tapScroll('scroll-down');
  b.tapScroll('scroll-end');
  client.dispose();
  assert.equal(b.scrollListeners.size, 0);
  assert.deepEqual(scrollDisabled(b), [true, true, true, true]);
  assert.equal(b.requests[1].signal.aborted, true);
  staleListener();
  pending.resolve(response(scrollingFrame({ sequence: 1, scrollY: 160 })));
  await settle();
  assert.equal(b.requests.length, 2);
  assert.equal(b.painted.length, 1);
});

test('fractional wheel movement accumulates without issuing zero-distance requests', async t => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  assert.equal(b.wheel({ deltaY: 0.5 }), true);
  assert.equal(b.requests.length, 1);
  b.replies.push(response(scrollingFrame({ sequence: 1, scrollY: 1 })));
  assert.equal(b.wheel({ deltaY: 0.5 }), true);
  assert.equal(b.requests[1].body, '{"y":1}');
  await settle();
  b.replies.push(response(scrollingFrame({ sequence: 2, scrollY: 0 })));
  assert.equal(b.key('Home'), true);
  await settle();
  const count = b.requests.length;
  assert.equal(b.wheel({ deltaY: -100 }), true, 'the viewport contains valid wheel input at its edge');
  assert.equal(b.key('ArrowUp'), true);
  assert.equal(b.requests.length, count, 'the top boundary does not create native attempts');
});

test('rejected or uncertain scrolling discards its pending target without an automatic retry', async t => {
  for (const code of [403, 409, 429, 500, 503, 'network']) await t.test(String(code), async t => {
    const b = browser();
    b.replies.push(response(scrollingFrame()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    const pending = deferred();
    b.replies.push(pending.promise);
    b.wheel();
    b.wheel();
    assert.equal(b.requests.length, 2);
    if (code === 'network') pending.reject(new Error('connection lost after sending scroll'));
    else pending.resolve(response({ error: 'Scroll failed', frame: scrollingFrame({
      sequence: 1, canScroll: code !== 409 && code !== 503,
      canActivate: code !== 409 && code !== 503,
    }) }, code));
    await settle();
    assert.equal(b.requests.length, 2, 'failed scrolling never drains the pending target');
    if (code === 429 || code === 500) {
      assert.equal(b.status.textContent, 'Scroll failed');
      b.replies.push(response(scrollingFrame({ sequence: 2, scrollY: 20 })));
      assert.equal(b.wheel(), true);
      assert.equal(b.requests[2].body, '{"y":20}', 'new input starts from the actual retained frame');
      await settle();
    } else {
      assert.equal(b.wheel(), false);
      b.key('PageDown');
      assert.equal(b.requests.length, 2);
      if (code === 403) assert.equal(b.data.size, 0);
    }
  });
});

test('disposing during a scroll aborts the request and discards its coalesced target', async () => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  await client.ready;
  const pending = deferred();
  b.replies.push(pending.promise);
  b.wheel({ deltaY: 100 });
  b.wheel({ deltaY: 100 });
  client.dispose();
  assert.equal(b.requests[1].signal.aborted, true);
  assert.equal(b.listeners.size, 0);
  pending.resolve(response(scrollingFrame({ sequence: 1, scrollY: 50 })));
  await settle();
  assert.equal(b.painted.length, 1);
  assert.equal(b.requests.length, 2);
});

test('reload is available for empty and offscreen outlines and uses the new native frame', async t => {
  for (const state of [emptyFrame(), scrollingFrame({ scrollY: 400 })]) {
    const b = browser();
    b.replies.push(response(state));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.reloadButton.disabled, false);
    if (state.visibleCount === 0) {
      assert.equal(b.attributes.get('aria-disabled'), 'true');
      assert.match(b.status.textContent, /No headings.*reload/);
    }
    const pending = deferred();
    b.replies.push(pending.promise);
    const reloading = b.reload();
    b.reload();
    b.click();
    b.key('Enter');
    b.wheel();
    b.key('End');
    assert.equal(b.requests.length, 2);
    assert.equal(b.reloadButton.disabled, true);
    assert.equal(b.reloadAttributes.get('aria-busy'), 'true');
    assert.equal(b.requests[1].path, '/reload');
    assert.equal(b.requests[1].method, 'POST');
    assert.equal(b.requests[1].body, '{}');
    assert.equal(b.requests[1].headers['x-a1-session'], token);
    assert.equal(b.requests[1].headers['content-type'], 'application/json');
    assert.match(b.status.textContent, /Reloading note/);

    const nextPixels = Buffer.alloc(rgb.length, 55).toString('base64');
    const point = { x: 101, y: 8 };
    pending.resolve(response(frame({ sequence: 1, visibleCount: 3,
      pixels: nextPixels, controlPoint: point })));
    await reloading;
    assert.equal(b.requests.length, 2, 'no movement against the old document follows the reload');
    assert.equal(b.reloadButton.disabled, false);
    assert.equal(b.reloadAttributes.get('aria-busy'), 'false');
    assert.deepEqual([...b.painted.at(-1).data.subarray(0, 4)], [55, 55, 55, 255]);
    assert.match(b.status.textContent, /Note reloaded.*3 headings/);
    assert.equal(b.data.get(tokenKey), token);
    b.replies.push(response(frame({ sequence: 2, expanded: 0, visibleCount: 0 })));
    assert.equal(b.key('Enter'), true);
    await settle();
    assert.equal(b.requests[2].path, '/input');
    assert.deepEqual(JSON.parse(b.requests[2].body), point);
  }
});

test('reload waits for existing scroll work and drops fractional and queued movement at the reload boundary', async t => {
  const b = browser();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const first = deferred();
  b.replies.push(first.promise);
  b.wheel();
  b.wheel();
  b.reload();
  assert.equal(b.requests.length, 2);
  assert.equal(b.reloadButton.disabled, true);
  const queued = deferred();
  b.replies.push(queued.promise);
  first.resolve(response(scrollingFrame({ sequence: 1, scrollY: 20 })));
  await settle();
  assert.equal(b.requests.length, 3);
  assert.equal(b.requests[2].body, '{"y":40}');
  b.reload();
  assert.equal(b.requests.length, 3, 'a reload is not queued behind the pending wheel target');
  assert.equal(b.reloadButton.disabled, true);
  queued.resolve(response(scrollingFrame({ sequence: 2, scrollY: 40 })));
  await settle();
  assert.equal(b.reloadButton.disabled, false);

  b.wheel({ deltaY: 0.5 });
  assert.equal(b.requests.length, 3, 'the old document has a fractional wheel remainder');
  const pendingReload = deferred();
  b.replies.push(pendingReload.promise);
  const reloading = b.reload();
  assert.equal(b.wheel({ deltaY: 0.5 }), true);
  assert.equal(b.key('End'), true);
  assert.equal(b.key('PageDown'), true);
  assert.equal(b.wheel({ ctrlKey: true }), false);
  assert.equal(b.key('Home', { ctrlKey: true }), false);
  b.reload();
  assert.equal(b.requests.length, 4);
  pendingReload.resolve(response(scrollingFrame({ sequence: 3, scrollY: 40, maxScrollY: 800 })));
  await reloading;
  assert.equal(b.requests.length, 4);
  b.wheel({ deltaY: 0.5 });
  assert.equal(b.requests.length, 4, 'the old fractional remainder was also discarded');
  b.replies.push(response(scrollingFrame({ sequence: 4, scrollY: 41, maxScrollY: 800 })));
  b.wheel({ deltaY: 0.5 });
  await settle();
  assert.equal(b.requests[4].body, '{"y":41}');
  b.replies.push(response(scrollingFrame({ sequence: 5, scrollY: 800, maxScrollY: 800 })));
  assert.equal(b.key('End'), true);
  await settle();
  assert.equal(b.requests[5].body, '{"y":800}', 'later movement uses the reloaded native maximum');
});

test('reload is disabled during activation and remains available after an empty replacement', async t => {
  const b = browser();
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const activation = deferred();
  b.replies.push(activation.promise);
  b.click();
  b.reload();
  assert.equal(b.reloadButton.disabled, true);
  assert.equal(b.requests.length, 2);
  activation.resolve(response(frame({ sequence: 1, expanded: 0, visibleCount: 0 })));
  await settle();
  assert.equal(b.reloadButton.disabled, false);
  b.replies.push(response(emptyFrame({ sequence: 2 })));
  await b.reload();
  assert.equal(b.reloadButton.disabled, false);
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.match(b.status.textContent, /Note reloaded.*No headings/);
  b.replies.push(response(frame({ sequence: 3, expanded: 0, visibleCount: 0 })));
  await b.reload();
  assert.equal(b.attributes.get('aria-label'), 'Show outline');
  assert.match(b.status.textContent, /Outline collapsed/);
  assert.equal(b.requests.length, 4);
});

test('failed reloads never retry and recoverable errors preserve manual reload access', async t => {
  for (const code of [403, 409, 429, 500, 503, 'network', 'stale']) await t.test(String(code), async t => {
    const b = browser();
    b.replies.push(response(scrollingFrame({ sequence: 2, scrollY: 400 })));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    const pending = deferred();
    b.replies.push(pending.promise);
    const reloading = b.reload();
    b.wheel();
    b.key('Home');
    if (code === 'network') pending.reject(new Error('connection lost after sending reload'));
    else if (code === 'stale') pending.resolve(response(emptyFrame({ sequence: 1 })));
    else pending.resolve(response({ error: 'Reload failed', frame: scrollingFrame({
      sequence: 3, scrollY: 400, canScroll: ![409, 503].includes(code),
      canReload: ![409, 503].includes(code),
    }) }, code));
    await reloading;
    assert.equal(b.requests.length, 2, 'no retry or old scroll target follows a failed reload');
    assert.equal(b.data.has(tokenKey), code !== 403);
    if ([429, 500].includes(code)) {
      assert.equal(b.reloadButton.disabled, false);
      b.replies.push(response(scrollingFrame({ sequence: 4, scrollY: 400 })));
      await b.reload();
      assert.equal(b.requests.length, 3);
      assert.equal(b.requests[2].path, '/reload');
    } else {
      assert.equal(b.reloadButton.disabled, true);
      b.reload();
      assert.equal(b.requests.length, 2);
    }
    if (code === 'stale') {
      assert.equal(b.painted.length, 1);
      assert.equal(b.attributes.get('aria-label'), 'Scrollable note outline');
    }
  });
});

test('disposing during a reload aborts it and removes the reload control handler', async () => {
  const b = browser();
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  await client.ready;
  const pending = deferred();
  b.replies.push(pending.promise);
  const reloading = b.reload();
  client.dispose();
  assert.equal(b.requests[1].signal.aborted, true);
  assert.equal(b.listeners.size, 0);
  assert.equal(b.reloadListeners.size, 0);
  assert.equal(b.reloadButton.disabled, true);
  pending.resolve(response(emptyFrame({ sequence: 1 })));
  await reloading;
  assert.equal(b.painted.length, 1);
  assert.equal(b.requests.length, 2);
});

test('frame catalogs own bounded labels and require a valid committed selection', () => {
  const supplied = selectionFrame({ noteChoices: [{ id: 0, label: 'A' }, { id: 1, label: 'é'.repeat(128) }] });
  const decoded = decodeLiveFrame(supplied);
  decoded.noteChoices[0].label = 'Changed';
  decoded.noteChoices.push({ id: 2, label: 'Extra' });
  assert.deepEqual(supplied.noteChoices, [{ id: 0, label: 'A' }, { id: 1, label: 'é'.repeat(128) }]);
  assert.doesNotThrow(() => decodeLiveFrame(emptyFrame({
    noteChoices: [], selectedNoteId: null, canReload: false,
  })));
  for (const invalid of [
    frame({ noteChoices: undefined }), frame({ noteChoices: null }), frame({ noteChoices: {} }),
    frame({ noteChoices: Array(1) }), frame({ noteChoices: [null] }),
    selectionFrame({ noteChoices: Array.from({ length: 9 }, (_, id) => ({ id, label: 'Note' })) }),
    frame({ noteChoices: [{ id: '0', label: 'Note' }] }),
    frame({ noteChoices: [{ id: 1, label: 'Note' }] }),
    selectionFrame({ noteChoices: [{ id: 0, label: 'A' }, { id: 0, label: 'B' }] }),
    frame({ noteChoices: [{ id: 0, label: 'Note', path: '/private/note.md' }] }),
    frame({ noteChoices: [{ id: 0, label: '' }] }),
    frame({ noteChoices: [{ id: 0, label: 'é'.repeat(129) }] }),
    frame({ noteChoices: [{ id: 0, label: 'x'.repeat(257) }] }),
    frame({ noteChoices: [{ id: 0, label: 'bad\nlabel' }] }),
    frame({ noteChoices: [{ id: 0, label: 'bad\u007flabel' }] }),
    frame({ noteChoices: [{ id: 0, label: 12 }] }),
    frame({ noteChoices: [], selectedNoteId: 0 }),
    frame({ selectedNoteId: null }), frame({ selectedNoteId: -1 }),
    frame({ selectedNoteId: 1 }), frame({ selectedNoteId: '0' }),
    frame({ canSelectNote: true }), selectionFrame({ canSelectNote: 1 }),
    selectionFrame({ canSelectNote: undefined }),
    selectionFrame({ sequence: 256, remainingEvents: 0, canSelectNote: true }),
  ]) assert.throws(() => decodeLiveFrame(invalid), /Invalid A1 live/);
});

test('the largest escaped note catalog fits the existing bounded response reader', async t => {
  const label = ('\\' + '"').repeat(128);
  assert.equal(Buffer.byteLength(label), 256);
  const noteChoices = Array.from({ length: 8 }, (_, id) => ({ id, label }));
  const state = selectionFrame({ noteChoices, selectedNoteId: 7 });
  const failure = { error: '\\'.repeat(256), frame: { ...state, sequence: 1, remainingEvents: 255 } };
  assert.ok(Buffer.byteLength(JSON.stringify(failure)) < 220_000);
  const b = browser();
  b.replies.push(response(state));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  assert.equal(b.painted.length, 1);
  assert.equal(b.noteChoice.value, '7');
  assert.equal(b.noteChoice.options.length, 8);
  assert.ok(b.noteChoice.options.every(option => option.textContent === label));
  b.replies.push(response(failure, 500));
  await b.select(0);
  assert.equal(b.painted.length, 2);
  assert.equal(b.noteChoice.value, '7');
  assert.equal(b.noteChoice.disabled, false);
});

test('the chooser commits the selected note with its native pixels, including empty and offscreen views', async t => {
  for (const state of [
    selectionFrame({ expanded: 0, visibleCount: 0, canActivate: false, controlPoint: null }),
    selectionFrame({ scrollY: 400, maxScrollY: 400, canScroll: true, canActivate: false, controlPoint: null }),
  ]) {
    const b = browser();
    b.replies.push(response(state));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.noteChoice.disabled, false);
    assert.equal(b.noteChoice.value, '0');
    assert.deepEqual(b.noteChoice.options.map(option => [option.value, option.textContent]),
      [['0', 'First note'], ['1', '<b>Second note</b>']]);
    const pending = deferred();
    b.replies.push(pending.promise);
    const selecting = b.select(1);
    assert.equal(b.noteChoice.value, '0', 'the native select is restored before the request settles');
    assert.equal(b.noteChoice.disabled, true);
    assert.equal(b.choiceAttributes.get('aria-busy'), 'true');
    assert.equal(b.reloadButton.disabled, true);
    assert.equal(b.painted.length, 1);
    b.select(1);
    b.reload();
    b.click();
    b.key('Enter');
    b.wheel();
    b.key('End');
    assert.equal(b.noteChoice.value, '0', 'ignored changes also restore the committed ID');
    assert.equal(b.requests.length, 2);
    assert.equal(b.requests[1].path, '/select');
    assert.equal(b.requests[1].method, 'POST');
    assert.equal(b.requests[1].body, '{"id":1}');
    assert.equal(b.requests[1].headers['x-a1-session'], token);
    assert.equal(b.requests[1].headers['content-type'], 'application/json');
    const point = { x: 101, y: 8 };
    pending.resolve(response(selectionFrame({ sequence: 1, selectedNoteId: 1, visibleCount: 3,
      controlPoint: point, pixels: Buffer.alloc(rgb.length, 55).toString('base64') })));
    await selecting;
    assert.equal(b.requests.length, 2, 'old scroll intent does not follow a document switch');
    assert.equal(b.noteChoice.value, '1');
    assert.equal(b.noteChoice.disabled, false);
    assert.equal(b.choiceAttributes.get('aria-busy'), 'false');
    assert.deepEqual([...b.painted.at(-1).data.subarray(0, 4)], [55, 55, 55, 255]);
    assert.match(b.status.textContent, /Note opened.*3 headings/);
    assert.equal(b.data.get(tokenKey), token);
    b.select(1);
    b.select('1.0');
    b.select(7);
    assert.equal(b.requests.length, 2, 'same and unknown choices send no browser request');
    assert.equal(b.noteChoice.value, '1');
    b.replies.push(response(selectionFrame({ sequence: 2, selectedNoteId: 1,
      expanded: 0, visibleCount: 0 })));
    assert.equal(b.key('Enter'), true);
    await settle();
    assert.deepEqual(JSON.parse(b.requests[2].body), point);
  }
});

test('selection waits for active scrolls and clears fractional movement across documents', async t => {
  const b = browser();
  const scrolling = overrides => selectionFrame({ maxScrollY: 400, canScroll: true, ...overrides });
  b.replies.push(response(scrolling()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const first = deferred();
  const queued = deferred();
  b.replies.push(first.promise, queued.promise);
  b.wheel();
  b.wheel();
  b.select(1);
  assert.equal(b.noteChoice.value, '0');
  assert.equal(b.noteChoice.disabled, true);
  first.resolve(response(scrolling({ sequence: 1, scrollY: 20 })));
  await settle();
  assert.equal(b.requests.length, 3);
  assert.equal(b.requests[2].body, '{"y":40}');
  b.select(1);
  queued.resolve(response(scrolling({ sequence: 2, scrollY: 40 })));
  await settle();
  assert.equal(b.requests.length, 3, 'busy selection is not queued after scrolling');
  b.wheel({ deltaY: 0.5 });
  const replacement = deferred();
  b.replies.push(replacement.promise);
  const selecting = b.select(1);
  assert.equal(b.wheel({ deltaY: 0.5 }), true);
  assert.equal(b.key('End'), true);
  assert.equal(b.key('PageDown'), true);
  assert.equal(b.wheel({ ctrlKey: true }), false);
  assert.equal(b.key('Home', { ctrlKey: true }), false);
  replacement.resolve(response(scrolling({ sequence: 3, selectedNoteId: 1, scrollY: 0, maxScrollY: 800 })));
  await selecting;
  assert.equal(b.requests.length, 4);
  b.wheel({ deltaY: 0.5 });
  assert.equal(b.requests.length, 4, 'fractional movement from the previous note is discarded');
  b.replies.push(response(scrolling({ sequence: 4, selectedNoteId: 1, scrollY: 1, maxScrollY: 800 })));
  b.wheel({ deltaY: 0.5 });
  await settle();
  assert.equal(b.requests[4].body, '{"y":1}');
});

test('selection is not queued behind activation or reload', async t => {
  for (const kind of ['input', 'reload']) {
    const b = browser();
    b.replies.push(response(selectionFrame()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    const pending = deferred();
    b.replies.push(pending.promise);
    if (kind === 'input') b.click();
    else b.reload();
    b.select(1);
    assert.equal(b.requests[1].path, '/' + kind);
    assert.equal(b.requests.length, 2);
    assert.equal(b.noteChoice.value, '0');
    assert.equal(b.noteChoice.disabled, true);
    pending.resolve(response(selectionFrame({ sequence: 1 })));
    await settle();
    assert.equal(b.requests.length, 2);
    assert.equal(b.noteChoice.value, '0');
    assert.equal(b.noteChoice.disabled, false);
  }
});

test('failed selection keeps the committed note and never retries an uncertain switch', async t => {
  for (const code of [403, 409, 429, 500, 503, 'network', 'stale']) await t.test(String(code), async t => {
    const b = browser();
    b.replies.push(response(selectionFrame({ sequence: 2, maxScrollY: 400, scrollY: 400,
      canScroll: true, canActivate: false, controlPoint: null })));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    const committed = b.painted[0];
    const pending = deferred();
    b.replies.push(pending.promise);
    const selecting = b.select(1);
    b.wheel();
    b.key('Home');
    if (code === 'network') pending.reject(new Error('connection lost after sending selection'));
    else if (code === 'stale') pending.resolve(response(selectionFrame({ sequence: 1, selectedNoteId: 1,
      pixels: Buffer.alloc(rgb.length, 90).toString('base64') })));
    else pending.resolve(response({ error: 'Selection failed', frame: selectionFrame({
      sequence: 3, maxScrollY: 400, scrollY: 400, canActivate: false, controlPoint: null,
      canScroll: ![409, 503].includes(code), canReload: ![409, 503].includes(code),
      canSelectNote: ![409, 503].includes(code),
    }) }, code));
    await selecting;
    assert.equal(b.requests.length, 2);
    assert.equal(b.noteChoice.value, '0');
    assert.equal(b.data.has(tokenKey), code !== 403);
    if ([429, 500].includes(code)) {
      assert.equal(b.noteChoice.disabled, false);
      b.replies.push(response(selectionFrame({ sequence: 4, selectedNoteId: 1 })));
      await b.select(1);
      assert.equal(b.requests.length, 3);
      assert.equal(b.noteChoice.value, '1');
    } else {
      assert.equal(b.noteChoice.disabled, true);
      b.select(1);
      assert.equal(b.requests.length, 2);
      assert.equal(b.noteChoice.value, '0');
    }
    if (code === 'stale') {
      assert.deepEqual(b.painted, [committed]);
      assert.equal(b.attributes.get('aria-label'), 'Scrollable note outline');
      assert.equal(b.key('Enter'), false);
    }
  });
});

test('a single note, denied authority and exhausted budgets disable the chooser without losing metadata', async t => {
  for (const state of [
    frame(),
    emptyFrame({ noteChoices: [], selectedNoteId: null, canReload: false }),
    selectionFrame({ canSelectNote: false, canActivate: false, canReload: false }),
    selectionFrame({ sequence: 256, remainingEvents: 0 }),
  ]) {
    const b = browser();
    b.replies.push(response(state));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.noteChoice.disabled, true);
    assert.equal(b.noteChoice.value, state.selectedNoteId === null ? '' : String(state.selectedNoteId));
    assert.deepEqual(b.noteChoice.options.map(option => option.textContent),
      state.noteChoices.length ? state.noteChoices.map(note => note.label) : ['No note available']);
    b.select(1);
    assert.equal(b.requests.length, 1);
    assert.equal(b.noteChoice.value, state.selectedNoteId === null ? '' : String(state.selectedNoteId));
  }
});

test('disposing during selection aborts it and keeps late pixels and choices unpublished', async () => {
  const b = browser();
  b.replies.push(response(selectionFrame()));
  const client = startLiveClient(b);
  await client.ready;
  const pending = deferred();
  b.replies.push(pending.promise);
  const selecting = b.select(1);
  client.dispose();
  assert.equal(b.requests[1].signal.aborted, true);
  assert.equal(b.choiceListeners.size, 0);
  assert.equal(b.noteChoice.disabled, true);
  pending.resolve(response(selectionFrame({ sequence: 1, selectedNoteId: 1 })));
  await selecting;
  assert.equal(b.painted.length, 1);
  assert.equal(b.noteChoice.value, '0');
  assert.equal(b.requests.length, 2);
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
  assert.match(b.status.textContent, /Reconnect to continue/);
});

test('an older scroll reply preserves committed viewport pixels and native control availability', async t => {
  const b = browser();
  b.replies.push(response(scrollingFrame({ sequence: 8, scrollY: 400 })));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const committed = b.painted[0];
  b.replies.push(response(scrollingFrame({ sequence: 7, scrollY: 0,
    pixels: Buffer.alloc(rgb.length, 90).toString('base64') })));
  assert.equal(b.wheel({ deltaY: -40 }), true);
  assert.equal(b.wheel({ deltaY: -40 }), true);
  await settle();
  assert.deepEqual(b.painted, [committed]);
  assert.equal(b.attributes.get('aria-label'), 'Scrollable note outline');
  assert.equal(b.attributes.get('aria-expanded'), undefined);
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.equal(b.key('Enter'), false, 'the stale visible title point is not retained');
  assert.equal(b.key('Home'), false);
  assert.equal(b.requests.length, 2, 'the queued target is discarded on stale-frame failure');
  assert.match(b.status.textContent, /Reconnect to continue/);
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
  assert.match(b.status.textContent, /Session closed.*Start a new session/);
  b.click();
  b.key('Enter');
  assert.equal(b.requests.length, 2);
});

test('denied and exhausted sessions retain their supplied view without forwarding input or reloads', async t => {
  for (const state of [frame({ canActivate: false, canReload: false }),
    frame({ sequence: 256, remainingEvents: 0 })]) {
    const b = browser();
    b.replies.push(response(state));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    assert.equal(b.painted.length, 1);
    assert.equal(b.attributes.get('aria-disabled'), 'true');
    assert.equal(b.reloadButton.disabled, true);
    b.click();
    b.key('Enter');
    b.key(' ');
    b.wheel();
    b.key('PageDown');
    b.reload();
    assert.equal(b.requests.length, 1);
    if (!state.canActivate) assert.equal(b.attributes.get('role'), 'img');
    if (state.remainingEvents === 0) assert.match(b.status.textContent, /input limit reached/);
  }
});

test('disposing the adapter aborts a pending request and removes all input handlers', async () => {
  const b = browser();
  const pending = deferred();
  b.replies.push(pending.promise);
  const client = startLiveClient(b);
  client.dispose();
  assert.equal(b.requests[0].signal.aborted, true);
  assert.equal(b.listeners.size, 0);
  assert.equal(b.choiceListeners.size, 0);
  assert.equal(b.noteChoice.disabled, true);
  assert.equal(b.reloadListeners.size, 0);
  assert.equal(b.reloadButton.disabled, true);
  pending.resolve(response(frame()));
  await client.ready;
  assert.equal(b.painted.length, 0);
});

test('the initial deadline settles stalled headers and ignores late pixels, authorization or rejection', async t => {
  for (const late of ['frame', 'authorization', 'rejection']) await t.test(late, async t => {
    const b = browser();
    const timers = deadlineClock();
    const pending = deferred();
    b.replies.push(pending.promise);
    const client = startLiveClient({ ...b, timers });
    t.after(client.dispose);
    assert.deepEqual(timers.delays, [65_000]);
    timers.advance(64_999);
    await settle();
    assert.equal(b.attributes.get('aria-busy'), 'true');
    assert.equal(b.requests[0].signal.aborted, false);
    timers.advance(1);
    await alreadySettled(client.ready);
    assert.equal(timers.pending.size, 0);
    assert.equal(b.requests[0].signal.aborted, true);
    assert.equal(b.attributes.get('aria-busy'), 'false');
    assert.equal(b.attributes.get('aria-disabled'), 'true');
    assert.equal(b.painted.length, 0);
    assert.equal(b.data.get(tokenKey), token);
    assert.match(b.status.textContent, /Connection interrupted/);
    const message = b.status.textContent;
    if (late === 'rejection') pending.reject(new Error('late transport failure'));
    else {
      const body = stalledBody(late === 'authorization' ? 403 : 200);
      pending.resolve(body.response);
      await settle();
      assert.equal(body.reads(), 0, 'a retired reply never starts its body reader');
      assert.ok(body.cancellations() > 0);
    }
    await settle();
    assert.equal(b.painted.length, 0);
    assert.equal(b.data.get(tokenKey), token, 'late authorization cannot erase current tab authority');
    assert.equal(b.status.textContent, message);
    b.click();
    b.reload();
    assert.equal(b.requests.length, 1, 'a deadline does not retry or admit new input');
  });
});

test('one deadline covers headers and a stalled body while preserving the committed frame', async t => {
  const b = browser();
  const timers = deadlineClock();
  b.replies.push(response(frame()));
  const client = startLiveClient({ ...b, timers });
  t.after(client.dispose);
  await client.ready;
  assert.equal(timers.pending.size, 0);
  const committed = b.painted[0];
  const headers = deferred();
  const body = stalledBody();
  b.replies.push(headers.promise);
  const reloading = b.reload();
  timers.advance(40_000);
  headers.resolve(body.response);
  await settle();
  assert.equal(body.reads(), 1);
  assert.equal(timers.pending.size, 1);
  timers.advance(24_999);
  await settle();
  assert.equal(b.attributes.get('aria-busy'), 'true');
  timers.advance(1);
  await alreadySettled(reloading);
  assert.equal(b.requests[1].signal.aborted, true);
  assert.ok(body.cancellations() > 0);
  assert.equal(timers.pending.size, 0);
  assert.deepEqual(b.painted, [committed]);
  assert.equal(b.attributes.get('aria-busy'), 'false');
  assert.equal(b.reloadAttributes.get('aria-busy'), 'false');
  assert.equal(b.reloadButton.disabled, true);
  const message = b.status.textContent;
  body.complete(frame({ sequence: 1, expanded: 0, visibleCount: 0,
    pixels: Buffer.alloc(rgb.length, 90).toString('base64') }));
  await settle();
  assert.equal(body.reads(), 1, 'late chunks cannot resume an abandoned body loop');
  assert.equal(body.releases(), 1);
  assert.deepEqual(b.painted, [committed]);
  assert.equal(b.attributes.get('aria-expanded'), 'true');
  assert.equal(b.status.textContent, message);
  b.reload();
  assert.equal(b.requests.length, 2, 'an uncertain reload is never replayed');
});

test('a scroll body deadline discards the coalesced target without sending another POST', async t => {
  const b = browser();
  const timers = deadlineClock();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient({ ...b, timers });
  t.after(client.dispose);
  await client.ready;
  const committed = b.painted[0];
  const body = stalledBody();
  b.replies.push(body.response);
  b.wheel();
  b.wheel();
  b.key('End');
  await settle();
  assert.equal(body.reads(), 1);
  timers.advance(65_000);
  await settle();
  assert.equal(timers.pending.size, 0);
  assert.equal(b.requests[1].signal.aborted, true);
  assert.equal(b.attributes.get('aria-busy'), 'false');
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.deepEqual(b.painted, [committed]);
  body.complete(scrollingFrame({ sequence: 1, scrollY: 20 }));
  await settle();
  b.wheel();
  b.key('Home');
  assert.equal(b.requests.length, 2, 'neither queued nor fresh input retries the uncertain action');
  assert.deepEqual(b.painted, [committed]);
});

test('a real response stream is released when its cancellation callback stalls', async t => {
  const b = browser();
  const timers = deadlineClock();
  let cancellations = 0;
  const stream = new ReadableStream({
    cancel() { cancellations++; return new Promise(() => {}); },
  });
  b.replies.push(new Response(stream, { headers: { 'content-type': 'application/json' } }));
  const client = startLiveClient({ ...b, timers });
  t.after(client.dispose);
  await settle();
  assert.equal(stream.locked, true);
  timers.advance(65_000);
  await alreadySettled(client.ready);
  assert.equal(cancellations, 1);
  assert.equal(stream.locked, false);
  assert.equal(timers.pending.size, 0);
  assert.equal(b.attributes.get('aria-busy'), 'false');
  assert.equal(b.painted.length, 0);
});

test('disposal settles stalled headers immediately and late authorization cannot erase a replacement token', async () => {
  const b = browser();
  const timers = deadlineClock();
  const pending = deferred();
  b.replies.push(pending.promise);
  const client = startLiveClient({ ...b, timers });
  client.dispose();
  assert.equal(timers.pending.size, 0);
  assert.equal(b.attributes.get('aria-busy'), 'false');
  await alreadySettled(client.ready);
  const replacementToken = 'b'.repeat(64);
  b.data.set(tokenKey, replacementToken);
  const late = stalledBody(401);
  pending.resolve(late.response);
  await settle();
  assert.equal(late.reads(), 0);
  assert.ok(late.cancellations() > 0);
  assert.equal(b.data.get(tokenKey), replacementToken);
  assert.equal(b.painted.length, 0);
});

test('disposal settles a stalled selection body without waiting for reader cancellation', async () => {
  const b = browser();
  const timers = deadlineClock();
  b.replies.push(response(selectionFrame()));
  const client = startLiveClient({ ...b, timers });
  await client.ready;
  const body = stalledBody();
  b.replies.push(body.response);
  const selecting = b.select(1);
  await settle();
  assert.equal(body.reads(), 1);
  client.dispose();
  assert.equal(timers.pending.size, 0);
  assert.equal(b.requests[1].signal.aborted, true);
  assert.equal(b.attributes.get('aria-busy'), 'false');
  assert.equal(b.choiceAttributes.get('aria-busy'), 'false');
  await alreadySettled(selecting);
  body.complete(selectionFrame({ sequence: 1, selectedNoteId: 1 }));
  await settle();
  assert.equal(body.reads(), 1);
  assert.equal(body.releases(), 1);
  assert.equal(b.painted.length, 1);
  assert.equal(b.noteChoice.value, '0');
  assert.equal(b.noteChoice.disabled, true);
  assert.equal(b.requests.length, 2);
});

test('completed replies and local failures release their deadlines', async t => {
  for (const outcome of ['success', 'retained-error', 'network', 'invalid-frame']) await t.test(outcome, async t => {
    const b = browser();
    const timers = deadlineClock();
    b.replies.push(response(frame()));
    const client = startLiveClient({ ...b, timers });
    t.after(client.dispose);
    await client.ready;
    assert.equal(timers.pending.size, 0);
    b.replies.push(outcome === 'network' ? Promise.reject(new Error('connection lost'))
      : outcome === 'invalid-frame' ? response(frame({ width: 400 }))
      : outcome === 'retained-error' ? response({ error: 'Reload failed', frame: frame({ sequence: 1 }) }, 500)
      : response(frame({ sequence: 1 })));
    await b.reload();
    assert.equal(timers.pending.size, 0);
    const message = b.status.textContent;
    timers.advance(65_000);
    await settle();
    assert.equal(b.status.textContent, message);
    assert.equal(b.attributes.get('aria-busy'), 'false');
    assert.equal(b.requests.length, 2);
  });
});

test('authorization failure settles even when response-body cancellation never completes', async t => {
  const b = browser();
  const timers = deadlineClock();
  const body = stalledBody(403);
  b.replies.push(body.response);
  const client = startLiveClient({ ...b, timers });
  t.after(client.dispose);
  await alreadySettled(client.ready);
  assert.equal(timers.pending.size, 0);
  assert.equal(body.reads(), 0);
  assert.ok(body.cancellations() > 0);
  assert.equal(b.data.size, 0);
  assert.equal(b.attributes.get('aria-busy'), 'false');
  assert.match(b.status.textContent, /Session access expired/);
});

test('explicit reconnect reconciles a lost action reply without replay and uses the new input generation', async t => {
  const b = browser();
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const committed = b.painted[0];
  b.replies.push(Promise.reject(new Error('reply lost after native commit')));
  b.click();
  await settle();
  assert.deepEqual(b.painted, [committed]);
  assert.equal(b.reconnectButton.hidden, false);
  assert.equal(b.reconnectButton.disabled, false);
  assert.equal(b.reloadButton.disabled, true);
  assert.equal(b.attributes.get('aria-disabled'), 'true');
  assert.match(b.status.textContent, /Reconnect to continue/);
  const sync = deferred();
  b.replies.push(sync.promise);
  const reconnecting = b.reconnect();
  assert.equal(b.reconnectButton.disabled, true);
  assert.equal(b.reconnectAttributes.get('aria-busy'), 'true');
  b.reconnect();
  b.click();
  b.reload();
  b.key('End');
  b.wheel();
  assert.deepEqual(b.requests.map(request => request.path), ['/sync', '/input', '/sync']);
  assert.equal(b.requests[2].body, '{}');
  assert.equal(b.requests[2].headers['x-a1-input-generation'], undefined);
  sync.resolve(response(frame({ inputGeneration: 2, sequence: 1, expanded: 0, visibleCount: 0 })));
  await reconnecting;
  assert.equal(b.painted.length, 2);
  assert.equal(b.attributes.get('aria-expanded'), 'false');
  assert.equal(b.attributes.get('aria-disabled'), 'false');
  assert.equal(b.reconnectButton.hidden, true);
  assert.equal(b.reconnectAttributes.get('aria-busy'), 'false');
  assert.equal(b.requests.length, 3, 'reconciliation reads the committed action without replay');
  b.replies.push(response(frame({ inputGeneration: 2, sequence: 2 })));
  b.click();
  await settle();
  assert.equal(b.requests[3].path, '/input');
  assert.equal(b.requests[3].headers['x-a1-input-generation'], '2');
  assert.equal(b.attributes.get('aria-expanded'), 'true');
});

test('a stale-generation rejection displays its retained frame but requires explicit synchronization', async t => {
  const b = browser();
  b.replies.push(response(frame()));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  b.replies.push(response({ error: 'Input generation expired', requiresSync: true,
    frame: frame({ inputGeneration: 2, sequence: 1, expanded: 0, visibleCount: 0 }) }, 409));
  await b.reload();
  assert.equal(b.painted.length, 2);
  assert.equal(b.attributes.get('aria-expanded'), 'false');
  assert.equal(b.reloadButton.disabled, true);
  assert.equal(b.reconnectButton.hidden, false);
  b.click();
  assert.equal(b.requests.length, 2);
  b.replies.push(response(frame({ inputGeneration: 3, sequence: 1, expanded: 0, visibleCount: 0 })));
  await b.reconnect();
  assert.equal(b.reloadButton.disabled, false);
  assert.equal(b.reconnectButton.hidden, true);
  assert.equal(b.requests.length, 3);
});

test('initial and retained-frame synchronization failures never enable input before a successful barrier', async t => {
  const b = browser();
  b.replies.push(Promise.reject(new Error('offline')));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  assert.equal(b.painted.length, 0);
  assert.equal(b.reconnectButton.hidden, false);
  b.replies.push(response({ error: 'Queue full', requiresSync: true,
    frame: frame({ inputGeneration: 2 }) }, 429));
  await b.reconnect();
  assert.equal(b.painted.length, 1);
  assert.equal(b.reloadButton.disabled, true);
  assert.equal(b.reconnectButton.hidden, false);
  b.click();
  assert.equal(b.requests.length, 2);
  b.replies.push(response(frame({ inputGeneration: 3 })));
  await b.reconnect();
  assert.equal(b.reloadButton.disabled, false);
  assert.equal(b.requests.length, 3);
});

test('synchronization refuses an unchanged generation and action frames cannot regress it', async t => {
  const b = browser();
  b.replies.push(response(frame({ inputGeneration: 5 })));
  const client = startLiveClient(b);
  t.after(client.dispose);
  await client.ready;
  const committed = b.painted[0];
  b.replies.push(response(frame({ inputGeneration: 4, sequence: 1 })));
  await b.reload();
  assert.deepEqual(b.painted, [committed]);
  assert.equal(b.reconnectButton.hidden, false);
  b.replies.push(response(frame({ inputGeneration: 5, sequence: 1 })));
  await b.reconnect();
  assert.deepEqual(b.painted, [committed]);
  assert.equal(b.reloadButton.disabled, true);
  b.replies.push(response(frame({ inputGeneration: 6, sequence: 1 })));
  await b.reconnect();
  assert.equal(b.reloadButton.disabled, false);
});

test('late timed-out authorization cannot overwrite a successfully reconnected view or its authority', async t => {
  const b = browser();
  const timers = deadlineClock();
  b.replies.push(response(scrollingFrame()));
  const client = startLiveClient({ ...b, timers });
  t.after(client.dispose);
  await client.ready;
  const old = deferred();
  b.replies.push(old.promise);
  b.wheel();
  b.key('End');
  timers.advance(65_000);
  await settle();
  assert.equal(b.reconnectButton.hidden, false);
  b.replies.push(response(scrollingFrame({ inputGeneration: 2, sequence: 1, scrollY: 20 })));
  await b.reconnect();
  const settled = b.painted.at(-1);
  const message = b.status.textContent;
  old.resolve(response({ error: 'Old session expired' }, 401));
  await settle();
  assert.equal(b.painted.at(-1), settled);
  assert.equal(b.data.get(tokenKey), token);
  assert.equal(b.status.textContent, message);
  assert.equal(b.reloadButton.disabled, false);
  assert.deepEqual(b.requests.map(request => request.path), ['/sync', '/scroll', '/sync']);
  assert.equal(timers.pending.size, 0);
});

test('authorization loss and closed sessions do not offer reconnect with unusable authority', async t => {
  for (const status of [401, 403, 503]) await t.test(String(status), async t => {
    const b = browser();
    b.replies.push(response(frame()));
    const client = startLiveClient(b);
    t.after(client.dispose);
    await client.ready;
    b.replies.push(response({ error: 'Unavailable', frame: frame() }, status));
    await b.reload();
    assert.equal(b.reconnectButton.hidden, true);
    assert.equal(b.reconnectButton.disabled, true);
    assert.equal(b.reloadButton.disabled, true);
    assert.match(b.status.textContent, status === 503 ? /Session closed/ : /Session access expired/);
    b.reconnect();
    b.click();
    assert.equal(b.requests.length, 2);
    if (status !== 503) assert.equal(b.data.has(tokenKey), false);
  });
});

test('reconnecting read-only and exhausted sessions restores the view without manufacturing input authority', async t => {
  for (const limited of [frame({ canActivate: false, canReload: false }),
    frame({ sequence: 256, remainingEvents: 0 })]) await t.test(
    limited.remainingEvents === 0 ? 'exhausted' : 'read-only', async t => {
      const b = browser();
      b.replies.push(Promise.reject(new Error('offline')));
      const client = startLiveClient(b);
      t.after(client.dispose);
      await client.ready;
      b.replies.push(response({ ...limited, inputGeneration: 2 }));
      await b.reconnect();
      assert.equal(b.painted.length, 1);
      assert.equal(b.reconnectButton.hidden, true);
      assert.equal(b.reloadButton.disabled, true);
      assert.equal(b.attributes.get('aria-disabled'), 'true');
      assert.match(b.status.textContent, limited.remainingEvents === 0 ? /input limit reached/ : /View only/);
      b.click();
      b.reload();
      b.wheel();
      assert.equal(b.requests.length, 2);
    });
});

test('disposing during reconnect settles it and removes recovery listeners', async () => {
  const b = browser();
  const timers = deadlineClock();
  b.replies.push(Promise.reject(new Error('offline')));
  const client = startLiveClient({ ...b, timers });
  await client.ready;
  const pending = deferred();
  b.replies.push(pending.promise);
  const reconnecting = b.reconnect();
  client.dispose();
  await alreadySettled(reconnecting);
  assert.equal(b.reconnectListeners.size, 0);
  assert.equal(b.reconnectButton.disabled, true);
  assert.equal(b.reconnectAttributes.get('aria-busy'), 'false');
  assert.equal(timers.pending.size, 0);
  pending.resolve(response(frame({ inputGeneration: 2 })));
  await settle();
  assert.equal(b.painted.length, 0);
  assert.equal(b.requests.length, 2);
});

test('fixed page assets use external scripts/styles and preserve a borderless native coordinate surface', () => {
  assert.match(livePage, /<script type="module" src="\/client\.mjs"><\/script>/);
  assert.match(livePage, /<link rel="stylesheet" href="\/style\.css">/);
  assert.match(livePage, /id="note-frame" width="320" height="160"/);
  assert.match(livePage, /tabindex="0"/);
  assert.match(livePage, /<button id="reload-note" type="button" disabled[^>]*>Reload note<\/button>/);
  assert.match(livePage, /<button id="reconnect-view" type="button" disabled hidden>Reconnect<\/button>/);
  assert.match(livePage, /role="group" aria-label="Scroll outline"/);
  for (const [id, label] of [['scroll-start', 'Top'], ['scroll-up', 'Page up'],
    ['scroll-down', 'Page down'], ['scroll-end', 'Bottom']]) {
    assert.match(livePage, new RegExp(`<button id="${id}" type="button" disabled aria-controls="note-frame">${label}</button>`));
  }
  assert.match(livePage, /After editing the file, reload to see your changes/);
  assert.match(livePage, /<label for="note-choice">Note<\/label>/);
  assert.match(livePage, /<select id="note-choice" disabled/);
  assert.ok(!livePage.includes('<style') && !livePage.includes('onclick='));
  assert.match(liveStyles, /canvas \{[^}]*padding: 0; border: 0;/);
  assert.match(liveStyles, /image-rendering: pixelated/);
  assert.match(liveStyles, /\.scroll-actions button \{[^}]*min-height: 44px;/);
});
