//! Select property origins for reads and binding patterns at one solver iteration.
use super::*;

const LENGTH: &[u16] = &[108, 101, 110, 103, 116, 104];

pub(super) struct Rest {
    pub receiver: NodeId,
    pub excluded: BTreeSet<Key>,
    pub dynamic_excluded: bool,
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
    pub(super) fn string_key_value(
        &self,
        receiver: NodeId,
        key: NodeId,
        domain: &record_names::Domain,
    ) -> Alternatives {
        let Some(keys) = self.values.get(&key).filter(|origins| !origins.is_empty()) else {
            return Alternatives::new();
        };
        let Some(receivers) = self
            .values
            .get(&receiver)
            .filter(|origins| !origins.is_empty())
        else {
            return Alternatives::new();
        };
        if keys.len() != 1
            || !keys.contains(&Origin::String)
            || !receivers.iter().all(|origin| {
                matches!(origin, Origin::NullObject(_))
                    || (!matches!(domain, record_names::Domain::Unknown)
                        && matches!(origin, Origin::Object(_)))
            })
        {
            return BTreeSet::from([Origin::Other]);
        }
        if let record_names::Domain::Fields(fields) = domain {
            return fields
                .iter()
                .flat_map(|key| self.select(receiver, key))
                .collect();
        }
        let mut found = self.enumerable_values(receiver);
        // A runtime name may select no own field even after a known write.
        found.insert(Origin::Undefined);
        found
    }

    pub(super) fn optional_computed(
        &self,
        receiver: NodeId,
        key: NodeId,
        domain: &record_names::Domain,
    ) -> Alternatives {
        let Some(keys) = self.values.get(&key).filter(|v| !v.is_empty()) else {
            return Alternatives::new();
        };
        let Some(roots) = self.values.get(&receiver).filter(|v| !v.is_empty()) else {
            return Alternatives::new();
        };
        if keys.iter().all(|origin| *origin == Origin::Number)
            && roots
                .iter()
                .all(|origin| matches!(origin, Origin::String | Origin::Null | Origin::Undefined))
        {
            BTreeSet::from([Origin::String, Origin::Undefined])
        } else if keys.iter().all(|origin| *origin == Origin::String)
            && roots.iter().all(|origin| {
                matches!(
                    origin,
                    Origin::NullObject(_) | Origin::Null | Origin::Undefined
                ) || (!matches!(domain, record_names::Domain::Unknown)
                    && matches!(origin, Origin::Object(_)))
            })
        {
            if let record_names::Domain::Fields(fields) = domain {
                return fields
                    .iter()
                    .flat_map(|key| self.optional_select(receiver, key))
                    .collect();
            }
            let mut found = self.enumerable_values_for(receiver, true);
            found.insert(Origin::Undefined);
            found
        } else {
            BTreeSet::from([Origin::Other])
        }
    }

    pub(super) fn pattern_unsigned_value(&self, receiver: NodeId) -> Alternatives {
        if self.values.get(&receiver).is_some_and(|roots| {
            !roots.is_empty() && roots.iter().all(|origin| *origin == Origin::String)
        }) {
            return BTreeSet::from([Origin::String, Origin::Undefined]);
        }
        self.unsigned_value(receiver)
    }

    pub(super) fn unsigned_value(&self, receiver: NodeId) -> Alternatives {
        self.unsigned_select(receiver, false)
    }
    pub(super) fn optional_unsigned_value(&self, receiver: NodeId) -> Alternatives {
        self.unsigned_select(receiver, true)
    }
    fn unsigned_select(&self, receiver: NodeId, optional: bool) -> Alternatives {
        let Some(receivers) = self
            .values
            .get(&receiver)
            .filter(|values| !values.is_empty())
        else {
            return Alternatives::new();
        };
        if !receivers.iter().all(|origin| {
            matches!(
                origin,
                Origin::Object(_) | Origin::NullObject(_) | Origin::Array(_)
            ) || (optional && matches!(origin, Origin::Null | Origin::Undefined | Origin::String))
        }) {
            return BTreeSet::from([Origin::Other]);
        }
        // Numeric names have no inherited value in the represented pristine realm.
        // Include every own initializer/write conservatively, and possible absence.
        let mut found = self.enumerable_values_for(receiver, optional);
        found.insert(Origin::Undefined);
        found
    }

