//! Source alternatives for every value an Array iteration may select.
use super::{Alternatives, BTreeMap, BTreeSet, Key, NodeId, Origin, extend, mutations};

#[derive(Default)]
pub(super) struct Elements {
    writes: BTreeMap<NodeId, Alternatives>,
    maybe_holes: BTreeSet<NodeId>,
}

impl Elements {
    pub(super) fn write(
        &mut self,
        array: NodeId,
        key: &Key,
        values: Alternatives,
        fields: &BTreeMap<NodeId, BTreeMap<Key, Option<NodeId>>>,
    ) -> bool {
        if key.as_slice() == [108, 101, 110, 103, 116, 104] {
            // Growth exposes holes. Shrinking can delete original slots that
            // later writes expose again, so every length write is conservative.
            return self.maybe_holes.insert(array);
        }
        if !mutations::array_index(key) {
            // Named properties are not selected by Array iteration.
            return false;
        }
        let mut changed = extend(&mut self.writes, array, values);
        if !fields.get(&array).is_some_and(|own| own.contains_key(key)) {
            // The flow-insensitive plan cannot prove contiguous extension or
            // that another mutation has not shortened the receiver meanwhile.
            changed |= self.maybe_holes.insert(array);
        }
        changed
    }

    pub(super) fn wildcard(&mut self, array: NodeId, values: Alternatives) -> bool {
        // The key could be length or a new index, even while its RHS remains
        // unresolved. Record that structural possibility independently of values.
        let changed = self.maybe_holes.insert(array);
        extend(&mut self.writes, array, values) || changed
    }

    pub(super) fn select(
        &self,
        receiver: NodeId,
        values: &BTreeMap<NodeId, Alternatives>,
        fields: &BTreeMap<NodeId, BTreeMap<Key, Option<NodeId>>>,
        index_deletes: &BTreeSet<NodeId>,
    ) -> Alternatives {
        let mut found = Alternatives::new();
        for origin in values.get(&receiver).into_iter().flatten() {
            let Origin::Array(array) = origin else {
                found.insert(Origin::Other);
                continue;
            };
            if let Some(own) = fields.get(array) {
                for (key, value) in own {
                    if mutations::array_index(key) {
                        if let Some(value) = value {
                            found.extend(values.get(value).into_iter().flatten());
                        } else {
                            found.insert(Origin::Other);
                        }
                    }
                }
            } else {
                found.insert(Origin::Other);
            }
            found.extend(self.writes.get(array).into_iter().flatten());
            if self.maybe_holes.contains(array) || index_deletes.contains(array) {
                found.insert(Origin::Other);
            }
        }
        // An unmutated empty Array contributes no selected value. In particular,
        // [] | ["text"] may retain String provenance from its nonempty branch.
        found
    }
}
