# JavaScript feature train

## Direction (2026-11-03)

The maintainer has assigned the compiler/runtime stream to order language
features by practical impact and implement them in that order, with additional
weight for A1 application authoring. A1 is a priority, not the exclusive consumer.
Ship useful feature families in reviewable slices; complete ECMAScript coverage
of each family is not a prerequisite for moving to the next useful slice.

The starting revision is
[`0f34cd7`](https://github.com/tseguine/grunkalunka/commit/0f34cd774c28074048fce90f10e4b2dfc3114d53),
after the source-lowering architecture series through
[#468](https://github.com/tseguine/grunkalunka/pull/468). New features use the
existing owned source cursor, typed Region IR, managed runtime and completion
machinery. Extract shared facts or helpers when a feature needs them; another
architecture-wide migration is not a gate for this train.

## Ordered feature families

| Order | Feature family | Practical payoff and first useful slices |
| --- | --- | --- |
| 1 | Capability calls composed with ordinary functions | Let A1 handlers organize policy in named helpers and captured callbacks while invoking the existing explicit Rust/WIT service. First admit numeric named helpers in `compile_ui_import`, then closures and captured state, preserving the current service ABI. |
| 2 | Runtime collection access and updates | Select headings by runtime position, inspect lists and maintain keyed application state. Start with array `.at(Number)` using the existing sparse-array storage; then expand computed keys and updates as their runtime key semantics become available. |
| 3 | Practical Strings and common operators | Measure and construct headings, status labels and search text. Start with UTF-16 `.length` and String concatenation; then character access, useful conversions and methods in response to source programs. |
| 4 | Everyday bindings and iteration | Make record/list processing natural: object and array binding patterns, defaults and array `for...of`, followed by commonly needed control forms. Reuse the source cursor and existing per-iteration binding machinery. |
| 5 | Synchronous source errors | Let ordinary handlers recover from invalid operations. Extend the existing normal/throw seam into synchronous execution and add catchable failures as runtime operations need them. |
| 6 | Broader calls, receivers and object construction | Support ordinary methods, dynamic `this`, less restrictive callable signatures and reusable model objects. Constructors/classes follow concrete consumers. |
| 7 | Managed asynchronous payloads | Carry records and Strings as Promise settlements, rejection reasons and await results. Build on existing traced suspension and related-activation identity. |
| 8 | General modules and remaining compatibility | Grow explicit application module graphs and live bindings, then less frequently needed builtins and language features. Expand conformance coverage with the implemented surface. |

This is a working order rather than a requirement to exhaust a family before
starting the next. An observed A1 source blocker or a feature that unlocks several
other families can move a bounded slice forward. Each adjustment should name the
actual source use case and dependency. Prototype/accessor completeness, every
coercion edge case and full Test262 coverage do not gate unrelated useful slices.

The first collection slice deliberately offers a runtime position through
`array.at(index)`. General `array[index]` also has JavaScript property-key
conversion and named-property behavior, including negative and fractional
Numbers; those require their own implementation. This distinction keeps the
first slice useful without silently changing the meaning of ordinary syntax.

## First implementation batch

1. **A1 named-helper composition.** Account for the imported Wasm function when
   emitting direct source calls. Execute source helpers around and inside the
   actual service call against the Rust/WIT provider and Node-Wasm.
2. **A1 captured-handler composition.** Extend the same mapping to closure table
   targets and the entry wrapper while retaining logical function identities,
   captured-cell behavior and existing import-free artifacts.
3. **Runtime array positions.** Admit `.at` on known managed arrays, including
   negative positions, sparse misses and source-ordered argument effects.
4. **String length.** Admit `.length` on known managed Strings, counting UTF-16
   code units through helpers, captures and existing supported source contexts.
5. **String concatenation.** Admit binary `+` between known Strings, preserving
   UTF-16 contents and the left snapshot across right-side effects or awaits.

## First batch completion

The dependency train implements these five slices. The existing suites retain
source oracles and deliberate admission boundaries:

| Implemented slice | Executable coverage |
| --- | --- |
| A1 named helpers and captured handlers | [`a1_ui_import.rs`](../crates/grunkalunka-runtime/tests/a1_ui_import.rs) compares native JS, Node-Wasm and the actual Rust/WIT provider, including call arguments, state and capability denial. |
| Runtime array positions | [`managed-arrays.rs`](../crates/grunkalunka-runtime/tests/managed-arrays.rs) covers Number positions, holes, negative indexing, operand order and saved selections. |
| String length and concatenation | [`managed-strings.rs`](../crates/grunkalunka-runtime/tests/managed-strings.rs) covers UTF-16, snapshots, helpers, captures, erased TS types, depth limits and the private concatenation-length guard. |
| Suspension composition | [`async-suspension/managed_values.rs`](../crates/grunkalunka-runtime/tests/async-suspension/managed_values.rs) compares selected values and saved String operands across awaits. |

The combined A1 fixtures select headings at runtime, construct and measure known
String labels, then invoke the service through captured handlers. They preserve
selected values and label snapshots after mutation. The boxed `.at` result is
observed through equality; it is not assumed to be a known String. These fixtures
extend compiler authoring evidence while the application acceptance below remains
separate. Local verification receipts, hosted checks and merge state are recorded
by the feature PRs; suite presence alone does not establish a hosted result.

## Second implementation batch

The second batch starts from [`a14de50`](https://github.com/tseguine/grunkalunka/commit/a14de5097004691e0f469c232efb048752eb9e80),
after the first native stack was merged. It continues the collection and String
families in this dependency order:

| Slice | Practical behavior | Executable coverage |
| --- | --- | --- |
| Null-prototype dictionaries | Construct `Object.create(null)` and use static own keys, including `constructor`, `toString` and `__proto__`. Track null-prototype origins through helpers, aliases and captures. | `managed-source.rs` and `async-suspension/managed_values.rs` |
| Runtime dictionary reads | Select an own value using a proven String key; preserve the receiver before evaluating the key and return undefined for a missing entry. | The same source/suspension suites plus the IR verifier tests |
| Runtime dictionary writes | Assign a boxed value through a proven String key while preserving receiver, key and assigned-value snapshots. Conservatively widen property facts through aliases for dynamic writes. | The same source/suspension suites plus the IR verifier tests |
| Array `pop` | Remove and return the last value, preserving holes, identity, argument effects and the current length after argument evaluation. | `managed-arrays.rs`, suspension and IR verifier tests |
| String ranges | Copy UTF-16 ranges with `slice` and `substring`, including their different negative-index and endpoint-order rules. | `managed-strings.rs`, suspension and IR verifier tests |
| String code units | Read a code unit with `charAt` or its numeric value with `charCodeAt`, including empty/NaN out-of-range results. | The same String/suspension suites |
| String search | Use `startsWith` and `includes` with a proven String needle and Number position. | The same String/suspension suites |
| A1 composition | Use dictionaries for keyed state, pop a heading history, derive/search labels in helpers, and invoke the existing service through a captured handler. | `a1_ui_import.rs` compares native JS, Node-Wasm and the actual Rust/WIT provider |

The A1 fixtures also retain receiver/key snapshots when a dictionary assignment
changes its receiver during key evaluation and changes the key while evaluating
the assigned value. Lookup and pop results stay boxed and are observed through
equality; those operations do not promise a String or callable result merely
because one tested entry has that type.

A1 action metadata can distinguish a missing override from an explicit Undefined
value using `Object.hasOwn` on a proven record, dictionary or array. The same
presence check distinguishes sparse holes from dense Undefined entries in a
rest list and recognizes the array's own non-enumerable length. String
receivers expose their length and UTF-16 code-unit indices. Numeric literal
keys use the same property name as member access. The `in` operator also
tests presence in proven null-prototype dictionaries, evaluating its key before
the receiver. Static keys on ordinary objects and arrays also account for
their inherited intrinsic names. Runtime String keys work on these receivers
when their prototype is known. A1 can test the selected action on either a
dictionary or an ordinary availability record. A handler can also construct
that record directly with a runtime action name, saving the computed String
key before evaluating the property's value. `Object.values` snapshots the
ordered action values into a fresh array for iteration, including explicit
Undefined fields. `Object.entries` lets the handler destructure each fresh
key/value pair and compare its name before invoking the service. A handler
that needs only names can iterate `Object.keys(actions)` and use String
operations on each yielded name. `Array.isArray(options.items)` lets an A1
handler choose a service argument according to whether the current value is
a list or a single action record. When an action table contains labelled
records, `for (const [name, {label}] of Object.entries(actions))` can trim
each label and select the matching service argument.

### Deliberate remaining limits

Runtime dictionary keys require a proven String and an all-null-prototype
receiver. Ordinary-object prototype lookup, descriptors, general key coercion,
optional access, and typed projections from unknown dictionary values remain
separate work. The constructor recognizes the unshadowed intrinsic
`Object.create(null)` or literal-computed `Object['create'](null)`, with exactly
one literal null argument through supported wrappers. Dynamic writes admit
plain `=` assignment; compound assignment, updates and deletion remain separate.

The array mutations include `pop`, `shift` and `reverse`. `push` is deferred because an array may already
have the maximum valid length; its overflow requires a source RangeError path,
not a private runtime trap. General Number bracket indexing remains separate
because it also addresses named properties for non-index Numbers.

String methods require proven String receivers, Number positions/endpoints and,
for search, explicit proven String needles. Missing or supported explicit undefined
positions use the method's normal default, and surplus arguments retain their
effects. General coercion, borrowed calls, optional calls and prototype changes
remain outside these slices. Broader binding/iteration and synchronous source
errors remain the next feature families in the working order above.

Local validation, hosted gates and native stack membership are recorded on the
individual PRs. These compiler authoring fixtures do not complete or reassign
the independent A1 application acceptance below.

## Follow-on String authoring

`trim`, `trimStart` and `trimEnd` now prepare source-owned labels and search
text using the same managed String lane. They preserve exact UTF-16 contents,
JavaScript whitespace rules and ordered surplus-argument effects, including
suspension. See [the method contract](managed-string-methods.md) for admission
limits and differential coverage. `endsWith` extends the search family with an
optional ending position, preserving the distinction between omitted/undefined
and explicit NaN endpoints. This compiler/runtime slice is independent
of the concurrent A1 persistent-session and reversible-toggle application work.

`indexOf` and `lastIndexOf` return UTF-16 positions for source label parsing and
range selection. Their Number results compose with helpers, captured handlers,
boxed fields and suspension; backward search preserves its distinct NaN default.
The String family remains bounded to proven receivers/needles and admitted
position arguments. Everyday bindings/iteration and synchronous source errors
remain the broader next feature families; completing every String builtin is
not a prerequisite.

## Everyday bindings and iteration

Local `const` and `let` declarations now extract nested records and lists through
mixed object/array binding patterns. Object shorthand, renaming and literal keys
compose with array elisions, empty patterns, identifier rest elements and lazy
defaults on identifiers or whole nested patterns. The same declaration path works
in ordinary blocks, helpers, supported async activations and traditional `for`
initializers. See
[object bindings](object-binding-patterns.md) and
[array bindings](array-binding-patterns.md) for the admitted surface and its
differential coverage.

The initializer executes once. Each selected nested receiver is saved while
its leaves finish in source order, so source rebinding or replacement of a parent
field cannot redirect a selection already in progress. Defaults run only for
Undefined and complete before the next selection, including across supported
suspension. Earlier leaves are visible to later defaults; abrupt completion
stops the remaining pattern. A default on a whole nested pattern finishes before
that pattern is entered, including for empty object or array patterns.

Every array pattern has independent intrinsic-iterator state, even when two
patterns consume the same array object. It observes live length until exhaustion
and then stays finished through later growth. A default in a descendant pattern
can change an ancestor array's length; the ancestor observes that change on its
next active step. Inner exhaustion does not finish a parent or sibling pattern.

Array rest collects the remaining values after earlier defaults into a fresh
dense array. Holes become own Undefined entries, references retain identity and
the copied slots are independent of the source. Rest observes the original
receiver's live length while active and stays empty after exhaustion. The rest
receiver has an Array proof. Iterating it follows the copied suffix through
aliases and chained rest expressions, including index writes on each array.
A1 can destructure each heading in the copied list and trim its label. Direct
indexed reads also use the corresponding source position when its kind is
proven. Missing positions can take binding defaults. Uncertain resizing,
cyclic sources and incompatible alias writes retain their conservative guards.

Array `for...of` accepts a `const` or `let` identifier or a supported object/array
pattern, including mixed nesting, identifier rest in arrays and defaults on
identifiers or nested patterns. It evaluates the iterable once and initializes
every head leaf before entering the body. Captured leaves receive fresh cells
each iteration. Existing completion machinery handles
unlabelled loop control and supported suspension in the RHS, head defaults or
body. See [Array and String iteration](array-iteration.md). A proven String can also
drive an identifier loop. An A1 handler can count characters in a display
label, treating a supplementary character as a single iteration and preserving
lone surrogate code units. A1 can snapshot its action list with `[...actions]`
and iterate the fresh array while retaining the identities of its records.
Spreading a String into a fresh array gives an A1 handler positional access
to the code points of a label, including a final supplementary character.
An enabled-action list can answer `enabled.includes(requested)` directly
before the handler invokes the service. Array `indexOf` and `lastIndexOf`
also supply the position of a matching action, including when duplicates occur.
Array `slice` lets a helper select a window of those actions for a captured
handler, preserving holes and the identities of copied records. The handler can
destructure each sliced record and trim its label, with copied-value proofs
accounting for source and result aliases and shared-child writes. Array `reverse`
reorders a history in place and returns that same array; a handler can combine
`slice().reverse()` to search a separate history from newest to oldest.
Array `shift` consumes the first queued action and moves the remaining sparse
positions down before the handler passes the current queue length to the service.

Every object or array receiver, including an empty nested pattern, needs a
positive source proof of its required kind. String/container facts remain
available to helpers and captures. Binding defaults can replace a proven
Undefined selection, including known missing fields or array holes, with a
proven fallback kind. Alias writes and unknown alternatives remain conservative;
an annotation or default does not erase an unknown alternative. Selected Numbers
stay boxed; this work adds no numeric unboxing. Pattern preparation and lowering
remain iterative, with the existing
128-local budget counting leaves and a shared 128-level container nesting limit.

Object rest copies the remaining own data fields of a proven object into a
fresh ordinary object after earlier defaults finish. The copy preserves slot
independence and reference identity in declarations and loop heads. Numeric
literal keys use the same JS property name in constructors, member accesses,
binding patterns and rest exclusions. An A1 handler extracts title metadata
from a numbered record, trims a copied heading with a proven String value
and passes the remaining heading count to the service. Retained-field proofs
include writes through aliases to the source, the copy and shared child objects.
A dictionary pattern can select a runtime String key and exclude its saved
value from rest even if another key suspends or reassigns the source variable.

An array rest target can destructure the completed tail with another array
pattern, including elisions, defaults and nested containers. Mutating the
original array from an inner default leaves the captured tail slots independent.

Object patterns read indexed and named fields on proven arrays, including
their current length. Object rest copies enumerable own entries from the
sparse array backing store. An array-rest target can likewise use an object
pattern on its freshly collected dense tail.

Parameter and assignment patterns,
general property-key conversion, primitive RHS coercion and general iterators remain
follow-on work.
`for-in`, `for await...of` and `var` or assignment loop heads are also outside this
slice. The compiler work does not alter the concurrent A1 application's session,
native view, service or packaging interfaces, or the browser stream's native
layout and engine-oracle work.

## Synchronous source errors

Ordinary functions now handle primitive policy failures through local
`try/catch`, including mutable captured reasons, nested rethrows, loops and
closures. The first slice reuses the existing typed completion seam without
changing the public call ABI. The original `gap-catch` fixture now executes with
its source and independent oracle preserved. A1 composition tests recover from
a local source failure before invoking the existing service, while provider
denial remains a terminal host outcome.

Synchronous `finally` now runs cleanup around normal completion, returns,
locally handled throws and unlabelled loop exits. One emitted cleanup body
preserves saved return/reason values and permits a cleanup completion to
override them. Ordinary helper cleanup also composes with async callers and
the existing A1 service; source failures and terminal host failures retain
their separate contracts.

See [synchronous errors](synchronous-errors.md) for the same-function lexical
boundary and differential tests. Cross-call errors, broader exception values
and source-error-producing runtime operations require further slices. In
particular, catchable intrinsic Errors need coherent value flow and coercion
before they can safely unlock array `push` overflow or nullable `Object.keys`.

## A1 scope and existing evidence

The separate A1 application stack, [#477](https://github.com/tseguine/grunkalunka/pull/477)
through [#480](https://github.com/tseguine/grunkalunka/pull/480), covers the service
host, source-driven input/repaint path and readable text. Its application-level
execution evidence and acceptance are tracked there; this language stack adds
authoring capabilities and composition coverage against the existing service.

The source-import composition suite now includes a helper that binds a nested
action record and a handler that extracts grouped headings, destructures each
heading during iteration and applies a lazy enabled default. The compiled source
derives scalar arguments for the existing Rust WIT service; its records and
arrays stay within the guest. These cases exercise the new authoring surface
without introducing a typed public A1 payload or resource boundary.
An optional-action case also applies whole-pattern defaults to missing view,
group and heading-option records before calling the service.
A rest case derives its count from a fresh heading list and verifies that
replacing a source slot leaves the copied heading reference intact.

Application validation and acceptance for source-imported input, the shared
state/reflow/repaint path, readable glyphs and equivalent native Rust behavior
are tracked in that separate stack. See
[A1 acceptance](a1-note-preview.md#candidate-next-a1-acceptance-and-compiler-priorities)
and [the scalar service profile](a1-ui-source-import.md). The language train makes
that application code easier to author; its compiler fixtures do not establish
those application acceptance gates.

The subsequent typed view-action or resource boundary is coordinated in
[#483](https://github.com/tseguine/grunkalunka/issues/483). It needs a separately
assigned authority, ownership, lifetime and failure contract shared with the
application stream. That coordination leaves the current scalar A1 stack and
this compiler feature train free to finish their assigned work.

Recent merged work already supplies managed references across related async
activations ([#375](https://github.com/tseguine/grunkalunka/pull/375),
[#377](https://github.com/tseguine/grunkalunka/pull/377)), object spread
([#379](https://github.com/tseguine/grunkalunka/pull/379)) and a scoped runtime
`Object.keys` operation with async source-error routing
([#380](https://github.com/tseguine/grunkalunka/pull/380)).
[#374](https://github.com/tseguine/grunkalunka/pull/374) fixes the original numeric
captured `for-let` reproducer. **2026-11-05 update:**
[#334](https://github.com/tseguine/grunkalunka/issues/334) is bounded to its two
unchanged managed-result and stored-function fixtures, now admitted through the
existing iteration-cell renewal. Older dated overview sections are not the new
implementation queue.

## PR stacking and validation

Use dependency-ordered PRs with each dependent branch based on the preceding
feature branch. Keep each PR useful and reviewable on its stated base, and state
the owning compiler/runtime stream, behavior added, remaining admission limits
and executed checks. The maintainer registers native stack membership when the
connector cannot; linked PR bases alone do not establish that registration.
The existing caretaker handles validation and integration. Local tests, hosted
checks, native stack membership and merge state remain distinct observations.

For each admitted slice, exercise actual JS/TS source and meaningful result,
effect-order or identity observations in the existing Node-Wasm/Wasmtime suites,
using native JavaScript as the semantic reference where applicable. Preserve
small-stack gates and existing resource diagnostics. Keep tests in canonical
suites with accurate dependency selection. Passing a narrow slice is sufficient
to move on; a known admitted-source miscompile needs correction or an explicit
scope boundary. Unimplemented valid syntax remains a tracked feature gap.

The broader [stack audit](https://github.com/tseguine/grunkalunka/issues/281),
[semantic-operations direction](https://github.com/tseguine/grunkalunka/issues/355)
and [managed async boundary](https://github.com/tseguine/grunkalunka/issues/356)
continue to inform implementation without making their entire scope a dependency
of every new feature.


Private synchronous named helpers now accept primitive literal parameter defaults.
Omitted or Undefined inputs select the fallback; null, false, zero and NaN remain
supplied values. Parameters retain per-call mutable cells and closure captures;
`function.length` stops at the first default. This bounded language feature keeps
entry, async, expression, rest and destructured parameter guards. See
[the compiler contract](compiler.md#primitive-literal-defaults-on-private-synchronous-helpers)
and the compiled-values / managed-logical differential suites.


Synchronous local arrows share literal defaults across inline calls, captures,
aliases, typed callback calls with supplied inputs and emitted own-field closures.
Defaulted creator parameters can be captured by returned functions. Public/async
entry, effectful/default-scope, complete callback signature and factory-result
limits remain explicit; reflection counts the position before the first default.


Function-scoped identifier `var` now hoists activation cells before synchronous
entry/helper/arrow bodies, preserves bare redeclarations, aliases parameters and
shares traditional-loop captures. Non-simple parameter environments copy selected
values into distinct body cells. Numeric proofs account for Undefined and all
writes; annotations erase. Pattern/callable-initializer, async
and catch-name guards remain bounded language gaps. The compiler contract and
compiled-values / managed-logical differential suites record the admitted slice.


Identifier var-for-of heads now reuse the function cell across intrinsic Array and
String visits, preserving pre-loop reads, empty-loop values, shared captures and
parameter aliases. Iteration writes widen source-kind analysis, including through
captures; no stale Number initializer may justify a later projection. Var pattern,
async and general-iterable heads remain separate language work.


Identifier var-for-in heads now share the function cell across existing own-key
enumeration. Empty sources preserve values, deleted keys do not assign the head,
and captures retain parameter aliases and final keys. String loop writes widen
kind analysis; pattern/async var and unproven receivers remain explicit guards.


Constant parameter defaults now include unshadowed Infinity/NaN and pure void
constants through the same helper/arrow/emitted-closure selection. Shadowed names,
TDZ reads and effects beneath void retain admission guards; no API is added.


Var destructuring declarations reuse hoisted cells for Object/Array bindings,
defaults, rest, parameter aliases and repeated declarations. Leaf-origin proofs
preserve safe projections and capture identity. Unproven pattern receivers, async var,
callable initializers and catch-name collisions remain separate work.


Var for-of patterns now share hoisted cells across proven container elements,
including defaults, nested selections and fresh rest values. Captures observe
later writes; zero visits preserve old values and origin proofs retain all loop
writes. Async var and unproven iterables/elements remain guarded.


Object destructuring can select length and canonical numeric keys from proven
Strings, including nested receivers, var captures and ordered defaults. Reads
preserve UTF-16 code units and saved RHS values. String object rest, dynamic/inherited
keys and unproven/mixed receivers remain guarded; no API is added.

For-in object binding heads now destructure each String key using the existing
String property planner. Per-visit lexical cells and shared var captures retain
their distinct behavior. Deleted keys skip the entire pattern, including default
side effects; empty enumerations preserve hoisted values. Corpus 150 covers own
key order, UTF-16, defaults, nested selections, control flow and capture identity.


String array destructuring now consumes code points for declarations and
iteration heads, including elisions, nested proven Strings, lazy defaults and
fresh rest Arrays. UTF-16 object-property selection remains distinct. A saved
String is materialized using the existing intrinsic spread operation; var cells,
lexical capture identity, source proofs and erased annotations stay unchanged.
Corpus 151 executes six exact former rejection sources and retains 796 guards.


Earlier-parameter defaults now copy initialized values in synchronous named
helpers, local arrows and emitted closures. Chained defaults preserve parameter
order, argument snapshots, primitive tags, reference identity and later body-var
separation. Resolved-symbol checks exclude self/later parameters, outer/body
scope reads and effects; public/async and unsupported value-lane guards remain.
Corpus 152 adds eighteen differential pairs and retains all 796 negative cases.


String Array destructuring assignments now update existing mutable cells using
code-point selection, elisions, defaults and fresh rest Arrays. The expression
returns the saved original String and its actual truthiness, including empty
Strings; private selection storage never becomes the result. Defaults, captures
and supported awaits retain source snapshots. Object-String, nested/member and
unproven-source assignment forms remain guarded. Corpus 153 adds thirteen pairs.


String object destructuring assignments now support static length and canonical
numeric keys, preserving UTF-16 selection, lazy defaults, captures and supported
awaits. The result remains the original String with its actual truthiness. Rest,
dynamic/inherited keys and unsupported targets remain guarded. Corpus 154 adds
ten differential pairs; no standard-library or host API is expanded.


Constant arithmetic defaults now support Number-only +, -, *, / and % in private
synchronous helpers/arrows, including nested constants, IEEE edge values, captured
results and emitted closures. Heap traversal retains source depth limits and does
not suppress operand admission beneath void. Scope, effects, coercion, async and
public-entry guards remain. Corpus 155 adds fourteen differential pairs.


Earlier-parameter unary defaults now support void, !, + and - through the same
lazy selection in helpers, arrows and emitted closures. Numeric signs require
four-kind primitive proofs; truthiness and void also support represented
references without invoking coercion hooks. Scope, source limits, unsupported
value lanes and public/async guards remain. Corpus 156 adds fifteen pairs.


Ordinary calls now accept fixed Array-literal argument spreads, including nested
literals, empty spreads and mixed arguments. Named helpers, inline/emitted arrows,
stored functions and callbacks share the borrowed source-argument view. Argument
effects, callee snapshots, omission/default selection, supported value/reference
lanes and supported await suspension retain their existing contracts. Dynamic
Array/String iterables, elisions, a spread thisArg to .call and unsupported callable
lanes remain guarded. No intrinsic/standard-library spread support is added.
Corpus 157 adds sixteen pairs and retains two exact former compiler guards as
executed positives, with all 796 compatibility negatives retained.


Mixed Array initializers now expand fixed Array-literal spread operands, including
multiple, nested and empty spreads. The heap projection preserves original
element expressions, effects, references, functions, String/Number position facts
and supported awaits. Outer elisions remain holes; spread elisions become own
Undefined slots. Dynamic Array/String operands in mixed initializers retain
guards; existing single-spread copy support stays available. Corpus 158 adds
thirteen differential pairs with all 796 compatibility negatives retained.
