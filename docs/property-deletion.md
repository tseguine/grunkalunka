# Property deletion

`delete receiver.key` and literal-key bracket forms operate on source-proven
ordinary objects, null-prototype dictionaries and arrays. Represented own data
properties are configurable: deletion returns true whether the entry existed
or was already absent. Deleting an inherited name leaves its prototype entry
unchanged. This operation does not read the selected property's value.

The receiver executes once. Removing an array index creates a hole and preserves
length; named array properties use the same own-entry storage. Remaining property
order is preserved, and recreating a deleted named property appends it to the
insertion order. Aliases, captured references and suspended activations observe
the same object mutation. A for-in loop skips an initially enumerated key that
has been deleted before its visit.

Field-kind analysis adds an Undefined alternative through aliases and helper
arguments. It cannot keep assuming a deleted field is a String, container or
callable. A supported destructuring default can replace that missing selection.

The key may be a String/Number literal, including negative and fractional numeric
names. On objects and dictionaries, runtime source-proven String keys also work:
the receiver is saved before the key expression runs, including across await or
receiver-variable reassignment. A failed key expression leaves the object intact.
Array runtime String keys stay guarded because they might select non-configurable length.
Proven Number and Boolean receivers have no own properties, so deletion returns
true after the receiver and admitted primitive key expressions finish in order.
Non-reference operands also evaluate their effects before returning true. In
particular, `delete (0, object.key)` reads the selected value and leaves the
property intact, while `delete (object.key)` still deletes that reference.
Await and rejection follow the same distinction. The unchanged original numeric
receiver fixture is now an executable canonical corpus case.

Single-step optional member references also support deletion:
`delete receiver?.key` and admitted literal/computed bracket keys return true
without selecting a property when the receiver is Null or Undefined. A non-nullish
object/Array is saved before key evaluation; key effects and supported awaits run
only in that branch. This deletes the reference itself, while a sequence such as
`delete (0, receiver?.key)` still deletes a value and leaves the property intact.
Runtime String keys work on objects/dictionaries, and source-`>>>`-proven unsigned
keys also work on Arrays. Deletion widens selected-value facts through aliases,
helpers and Array elements, preserving the ordinary missing-value guards.

Non-optional nullish reference receivers, String reference receivers, optional
calls/multi-step chains, effectful object-key coercion and general descriptors
remain separate work. Deleting an array's
non-configurable length stays guarded until a source TypeError can be represented;
it must not silently succeed or trap in the private helper.

Compiler units and IR-verifier mutations run in the fast PR profile. The existing
managed-source and async-suspension suites contain native-JS/Node-Wasm/Wasmtime
oracles. The large-literal constructor case uses an explicit bounded test fuel
budget, separate from the unchanged production per-call quota.
