// One import-free Wasm instance per invocation. The host provides real Promises
// and its FIFO microtask queue; the guest owns activations and Promise records.
export function createAsyncFunction(bytes) {
  const module = bytes instanceof WebAssembly.Module ? bytes : new WebAssembly.Module(bytes);
  if (WebAssembly.Module.imports(module).length) throw new TypeError('async modules must be import-free');
  return function run(a, b) {
    return new Promise((resolve, reject) => {
      if (typeof a !== 'number' || typeof b !== 'number') throw new TypeError('async inputs must be Numbers');
      const guest = new WebAssembly.Instance(module).exports;
      let jobs = 32;
      let usable = true;
      const queued = typeof guest.take_jobs === 'function';
      const tracked = typeof guest.take_promise_event === 'function';
      const rejections = new Map();
      const rejectedIds = new Set();
      const errors = new Map();
      const promises = new Map();
      const errorObjects = typeof guest.error_kind === 'function';
      const promiseObjects = typeof guest.promise_state === 'function';
      if (tracked && !queued) throw new TypeError('Promise events require queued async protocol');
      if (errorObjects && !queued) throw new TypeError('error objects require queued async protocol');
      if (promiseObjects && !queued) throw new TypeError('Promise objects require queued async protocol');
      function promiseId(id) {
        if (!Number.isInteger(id) || id < 16 || id > 1048544 || id % 16 !== 0) throw new TypeError('invalid Promise object identity');
      }
      function snapshot(id) {
        promiseId(id);
        const row = guest.promise_state(id);
        if (!Array.isArray(row) || row.length !== 4) throw new TypeError('invalid Promise snapshot');
        const [state, handled, tag, payload] = row;
        if (![0, 1, 2].includes(state) || (handled !== 0 && handled !== 1)) throw new TypeError('invalid Promise snapshot state');
        if (state === 0) {
          if (tag !== 0 || !Object.is(payload, 0)) throw new TypeError('invalid pending Promise snapshot');
        } else if (tag === 5 && state === 2) promiseId(payload);
        else decode(tag, payload);
        const entry = promises.get(id), previous = entry?.observed;
        if (previous?.[0] && (state !== previous[0] || tag !== previous[2] || !Object.is(payload, previous[3]))) throw new TypeError('Promise settlement changed');
        if (previous?.[1] && !handled) throw new TypeError('Promise handling changed');
        if (entry) entry.observed = row;
        return row;
      }
      function promiseObject(id) {
        promiseId(id);
        if (!promises.has(id)) {
          if (promises.size >= 32768) throw new RangeError('too many Promise objects');
          const observed = snapshot(id);
          const entry = {state: 0, observed, suppressed: false, published: false};
          entry.promise = new Promise((resolve, reject) => { entry.resolve = resolve; entry.reject = reject; });
          promises.set(id, entry);
        }
        return promises.get(id).promise;
      }
      function refreshPromises() {
        // Map iteration also visits newly decoded reasons, without recursion.
        for (const [id, entry] of promises) {
          const [state, handled, tag, payload] = snapshot(id);
          if (!entry.state && state) {
            const value = decode(tag, payload, state === 2);
            entry.state = state;
            if (state === 1) entry.resolve(value); else entry.reject(value);
          }
          if (state === 2 && handled && !entry.suppressed) {
            entry.promise.catch(() => {}); entry.suppressed = true;
          }
        }
      }
      function decode(tag, payload, allowPromise = false) {
        if (tag === 0 && Object.is(payload, 0)) return undefined;
        if (tag === 1 && Object.is(payload, 0)) return null;
        if (tag === 2 && (Object.is(payload, 0) || payload === 1)) return payload === 1;
        if (tag === 3 && typeof payload === 'number') return payload;
        if (tag === 5 && promiseObjects && allowPromise) return promiseObject(payload);
        if (tag === 4 && errorObjects) {
          if (!Number.isInteger(payload) || payload < 16 || payload > 1048544 || payload % 16 !== 0) throw new TypeError('invalid error object identity');
          if (guest.error_kind(payload) !== 1) throw new TypeError('invalid error object kind');
          if (!errors.has(payload)) errors.set(payload, new TypeError('Promise cannot resolve to itself'));
          return errors.get(payload);
        }
        throw new TypeError('invalid primitive value tag or payload');
      }
      function fail(error) {
        usable = false;
        for (const entry of promises.values()) {
          if (entry.state === 0) {
            entry.state = -1;
            entry.reject(error);
          }
          // Neither a private failed decode nor terminal host failure is a
          // published guest unhandled-rejection event.
          if ((!entry.published || entry.state === -1) && !entry.suppressed) {
            entry.promise.catch(() => {}); entry.suppressed = true;
          }
        }
        reject(error);
      }
      function accept([status, tag, payload]) {
        const value = decode(tag, payload, status === 3);
        if (queued) {
          if (status !== 0 && status !== 1 && status !== 3) throw new TypeError('invalid queued async protocol status');
          const issued = guest.take_jobs();
          if (!Number.isInteger(issued) || issued < 0 || issued > jobs) throw new RangeError('async activation exceeded its 32-job budget');
          jobs -= issued;
          if (tracked) {
            // Publish at each boundary, before root settlement. Native rejected
            // proxies let the host apply its own end-of-checkpoint reporting.
            for (let count = 0; ; count++) {
              if (count === 32768) throw new RangeError('too many Promise events');
              const [kind, id, eventTag, eventPayload] = guest.take_promise_event();
              if (kind === 0) {
                if (id !== 0 || eventTag !== 0 || !Object.is(eventPayload, 0)) throw new TypeError('invalid empty Promise event');
                break;
              }
              if (!Number.isInteger(id) || id < 16 || id % 16 !== 0) throw new TypeError('invalid Promise event identity');
              if (kind === 1 && !rejections.has(id) && !(promiseObjects && rejectedIds.has(id))) {
                if (promiseObjects) {
                  const object = promiseObject(id);
                  const [state, , tag, payload] = snapshot(id);
                  if (state !== 2 || tag !== eventTag || !Object.is(payload, eventPayload)) throw new TypeError('Promise rejection event disagrees with its record');
                  rejections.set(id, object);
                  rejectedIds.add(id);
                } else rejections.set(id, Promise.reject(decode(eventTag, eventPayload)));
              }
              else if (kind === 2 && rejections.has(id) && eventTag === 0 && Object.is(eventPayload, 0)) {
                if (promiseObjects) {
                  const [state, handled] = snapshot(id);
                  if (state !== 2 || handled !== 1) throw new TypeError('Promise handling event disagrees with its record');
                  const entry = promises.get(id);
                  if (!entry.suppressed) { entry.promise.catch(() => {}); entry.suppressed = true; }
                } else rejections.get(id).catch(() => {});
                rejections.delete(id);
              } else throw new TypeError('invalid Promise event transition');
            }
          }
          refreshPromises();
          for (const entry of promises.values()) entry.published = true;
          // Every enqueue happened before this root settlement. Register each
          // callback now so other host microtasks see the same FIFO ordering.
          for (let i = 0; i < issued; i++) queueMicrotask(() => {
            if (!usable) return;
            try { accept(guest.resume(0, 0)); }
            catch (error) { fail(error); }
          });
          if (status === 0) resolve(value);
          else if (status === 3) reject(value);
          return;
        }
        if (status === 0) resolve(value);
        else if (status === 3) reject(value);
        else if (status === 1 || status === 2) {
          if (jobs-- === 0) throw new RangeError('async activation exceeded its 32-job budget');
          queueMicrotask(() => {
            try {
              if (status === 2) reject(value);
              else accept(guest.resume(tag, payload));
            } catch (error) { reject(error); }
          });
        } else throw new TypeError('invalid async protocol status');
      }
      try { accept(guest.start(a, b)); }
      catch (error) { fail(error); }
    });
  };
}
