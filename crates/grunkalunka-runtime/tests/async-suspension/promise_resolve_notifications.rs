// Executed by the shared native/Node-Wasm/Wasmtime checkpoint harness.
pub const SOURCES: &[&str] = &[
    "export async function run(a,b){const p=Promise.reject(b),x=a?p:b;const q=Promise.resolve(x);return q===p;}",
    "export async function run(a,b){const p=Promise.reject(b),x=a?p:b;const q=Promise.resolve(x,await a);return q===p;}",
    "export async function run(a,b){const p=Promise.reject(b),x=a?p:b;const q=Promise.resolve(x);try{await q;}catch(e){}return a;}",
    "export async function run(a,b){const p=Promise.reject(b),x=a?p:b;try{const q=Promise.resolve(x,await Promise.reject(a));return q===p;}catch(e){return a;}}",
    "export async function run(a,b){const p=Promise.reject(b),x=a?p:b;const q=Promise.resolve(x),r=Promise.resolve(x);return q===r&&q===p;}",
    "export async function run(a,b){const f=async()=>{await a;throw b;};const p=f(),x=a?p:b;const q=Promise.resolve(x);return q===p;}",
];
