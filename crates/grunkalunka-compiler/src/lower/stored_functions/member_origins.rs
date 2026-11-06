//! Select property origins for reads and binding patterns at one solver iteration.
use super::*;

pub(super) struct Rest {
    pub receiver: NodeId,
    pub excluded: BTreeSet<Key>,
}

pub(super) struct Members<'a, 'ast> {
    pub semantic: &'a Semantic<'ast>,
    pub values: &'a BTreeMap<NodeId, Alternatives>,
    pub fields: &'a BTreeMap<NodeId, BTreeMap<Key, Option<NodeId>>>,
    pub open_fields: &'a BTreeSet<NodeId>,
    pub rest_sources: &'a BTreeMap<NodeId, Rest>,
    pub field_writes: &'a BTreeMap<(NodeId, Key), Alternatives>,
    pub wildcard_writes: &'a BTreeMap<NodeId, Alternatives>,
    pub index_deletes: &'a BTreeSet<NodeId>,
}

impl Members<'_, '_> {
    pub(super) fn select(&self, receiver: NodeId, key: &Key) -> Alternatives {
        enum Step {
            Read(Origin),
            FinishRest(NodeId),
        }
        let origins = |id: NodeId| self.values.get(&id).into_iter().flatten().copied();
        let mut initial = origins(receiver).map(Step::Read);
        let mut work = Vec::new();
        let mut completed = BTreeMap::new();
        let mut found = Alternatives::new();
        while let Some(step) = work.pop().or_else(|| initial.next()) {
            let origin = match step {
                Step::Read(origin) => origin,
                Step::FinishRest(object) => {
                    completed.insert(object, true);
                    continue;
                }
            };
            let (Origin::Object(object) | Origin::NullObject(object) | Origin::Array(object)) =
                origin
            else {
                found.insert(Origin::Other);
                continue;
            };
            let array = matches!(origin, Origin::Array(_));
            let dictionary = matches!(origin, Origin::NullObject(_));
            if (array
                && (self.open_fields.contains(&object)
                    || key.as_slice() == [108, 101, 110, 103, 116, 104]))
                || evaluate::managed_member_key(
                    &self.semantic.nodes().kind(object),
                    key.clone(),
                    array,
                    dictionary,
                )
                .is_err()
            {
                // Intrinsic length, inherited keys and opaque array positions
                // cannot be inferred to be absent own properties.
                found.insert(Origin::Other);
                continue;
            }

            if let Some(written) = self.field_writes.get(&(object, key.clone())) {
                found.extend(written);
            }
            if let Some(written) = self.wildcard_writes.get(&object) {
                found.extend(written);
            }
            if self.index_deletes.contains(&object) && mutations::array_index(key) {
                found.insert(Origin::Other);
            }

            if let Some(rest) = self.rest_sources.get(&object) {
                if rest.excluded.contains(key) {
                    found.insert(Origin::Undefined);
                } else {
                    // Follow the copy's initial value through the source while
                    // retaining writes to each independent slot. A source cycle
                    // contributes uncertainty; converging acyclic paths share work.
                    match completed.get(&object) {
                        Some(true) => (),
                        Some(false) => {
                            found.insert(Origin::Other);
                        }
                        None => {
                            completed.insert(object, false);
                            work.push(Step::FinishRest(object));
                            work.extend(origins(rest.receiver).map(Step::Read));
                        }
                    }
                }
                continue;
            }

            match self.fields.get(&object).and_then(|own| own.get(key)) {
                Some(Some(value)) => found.extend(origins(*value)),
                Some(None) if array => {
                    found.insert(Origin::Undefined);
                }
                None if self.fields.contains_key(&object)
                    && !self.open_fields.contains(&object) =>
                {
                    found.insert(Origin::Undefined);
                }
                _ => {
                    found.insert(Origin::Other);
                }
            }
        }
        found
    }
}
