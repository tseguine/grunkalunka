//! Source alternatives for every value an Array iteration may select.
use super::{Alternatives, BTreeMap, BTreeSet, Key, NodeId, Origin, extend, mutations};

#[derive(Default)]
pub(super) struct Elements {
    // Rest allocation -> (source pattern, number of consumed positions).
    rest_sources: BTreeMap<NodeId, (NodeId, u32)>,
    writes: BTreeMap<(NodeId, u32), Alternatives>,
    wildcard_writes: BTreeMap<NodeId, Alternatives>,
    maybe_holes: BTreeSet<NodeId>,
}

impl Elements {
    pub(super) fn add_rest(&mut self, array: NodeId, receiver: NodeId, start: u32) {
        self.rest_sources.insert(array, (receiver, start));
    }

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
        let Some(index) = mutations::array_index(key) else {
            // Named properties are not selected by Array iteration.
            return false;
        };
        let mut changed = extend(&mut self.writes, (array, index), values);
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
        extend(&mut self.wildcard_writes, array, values) || changed
    }

    pub(super) fn select(
        &self,
        receiver: NodeId,
        values: &BTreeMap<NodeId, Alternatives>,
        fields: &BTreeMap<NodeId, BTreeMap<Key, Option<NodeId>>>,
        index_deletes: &BTreeSet<NodeId>,
    ) -> Alternatives {
        enum Step {
            Read(Origin, u32),
            Finish(NodeId),
        }

        let origins = |id: NodeId| values.get(&id).into_iter().flatten().copied();
        let mut roots = origins(receiver).map(|origin| Step::Read(origin, 0));
        let mut work = Vec::new();
        let mut active = BTreeSet::new();
        let mut min_start = BTreeMap::new();
        let mut found = Alternatives::new();
        while let Some(step) = work.pop().or_else(|| roots.next()) {
            let (array, start) = match step {
                Step::Read(Origin::Array(array), start) => (array, start),
                Step::Read(_, _) => {
                    found.insert(Origin::Other);
                    continue;
                }
                Step::Finish(array) => {
                    active.remove(&array);
                    continue;
                }
            };
            if self.maybe_holes.contains(&array) || index_deletes.contains(&array) {
                found.insert(Origin::Other);
            }
            found.extend(self.wildcard_writes.get(&array).into_iter().flatten());
            for (_, written) in self.writes.range((array, start)..=(array, u32::MAX)) {
                found.extend(written);
            }
            if let Some(&(source, skipped)) = self.rest_sources.get(&array) {
                if active.contains(&array) {
                    found.insert(Origin::Other);
                    continue;
                }
                // A previous traversal starting earlier already covers all of
                // this suffix. Revisit only when a path exposes more positions.
                if min_start
                    .get(&array)
                    .is_some_and(|&earlier| earlier <= start)
                {
                    continue;
                }
                min_start.insert(array, start);
                active.insert(array);
                work.push(Step::Finish(array));
                let next = start.saturating_add(skipped);
                work.extend(origins(source).map(|origin| Step::Read(origin, next)));
                continue;
            }
            if let Some(own) = fields.get(&array) {
                for (key, value) in own {
                    if mutations::array_index(key).is_some_and(|index| index >= start) {
                        if let Some(value) = value {
                            found.extend(origins(*value));
                        } else {
                            found.insert(Origin::Other);
                        }
                    }
                }
            } else {
                found.insert(Origin::Other);
            }
        }
        // An unmutated empty Array contributes no selected value. In particular,
        // [] | ["text"] may retain String provenance from its nonempty branch.
        found
    }
}
