# Captured cell storage

Local arrows still specialize into typed regions. Their actually referenced
bindings now have stable addresses in a private guest-memory arena, rather than
depending on the creating helper's Wasm locals. Sibling arrows address the same
cell; already computed `ValueId` snapshots remain separate. This is a storage
prerequisite for escaping closures, not admission of escaping function values.

The source checker collects slot references from the checked scratch body,
including references captured by nested unused arrows. It excludes unrelated
visible bindings and erased type-only references. A `CaptureCells` IR marker
records the requirement at arrow creation. The verifier requires each slot to be
lexically visible and initialized. Aliases reuse the checked capture set, and
source budgets still count the original body once.

Each captured slot receives a hidden `i32` pointer. Entry parameters allocate
their cells in the function prologue; other slots allocate at `Initialize`, once
per dynamic lexical entry. Loads and stores use the pointer. Cells occupy 16
aligned bytes: Number uses `f64` at offset 0, Predicate uses `i32` at offset 0,
and General uses its primitive tag at offset 0 and `f64` payload at offset 8.
Tag/payload stores are separate; no numeric conversion or new primitive tag is
introduced. Uncaptured slots retain their previous local/global representation.

The arena starts at one 64 KiB Wasm page and can grow to sixteen pages (1 MiB,
65,536 cells). Allocation checks the inclusive limit before advancing its cursor,
and checks `memory.grow` failure. The memory and cursor are private. A module with
no captured cells adds neither an arena nor a wrapper. Growth follows the
[Wasm memory instruction contract](https://webassembly.github.io/spec/core/exec/instructions.html#exec-memory-grow).

For synchronous modules, an exported wrapper resets the cursor for each external
`run`. All source functions retain their numeric indices, including internal
recursive calls to entry zero; those calls bypass the reset. Helpers may return
while their cells remain intact. The next external invocation reuses storage,
including after fuel or allocator traps, and does not shrink allocated pages.
The exported Number/primitive signatures are unchanged.

An async instance already represents one invocation. Its frame globals now also
retain captured-cell pointers across suspension; initialization runs only at the
fresh state. The same cell load/store emitter serves both backends. Typed global
indices retain their existing layout, and the allocator retains its own locals.

This bulk lifetime is sufficient for the current Number/four-primitive exported
results: no cell or function reference escapes an invocation. A future exported
function-value ABI must revisit reclamation. Function records, lifted bodies,
indirect calls, recursive closures, later-initialized captures, per-iteration
escaping `let` environments, collection and general Promise interoperability
remain open. The native-JS factory below produces 563 for inputs 4 and 2; it is
still a deliberate compiler rejection:

```js
const make = x => () => ++x;
const f = make(a), g = make(b);
return f() * 100 + f() * 10 + g();
```

Maintained tests inspect helper cells after return, exercise recursion through
the internal entry, check growth/limit/trap/reuse in both engines, and distinguish
one captured helper parameter from 999 unrelated parameters. Local-arrow and
async oracle tests cover shared writes, nested captures, all three slot types,
snapshots, primitive payload bits and suspension. These are bounded observations;
they do not close the source-level escaping-closure gap.
