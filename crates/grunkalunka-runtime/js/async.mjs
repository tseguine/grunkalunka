// One import-free Wasm instance per invocation. The host provides real Promises
// and its FIFO microtask queue; the guest keeps the suspended primitive activation.
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
      if (tracked && !queued) throw new TypeError('Promise events require queued async protocol');
      function decode(tag, payload) {
        if (tag === 0 && Object.is(payload, 0)) return undefined;
        if (tag === 1 && Object.is(payload, 0)) return null;
        if (tag === 2 && (Object.is(payload, 0) || payload === 1)) return payload === 1;
        if (tag === 3 && typeof payload === 'number') return payload;
        throw new TypeError('invalid primitive value tag or payload');
      }
      function accept([status, tag, payload]) {
        const value = decode(tag, payload);
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
              if (kind === 1 && !rejections.has(id)) rejections.set(id, Promise.reject(decode(eventTag, eventPayload)));
              else if (kind === 2 && rejections.has(id) && eventTag === 0 && Object.is(eventPayload, 0)) {
                rejections.get(id).catch(() => {});
                rejections.delete(id);
              } else throw new TypeError('invalid Promise event transition');
            }
          }
          // Every enqueue happened before this root settlement. Register each
          // callback now so other host microtasks see the same FIFO ordering.
          for (let i = 0; i < issued; i++) queueMicrotask(() => {
            if (!usable) return;
            try { accept(guest.resume(0, 0)); }
            catch (error) { usable = false; reject(error); }
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
      catch (error) { usable = false; throw error; }
    });
  };
}
