//! Select property origins for reads and binding patterns at one solver iteration.
use super::*;

const LENGTH: &[u16] = &[108, 101, 110, 103, 116, 104];

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
    pub elements: &'a elements::Elements,
    pub index_deletes: &'a BTreeSet<NodeId>,
}

impl Members<'_, '_> {
    pub(super) fn select(&self, receiver: NodeId, source_key: &Key) -> Alternatives {
        enum Step {
            Read(Origin, Option<u32>),
            FinishRest(NodeId, Option<u32>),
        }
        let origins = |id: NodeId| self.values.get(&id).into_iter().flatten().copied();
        let mut initial = origins(receiver).map(|origin| Step::Read(origin, None));
        let mut work = Vec::new();
        let mut active = BTreeSet::new();
        let mut completed = BTreeSet::new();
        let mut found = Alternatives::new();
        while let Some(step) = work.pop().or_else(|| initial.next()) {
            let (origin, shifted) = match step {
                Step::Read(origin, shifted) => (origin, shifted),
                Step::FinishRest(object, shifted) => {
                    active.remove(&object);
                    completed.insert((object, shifted));
                    continue;
                }
            };
            let (Origin::Object(object) | Origin::NullObject(object) | Origin::Array(object)) =
                origin
            else {
                found.insert(Origin::Other);
                continue;
            };
            let translated;
            let key = if let Some(index) = shifted {
                translated = index.to_string().encode_utf16().collect();
                &translated
            } else {
                source_key
            };
            let array = matches!(origin, Origin::Array(_));
            let dictionary = matches!(origin, Origin::NullObject(_));
            if (array && (self.open_fields.contains(&object) || key.as_slice() == LENGTH))
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
            if self.index_deletes.contains(&object) && mutations::array_index(key).is_some() {
                found.insert(Origin::Other);
            }

            if shifted.is_some() && self.elements.may_resize.contains(&object) {
                // Structural mutations may have exhausted the iterator before
                // the observed index existed, even if it was added again later.
                found.insert(Origin::Other);
            }

            let copy = if let Some(rest) = self.rest_sources.get(&object) {
                if rest.excluded.contains(key) {
                    found.insert(Origin::Undefined);
                    continue;
                }
                Some((rest.receiver, shifted))
            } else if let Some(&(source, skipped)) = self.elements.rest_sources.get(&object) {
                // Only indexed values are copied. Out-of-bounds and named keys
                // start absent; independent writes to the result were included above.
                let Some(index) = mutations::array_index(key)
                    .and_then(|index| index.checked_add(skipped))
                    .filter(|index| *index < u32::MAX)
                else {
                    found.insert(Origin::Undefined);
                    continue;
                };
                if self.elements.may_resize.contains(&object) {
                    found.insert(Origin::Other);
                }
                Some((source, Some(index)))
            } else {
                None
            };

            if let Some((source, source_index)) = copy {
                if active.contains(&object) {
                    found.insert(Origin::Other);
                } else if !completed.contains(&(object, shifted)) {
                    // Converging paths can ask different positions of the same
                    // allocation. Only equal projections share a finished result.
                    active.insert(object);
                    work.push(Step::FinishRest(object, shifted));
                    for origin in origins(source) {
                        if key.as_slice() == LENGTH && matches!(origin, Origin::Array(_)) {
                            // Object rest copies the array's enumerable own
                            // fields. Its intrinsic length is never copied.
                            found.insert(Origin::Undefined);
                        } else {
                            work.push(Step::Read(origin, source_index));
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
