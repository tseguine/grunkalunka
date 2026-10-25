// One import-free Wasm instance per invocation. The host provides real Promises
// and its FIFO microtask queue; the guest keeps the suspended numeric activation.
export function createAsyncFunction(bytes) {
  const module = bytes instanceof WebAssembly.Module ? bytes : new WebAssembly.Module(bytes);
  if (WebAssembly.Module.imports(module).length) throw new TypeError('async modules must be import-free');
  return function run(a, b) {
    return new Promise((resolve, reject) => {
      if (typeof a !== 'number' || typeof b !== 'number') throw new TypeError('async inputs must be Numbers');
      const guest = new WebAssembly.Instance(module).exports;
      let jobs = 32;
      function accept([status, value]) {
        if (status === 0) resolve(value);
        else if (status === 1 || status === 2) {
          if (jobs-- === 0) throw new RangeError('async activation exceeded its 32-job budget');
          queueMicrotask(() => {
            try {
              if (status === 2) reject(value);
              else accept(guest.resume(value));
            } catch (error) { reject(error); }
          });
        } else throw new TypeError('invalid async protocol status');
      }
      accept(guest.start(a, b));
    });
  };
}
