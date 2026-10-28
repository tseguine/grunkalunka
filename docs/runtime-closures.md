# Runtime local function values

The Number, primitive-return and primitive async profiles support initialized local
function values selected at runtime. Constructors include same-signature conditional
branches, logical `&&`/`||`/`??` selections, arrows, known factory results and existing
local closure aliases.
Known primitive async arrows have a separate [directly awaited call slice](async.md#suspension-and-host-jobs);
they are not ordinary runtime closure values.
Parentheses and erased TypeScript wrappers preserve their runtime meaning.
Scalar side effects can precede a constructor in a comma expression. Calls through
the resulting identifier or alias use an exact logical signature with Number,
represented primitive General and finite scalar-result callback arguments. Concise synchronous bodies can return Number,
Undefined, Null or Boolean, retaining the selected kind and payload. Runtime
factories return function pairs with Number, General or finite scalar-result callback parameters
and primitive or further function results. Leaf arrows may have concise
or block bodies. Bare returns and normal completion of local primitive block
bodies produce Undefined. Early returns and loops retain their existing completion
rules; complete Number-returning bodies keep their Number specialization.

```js
export function run(a, b) {
  const make = x => () => ++x;
  const f = a ? make(a) : make(b);
  const alias = f;
  return f() * 10 + alias();
}
```

For inputs 4 and 2 this returns 56; for 0 and 2 it returns 34. Only the selected
constructor executes. Aliases retain the same code and environment. Captured
variables remain shared cells, including captures of another runtime function
value. Numeric call results can initialize or update existing scalar bindings;
later primitive writes still select General storage where required.

Logical selection requires matching complete function signatures, including callback
parameter kinds and the result kind after scalar interface joins described below.
A represented function is truthy: `f && g`
evaluates `f` once and selects `g`; `f || g` keeps the captured value of `f` and skips
`g`. Functions are also non-nullish, so `f ?? g` keeps the same left snapshot and
skips the right operand. Evaluating an operand can construct a function, update a
binding or suspend;
testing its truthiness does not invoke its body. Skipped effects, awaits and
rejections stay skipped. Both operands are still checked against the supported
source profile. This also works for higher-order functions and function-returning
factories. Mixed primitive/function joins, function identity
and exporting a function through the primitive host ABI remain unsupported.

Coalescing requires compatible established operand kinds. Primitive binary `??`
also lets a constructor-free selector such as `(x,y)=>x??y` pass preliminary checking
with default numeric formals, then specialize using its actual function arguments.
Full function signatures still must match; TypeScript assertions establish no kinds.

`let` function bindings retain their initializer's logical signature and support
plain `=` and logical `&&=`, `||=`, `??=` assignment from the same constructors.
Logical assignment snapshots the old pair once. `&&=` evaluates and stores the RHS;
`||=` and `??=` retain the old pair and skip the RHS and store. Both sides must have
the same full signature even when the RHS is skipped. A rejected RHS leaves the
assignment store undone; earlier RHS writes remain visible.
Conditional/chained assignments
return the RHS function snapshot, so another local can retain it. Discarded function values
and assignments may occur in comma prefixes. Calls snapshot both callee fields
before evaluating ordered arguments, including when an argument reassigns the
binding or suspends. For example, with `a=4, b=2` this returns 695:

```js
export function run(a,b) {
  let f = x => a+x;
  const old = f;
  const before = f((f = x => b-x, 3));
  return before*100 + f(3)*10 + old(1);
}
```

Capturing a mutable function binding shares its cell; copying its value into an
alias snapshots the pair. Rebinding an already initialized function can form
self/mutual recursive calls through those cells, bounded by runtime fuel and the
arena. An initializer cannot capture its own unpublished binding. Mixed scalar/
function storage, signature changes, arithmetic compound assignments and numeric
updates remain rejected. Assignment results cannot enter primitive/coercion positions.

A `Closure(signature)` IR value is an `i32` code index plus an `i32` environment
address. It is separate from the four-primitive General domain and is never
packed into a Number or primitive host tag. A function-valued cell stores the
two fields at offsets 0 and 4 of its existing 16-byte cell. Conditional joins,
initialization and loads preserve both fields as one typed value. Numeric-leaf
signature IDs equal their arity; richer IDs refer to module-local ordered
parameter lists and a result type. Callback signatures have fixed declared inputs,
which may be Numbers or finite scalar-result callbacks. Supplied call counts may
exceed that fixed count: the callee pair is saved first, all arguments finish in
caller scope, and surplus values are discarded before the call. Only declared
inputs occupy the argument frame. Missing inputs and more than 1,000 supplied
arguments stay unsupported, including on callbacks and runtime factories.

The compiler lifts a synchronous body into an internal function. Its first IR
operation, `ClosureEntry`, imports initialized captured-cell addresses into
root-owned slots.
`MakeClosure` checks target kind, capture count, initialization, type and mutability.
`ClosureCall` checks the complete logical signature, argument types and result
type against the function's declared result metadata.
Ordinary `Call` cannot invoke a lifted body; the exported entry cannot use the
closure ABI. Function values cannot be packed as primitives or awaited in the current protocol.

The private Wasm table uses `(environment: i32, arguments: i32) -> f64` for
Number results, `(environment: i32, arguments: i32) -> (i32, f64)` for primitive
results, and `(environment: i32, arguments: i32) -> (i32, i32)` for
[runtime factory results](runtime-function-results.md). A packed argument frame contains the
ordered typed snapshots. Number and two-i32 callback entries occupy eight bytes;
General tag/f64 entries occupy sixteen bytes. Caller stores and callee loads share
prefix offsets from that backend policy. Source-known runtime closures and
factories now use General input kinds when their actual scalar arguments require
them. Number arguments are boxed when the callee expects General, while callback
pairs retain their separate kind. The body copies its typed logical parameters
before execution. This supports zero
through 1,000 parameters without expanding the physical Wasm signature. An
environment contains ordered captured-cell addresses. Imported pointers remain valid after creator return. Static callbacks
inside lifted bodies have their captures remapped to those imports. Source
helpers retain their original function indices; lifted bodies append afterward.
In the async profile the dispatcher and protocol entries have different physical
indices, so table entries refer to named lifted bodies rather than numeric IDs.
Only lifted bodies populate table slots; ordinary source-function positions are null.

Cells, environments and argument frames share the private 1 MiB invocation arena.
Frames round up to 16-byte alignment. Allocation checks the size, alignment,
inclusive capacity and memory-growth failure before advancing the cursor. The
memory and table are unexported. The next external invocation resets the cursor,
including after a fuel or allocator trap; internal calls preserve it. Async
activations instead own a fresh instance until settlement or failure and retain
their arena across jobs. Frames consume the bounded arena until that lifetime ends.

Source locals/statements still count once. Constructors and indirect calls share
the 256 generated-expansion limit per function; at most 256 lifted bodies are
admitted per module, with capacity rechecked after nested construction. Eight
combined active closure/block lowering frames bound native recursion. A 2 MiB
stack probe accepts six nested block wrappers around the leaf and rejects seven;
source nesting, fuel and arena limits remain additional restrictions. Crossings
between function constructors and scalar comma-prefix evaluation have a separate
16-active-cursor guard; a 2 MiB stack probe accepts 15 nested assignment prefixes
and rejects 16. These crossings carry the caller's original AST depth.

Tests compare original JS with Node Wasm and Wasmtime, including multiple captures,
aliases, nested runtime/static callbacks, helper indices, argument effects, all
captured slot kinds, 1,000 parameters, erased types, allocator boundaries and
recovery. Malformed IR tests check environment and calling-convention invariants.
Existing 919 rejection sources retain their exact diagnostics and hashes; the
former focused runtime-factory-selection rejection now has a positive execution
case. Canonical corpus counts remain unchanged.

The async frame retains code/environment pairs, scalar snapshots and capture-cell
pointers across suspension. Lifted bodies keep ordinary activation locals, while
only the source entry has resume states. Awaited conditions and factory/call
arguments preserve their original evaluation order. `await f()` can suspend on a
Number result; `await f` remains rejected because functions are not primitives.
Existing static closure specialization across await remains available.
[Known local helpers and factories accept callback arguments](local-callback-arguments.md)
with finite scalar-result signatures, including captured callback cells after creator return.
[Runtime-selected functions also accept callbacks taking further callbacks](higher-order-callbacks.md).
Concise and complete block runtime factories can return further factories through
acyclic result signatures with Number, General or finite scalar-result callback parameters.
Cyclic callback types, unrepresented leaf
return kinds, function coercions, identity and properties, recursive initialization
and escaping per-iteration environments remain open.
The exported Number/primitive/async signatures and dependency set are unchanged.

Represented functions are now truthy in [Boolean contexts](function-truthiness.md).
The source expression executes once while the tested function's body stays dormant;
constructors, assignments, known returned values and admitted higher-order pairs use
the existing predicate path. This adds neither numeric coercion nor function transport.

General IR parameters are restricted to this private closure entry protocol;
ordinary helpers and exported entries retain their Number parameter contracts.
Executable IR tests exercise mixed scalar/function/General frames, primitive and
Number returns, a captured General parameter, and the unchanged 1,000 parameter
limit. Known scalar-result local source calls such as `(x=>+x)(null)` now specialize
primitive inputs into their caller's regions, including awaited known async calls. Their
parameter cells preserve primitive kinds, snapshots and captured writes. This
also applies to known concise and block-bodied factories with independently
established returned signatures. Known scalar actuals now feed formal storage and
result facts, so `(x=>x)(null)` returns Null in the primitive-value profile. Known
aliases, forwarding calls and captures preserve these facts; generic function
inputs keep their separate specialization. Runtime signatures, lifted parameter
slots and caller evaluation share those input facts, enabling primitive arguments
through indirect calls and returned factories. Input kinds are separate from
storage kinds: a Number input that is later assigned Null keeps its Number input
ABI and receives a General local cell. This preserves selection with another
Number-input function. Inlined known calls specialize their arguments and do not
widen the shared private ABI; runtime call sites establish that input requirement.
Source-known constructors sharing a runtime function value now join scalar input
kinds when their logical arities match. A runtime General input requirement on
one constructor propagates to the corresponding scalar formals of its peers.
Their cell/result dependencies and downstream call facts settle together; this
also keeps identity results and captured parameters consistent with the entry
kind. Callback positions retain their separate signatures. General local writes
or inlined actuals alone do not seed this join, and the other result/signature
guards remain unchanged.
The same constructor relation includes immediately invoked conditional/logical
callees, comma-wrapped selections and known forwarded/factory results, without
requiring an intermediate binding. For example, after a General call establishes
`f`'s input kind, `(a ? f : g)(b)` joins the peer input kinds just like calling a
binding initialized from `a ? f : g`. This analysis executes no callee or argument
effects; the evaluator retains the callee snapshot before evaluating arguments.
Actual constructors also propagate through demanded callback parameters, aliases,
forwarding and captured callbacks. Calls through those parameters establish the
constructor's primitive input requirements. Generic identity parameters retain
their separate call-site specialization. Source-known computed callees also
contribute result facts: `id(make(a)())` preserves Null when the returned closure
returns Null. Complete Number returns have a separate positive proof, shared by
storage, argument specialization and signature queries. Missing or fallthrough
returns do not prove Number; unresolved callees still contribute no result fact.
Return queries retain source-depth and bounded expansion guards. Existing Number
factory depth boundaries remain unchanged.
Validated awaited `Promise.resolve`/`Promise.reject` calls also contribute their
payload's scalar facts. This permits a callback's primitive input kind to be
established by its first awaited call. Unknown callees and ordinary Promise-object
arguments remain outside that exemption; await validation and reaction jobs are
unchanged.
The runtime arena and frame alignment are unchanged;
new General arguments simply
consume sixteen-byte entries within the existing allocation limit.
Maintained source tests cover mixed General/Number/callback entries, saved callees,
parameter writes/captures and async job order in both engines. A frame with 1,000
General parameters executes with an explicit bounded Wasmtime test fuel budget and
Node; this does not raise the default host fuel allowance. Missing inputs,
defaults, rest/spread and unrepresented String/object values retain their guards.

Selected scalar constructors also join Number and General result interfaces. If
any source-known peer lacks a Number proof, the group uses the existing tagged
General return pair, packing Number results without changing their bits. This
propagates through overlapping groups and applies to named, immediate and callback
selections. Input inference and source-local Number proofs remain separate: a
call proven to return Number can still use arithmetic even when its private
result interface is General. A genuinely mixed result requires a value, Boolean
or explicit coercion position. Callee snapshots, argument effects, captures and
async reaction order retain their existing behavior. Function-valued results
retain their separate signatures; this does not join functions with primitives
or add String/object result support.

Constructor groups also follow complete factory returns without requiring a source
call. An unused nested factory can therefore change between Number and Null leaf
results using the same common General interface as its called form. Returned
aliases, block returns and intervening shared constructors retain their original
symbols. Traversal uses the existing eight-frame signature-query bound, evaluates
no body or argument, and leaves recursive/unrepresented result guards in place.
Lone factories also seed the query, including unused conditional/logical and
complete block returns. Dormant constructor effects stay dormant, and generic
identity calls retain their separate specialization. Returned functions continue
to use the existing scalar-kind, capture and async caller protocols.

Synchronous runtime closures can also return represented fulfilled primitive
Promises through concise or complete block bodies. The logical signature uses
`FulfilledPromise`, distinct from General; emission reuses the existing tag/f64
return pair. Reassignment, returned-function chains and supported callback
forwarding preserve result snapshots, captured cells and callee-before-argument
evaluation. Calling adds no reaction job; an explicit await adds its ordinary job.
Mixed Promise/primitive parameter uses, mixed primitive/Promise result interfaces, pending/rejected
Promise objects and async-return adoption remain outside this slice. See
[fulfilled Promise values](fulfilled-promise-locals.md) for evidence and guards.

The private IR can now describe fulfilled Promise parameters and uses sixteen-byte
tag/f64 argument entries. Indirect calls and callback signatures check that logical
kind rather than treating the pair as a primitive General value. Executable IR
fixtures cover mixed argument widths, Promise-consuming callbacks, captured
parameter cells and the 1,000/1,001 limit. Source argument inference and binding now
admit these fulfilled inputs in known/runtime closures and supported callbacks and
factories. Original-symbol positive facts propagate through aliases and actual
calls, then disposable definition checking and actual evaluation retain the same
logical kind. Mixed Promise/primitive actuals and kind-changing Promise-formal
stores remain rejected. Source oracles separately compare snapshots, captures,
effects, all primitive payload tags and caller/body reaction jobs.

Runtime alternatives share a demanded callback role at the same formal position,
including a peer that never invokes that callback. For example:

```js
export function run(a, b) {
  const f = a ? (p, x, fn, y) => fn(p) : (p, x, fn, y) => p;
  return f(b, a, p => p, false);
}
```

The corresponding fulfilled-Promise-input case also lowers. Existing body and
forwarding demands seed the role; complete argument signatures still refine and
check it. Supplying a constructor to an ordinary identity call alone does not
make that identity function monomorphic. Aliases, reassignment, immediate calls,
returned alternatives and supported higher-order forwarding use the same rule.
Unused callback arguments and surplus arguments still evaluate in order, and the
callee snapshot precedes those evaluations. The maintained
`selected_unused_callback_peers_preserve_numeric_inputs` and
`selected_unused_callback_peers_preserve_promises` tests compare JS/TS native
JavaScript with Node-Wasm and Wasmtime, including snapshots, captures, primitive
and Promise inputs, and reaction jobs. Callback coercions, kind-changing stores,
incompatible complete signatures and existing missing/spread-input restrictions
remain rejected. This is bounded callback admission, not general function support.

Async arrows with admitted primitive bodies and no own-scope await/throw can be
represented by these same runtime closure pairs. Their private result signature
is `FulfilledPromise`; the lifted body computes a General primitive completion
and then wraps it. This keeps call-prefix effects synchronous and defers only the
caller's await reaction. Concise bodies and blocks with explicit or fallthrough
Undefined are checked by the existing primitive lowerer. Selection, reassignment,
returned factories, callback forwarding, stored results and fulfilled-Promise
parameters preserve the same callee/argument/capture snapshots. Unused async
constructors do not run their bodies. Mixed async/synchronous primitive result
interfaces, Promise-return adoption, async function/object/string outcomes and
general pending Promise objects remain unsupported. Known directly awaited async
calls still use their existing suspension and rejection boundary.

Non-suspending async arrows that throw primitive reasons now return the distinct
`SettledPromise` kind. Constructor groups share that interface across throwing
and fulfilling peers, including synchronous fulfilled-Promise-returning arrows.
Closed body capture preserves primitive catches, rethrows, loops, early returns,
fallthrough Undefined and synchronous prefix effects. Private calls can forward
the snapshot through supported synchronous callbacks and factories; its immediate
await adds one caller reaction and delivers rejection to the nearest caller catch.
The physical closure pair and private tag/f64 result layout are unchanged.

```js
export async function run(a, b) {
  const f = a ? async () => { throw b; } : async () => null;
  try { return await f(); } catch (reason) { return reason; }
}
```

Completed async source results now materialize once into identity records. They
can be stored, discarded, tested for truthiness, passed through homogeneous
Promise parameters and adopted on async return. Source object coercion and
general Promise payloads remain open; the logical snapshots remain internal.
Maintained source tests compare JS/TS native execution with Node-Wasm and Wasmtime
outcomes, primitive bits, callee/actual/capture snapshots and complete reaction
traces. They retain 1,000/1,001 parameter and 32/33 await boundaries and compare
erased typed syntax against the corresponding JS artifact.

Async arrows with own-scope suspension now use async-effect signatures and saved
guest coroutine frames. Runtime selection/reassignment and immediate async peers,
callbacks, synchronous factories returning async functions and nested child calls
preserve the complete effect. A directly awaited call snapshots its callee before
all actual arguments, executes the body prefix eagerly and preserves separate
child/caller reactions, primitive rejection and lexical catches. Original comma
prefixes execute in order. General primitive and fulfilled-Promise inputs retain
their logical kinds; no annotation supplies a runtime proof.

Pending-capable calls now produce stored identity values when requested. Numeric
coercion, source identity comparison, adoption, mixed sync/async effects, recursive
source initialization, general pending payloads and finally remain outside this slice. The backend's recursive
IR-frame tests are not a source recursion claim. Source tests compare complete
34-marker native/Node-Wasm traces and Wasmtime values/reaction counts, including
1,000/1,001 parameters and 32/33 logical await sites. Wide transport tests use an
explicit larger test fuel budget; the production 10,000-fuel limit is unchanged.

Direct await also accepts conditional operands that select one of these calls,
an admitted primitive or a fulfilled/settled snapshot. Nested selections and
comma prefixes preserve condition, capture, callee and actual-argument snapshots,
including prefix suspension/rejection. Each selected leaf retains the original
caller's single reaction; its child awaits remain separate. For example,
`await (flag ? selected() : null)` can use a suspending async `selected` closure.
All branch bodies remain checked. Homogeneous pending logical results and storage
now use the source identity path below; mixed snapshot/identity storage still
requires a conversion design.

### Pending source values

Suspending async-effect calls can now return a guest Promise identity to an async
source body. `let`/`const` bindings, aliases, plain and logical reassignment,
captured getters, homogeneous conditional/logical results and later awaits keep
that identity separate from its primitive completion. Calls start eagerly and
can overlap, including calls to the same compiled body with separate saved values.
Object truthiness and discarded calls retain their eager effects and queued work.

```js
export async function run(a, b) {
  const f = async x => { await a; return x; };
  const p = f(a);
  const q = f(b);
  await p;
  return await q;
}
```

`Promise.resolve(p, extra1, extra2)` preserves an existing represented Promise identity.
The argument runs before all extras; extras may mutate captures or suspend before
the caller registers its await. Captured callee pairs and argument snapshots
survive this work. This is the existing-Promise identity case, not general adoption.
Strict/loose equality and inequality now compare represented Promise identities,
including aliases, distinct calls with equal completions, captured/private inputs
and operands spanning suspension. Supported General values remain distinct from
a Promise, preserving both operands' effects. Comparison neither awaits the
Promise nor handles its rejection. Mixed identity/snapshot storage, mixed
pending/scalar parameter domains, primitive coercion and snapshot async-return
adoption remain explicit source gaps. Immediate non-suspending async closures
retain their fulfilled/settled snapshot contracts.

Homogeneous pending inputs now follow positive original-source provenance into
private signatures. Known calls, runtime selection, higher-order callbacks and
factories transport the same handle; captured formals and synchronous forwarding
preserve it. Caller argument evaluation snapshots the handle before later actuals
or surplus arguments can replace the caller's binding. Parameter reassignment may
store another represented pending identity. Awaiting through the formal preserves
primitive rejection handling and the caller's separate reaction. TypeScript
annotations are erased, not used to infer these kinds. Mixed pending/snapshot/scalar
input interfaces remain separate gaps. Async roots/private functions now adopt
returned pending identities, including concise helpers without an explicit await.
Registration and forwarding use separate jobs and the original saved frame.
Rejected adoption bypasses local catches while returned-expression evaluation
errors retain lexical handling. Outer awaits may catch the adopting function's
rejection. Primitive results, captured handles, sibling frames, later registration
and notification ownership are checked against native Promise traces. Snapshot
returns and arbitrary thenable/identity-primitive joins still need further work.

Awaited primitive rejection reaches its lexical catch, including repeated and
late awaits. Detached child rejections now produce host notifications. At `(4, 2)`
the source below fulfills the root with `4` and reports child reason `2` through
Node's `unhandledRejection` event in both native JS and the Wasm adapter.

```js
export async function run(a, b) {
  const f = async () => { await a; throw b; };
  f();
  return a;
}
```

The guest emits ordered rejection and subsequent-handling notifications with
invocation-local Promise IDs. The Node adapter creates native rejected Promise
proxies at each guest boundary and attaches a handler when the guest observes
the rejection. Native host checkpoint policy then reports only rejections still
unhandled; awaiting a rejected child later in the same checkpoint suppresses the
event. Proxies carry primitive reasons and do not expose guest execution state.
Wasmtime's `take_promise_notifications()` consumes the same ordered events;
embeddings apply their own checkpoint policy. Events are separate from runnable
jobs and remain observable after root settlement. This is not general Promise
adoption, escaping Promise objects or HTML event-loop conformance.

Both adapters continue ready work after root settlement. Source tests compare
full native/Node-Wasm marker traces;
Wasmtime uses an explicit shared FIFO and marker chain, drains remaining work,
and compares completion bits and marker positions. Counting continuations alone
would incorrectly equate sibling jobs with external ticks. Existing site, job,
arena, state and production fuel bounds remain in force.

### Self-resolution errors through captured Promise cells

A captured mutable pending binding can be replaced by the Promise of the async
function reading it. Returning that same Promise now rejects with an intrinsic
TypeError, so an outer await/catch completes rather than hanging with no ready job.
Error identities survive catch cells, aliases, repeated awaits, rethrow, private
callback transport and notification delivery after root settlement. Node receives
native TypeError objects cached per invocation; Wasmtime reports invocation-local
typed IDs. Neither a numeric ID surrogate nor a host trap substitutes for the
guest reason. The primitive protocol remains unchanged. Explicit numeric coercion,
truthiness, identity equality and all four relational operators are covered for
the represented intrinsic Errors. Relational comparison preserves General operands
through calls and suspension, distinguishes equal primitive strings from distinct
object identities, and keeps explicit numeric coercion separate. Arbitrary Error
properties and general strings remain gaps documented in [async](async.md).

### Error reasons in settled immediate closures

The opt-in intrinsic Error kind is preserved through runtime-selected immediate
async results, whether fulfilled or rejected. Captured and forwarded reasons keep
their identity; repeated rethrow and subsequent numeric coercion retain the same
General semantics. Rejection no longer shares bit 4 with the Error kind: the
backend reserves its high status bit while retaining the established tag/f64 pair
and sixteen-byte private storage. Existing primitive values and reaction order
remain unchanged. This does not materialize immediate snapshots as Promise objects.

### Materializing completed results in private helpers

`MaterializePromise` lets synchronous private IR helpers return fresh Promise
identities from fulfilled/settled completion snapshots. Construction is eager and
non-suspending; repeated calls allocate distinct records and preserve aliases,
payload kinds and rejection status. Await and async-return adoption retain their
existing reaction semantics. Constructor-only modules link the same guest kernel.
The async frontend now uses this operation for completed source constructors and
immediate async results. Alias reads preserve the handle; synchronous callbacks,
factories and captured bindings retain it, and async returns adopt it. Completed
and pending results share the private Promise interface. Immediate bodies retain
compact synchronous completion code; only suspending/adopting constructors need
coroutine frames. Existing host protocols, record widths and budgets are unchanged.
General Strings/payloads, thenables and source storage/call unions remain open.

### Conditional and comma async results

Async closures may select primitive/intrinsic Error and represented Promise
return leaves through conditionals, comma and logical expressions. Any Promise
return leaf gives the constructor its adoption effect; selected primitive branches
still complete without adoption jobs. Captures, runtime-selected constructors,
supported async callbacks and simultaneous activations preserve source snapshots
and microtask order. Logical returns test the original General or Promise left
snapshot and resolve only the selected operand; discarded Promise rejections
remain observable. Nested heterogeneous left joins use the carrier below.
Mixed private parameter domains remain open. Synchronous helpers may start
suspending async callbacks through the callable entries described below.

### Nested async selection carriers

Async constructors and supported callbacks now retain General/Promise choices
through nested logical left operands. Captures, runtime-selected constructors
and repeated activations keep the selected kind before resolving the return.
The internal `AsyncValue` carrier uses shared suspended storage but is excluded
from private call signatures and exported completions. General/Promise source
parameter and local-storage unions remain separate work.

### Callable async prefix entries

Synchronous closure helpers can start represented async callbacks, return their
Promise handles, forward them through further helpers or discard them. Starting
runs only the synchronous prefix and does not imply awaiting or handling the child.
Private argument frames preserve General pairs, Promise handles and callback pairs.

The backend supplies callable entries only when ordinary helpers contain starts.
The Wasm call stack retains each entry's caller frame and continuation while its
child prefix runs. The coroutine caller preserves its persistent lanes around
ordinary direct and indirect calls; a nested prefix can re-enter the same body
without overwriting the outer activation. Child reaction jobs retain their own
frames. Existing actual-suspension guards and all budgets remain unchanged.
This does not admit arbitrary recursive initialization, thenables or mixed private
input/storage domains. Native JS, Node-Wasm and Wasmtime source tests compare
results and marker order, with separate detached-rejection notification checks.