    pub(super) fn enumerable_values(&self, receiver: NodeId) -> Alternatives {
        self.enumerable_values_for(receiver, false)
    }
    fn enumerable_values_for(&self, receiver: NodeId, optional: bool) -> Alternatives {
        let mut found = Alternatives::new();
        let origins = |id: NodeId| self.values.get(&id).into_iter().flatten().copied();
        for origin in origins(receiver)
            .filter(|origin| !optional || !matches!(origin, Origin::Null | Origin::Undefined))
        {
            let site = match origin {
                Origin::Object(site) | Origin::Array(site) | Origin::NullObject(site) => site,
                Origin::String => {
                    found.insert(Origin::String);
                    continue;
                }
                _ => {
                    found.insert(Origin::Other);
                    continue;
                }
            };
            // Dynamic names can collide, but every value of a data property
            // still originates in an initializer or a write to this allocation.
            match self.semantic.nodes().kind(site) {
                AstKind::ObjectExpression(object) => {
                    for property in &object.properties {
                        match property {
                            ObjectPropertyKind::ObjectProperty(p) => {
                                found.extend(origins(p.value.node_id()))
                            }
                            ObjectPropertyKind::SpreadProperty(_) => {
                                found.insert(Origin::Other);
                            }
                        }
                    }
                }
                kind if object_intrinsics::array_constructor_site(kind, self.semantic) => {
                    if let Some(fields) = self.fields.get(&site) {
                        for value in fields.values().flatten() {
                            found.extend(origins(*value));
                        }
                    }
                }
                AstKind::ArrayExpression(array) => {
                    if let Some(elements) = iteration::literal_elements(array) {
                        for element in elements {
                            match element {
                                iteration::LiteralElement::Value(expr, _) => {
                                    found.extend(origins(expr.node_id()))
                                }
                                iteration::LiteralElement::Undefined => {
                                    found.insert(Origin::Undefined);
                                }
                                iteration::LiteralElement::Hole => (),
                            }
                        }
                    } else {
                        found.insert(Origin::Other);
                    }
                }
                _ if matches!(origin, Origin::NullObject(_)) => (),
                _ => {
                    found.insert(Origin::Other);
                }
            }
            for ((_, key), written) in self
                .field_writes
                .range((site, Key::new())..)
                .take_while(|((object, _), _)| *object == site)
            {
                if !matches!(origin, Origin::Array(_)) || key.as_slice() != LENGTH {
                    found.extend(written);
                }
            }
            found.extend(self.wildcard_writes.get(&site).into_iter().flatten());
        }
        found
    }

    pub(super) fn select(&self, receiver: NodeId, source_key: &Key) -> Alternatives {
        self.select_member(receiver, source_key, false)
    }

    pub(super) fn optional_select(&self, receiver: NodeId, source_key: &Key) -> Alternatives {
        self.select_member(receiver, source_key, true)
    }

    fn select_member(&self, receiver: NodeId, source_key: &Key, optional: bool) -> Alternatives {
        enum Step {
            Read(Origin, Option<u32>),
            FinishRest(NodeId, Option<u32>),
        }
        let origins = |id: NodeId| self.values.get(&id).into_iter().flatten().copied();
        let roots: Vec<_> = origins(receiver).collect();
        let nullish = optional
            && roots
                .iter()
                .any(|origin| matches!(origin, Origin::Null | Origin::Undefined));
        let mut initial = roots
            .into_iter()
            .filter(|origin| !optional || !matches!(origin, Origin::Null | Origin::Undefined))
            .map(|origin| Step::Read(origin, None));
        let mut work = Vec::new();
        let mut active = BTreeSet::new();
        let mut completed = BTreeSet::new();
        let mut found = if nullish {
            BTreeSet::from([Origin::Undefined])
        } else {
            Alternatives::new()
        };
        while let Some(step) = work.pop().or_else(|| initial.next()) {
            let (origin, shifted) = match step {
                Step::Read(Origin::String, None) => {
                    if source_key.as_slice() == LENGTH {
                        found.insert(Origin::Number);
                    } else if binding_patterns::string_index(source_key).is_some() {
                        found.extend([Origin::String, Origin::Undefined]);
                    } else {
                        found.insert(Origin::Other);
                    }
                    continue;
                }
                Step::Read(Origin::String, Some(_)) => {
                    // An indexed projection through an iterator copy may miss.
                    found.extend([Origin::String, Origin::Undefined]);
                    continue;
                }
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
            if optional && array && key.as_slice() == LENGTH {
                found.insert(Origin::Number);
                continue;
            }
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
            if (self.index_deletes.contains(&object) || self.elements.reordered.contains(&object))
                && mutations::array_index(key).is_some()
            {
                found.insert(Origin::Other);
            }

            if self.elements.key_lists.contains(&object) && mutations::array_index(key).is_some() {
                // Indexed access may miss even though every initially yielded
                // element is a String. Previously collected alias writes still apply.
                found.extend([Origin::String, Origin::Undefined]);
                continue;
            }
            if self.elements.pair_sites.contains(&object) && mutations::array_index(key) == Some(0)
            {
                found.insert(Origin::String);
                if self.elements.may_resize.contains(&object) {
                    found.insert(Origin::Other);
                }
                continue;
            }
            if let Some(snapshot) = self.elements.enumerated_values.get(&object)
                && let Some(index) = mutations::array_index(key)
            {
                if !self.elements.pair_sites.contains(&object) {
                    found.extend(snapshot);
                    found.insert(Origin::Undefined);
                    continue;
                }
                if index == 1 {
                    found.extend(snapshot);
                    if self.elements.may_resize.contains(&object) {
                        found.insert(Origin::Other);
                    }
                    continue;
                }
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
                // An unknown excluded name can remove this exact field. Its
                // possible value remains a source alternative, never a proof
                // that it is present on the fresh rest object.
                if rest.dynamic_excluded {
                    found.insert(Origin::Undefined);
                }
                if self.values.get(&rest.receiver).is_some_and(|roots| {
                    !roots.is_empty() && roots.iter().all(|origin| *origin == Origin::String)
                }) && binding_patterns::string_index(key).is_none()
                {
                    // A String's length and inherited names are not copied.
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
