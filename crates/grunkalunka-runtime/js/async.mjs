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
      function decode(tag, payload) {
        if (tag === 0 && Object.is(payload, 0)) return undefined;
        if (tag === 1 && Object.is(payload, 0)) return null;
        if (tag === 2 && (Object.is(payload, 0) || payload === 1)) return payload === 1;
        if (tag === 3 && typeof payload === 'number') return payload;
        throw new TypeError('invalid primitive value tag or payload');
      }
      function accept([status, tag, payload]) {
        const value = decode(tag, payload);
        if (status === 0) resolve(value);
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
      accept(guest.start(a, b));
    });
  };
}
