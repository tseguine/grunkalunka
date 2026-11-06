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
Array runtime keys stay guarded because they might select non-configurable length.
Primitive receivers, non-reference
operands and optional chains remain separate work. Deleting an array's
non-configurable length stays guarded until a source TypeError can be represented;
it must not silently succeed or trap in the private helper.

Compiler units and IR-verifier mutations run in the fast PR profile. The existing
managed-source and async-suspension suites contain native-JS/Node-Wasm/Wasmtime
oracles. The large-literal constructor case uses an explicit bounded test fuel
budget, separate from the unchanged production per-call quota.
