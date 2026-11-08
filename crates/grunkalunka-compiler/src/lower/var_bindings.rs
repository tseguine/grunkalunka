//! Function-scoped var cells; declarations write existing cells, never rebind them.
use super::*;
use oxc_ast::{AstKind, ast::FormalParameters};
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn owner(id: NodeId, semantic: &Semantic<'_>) -> Option<NodeId> {
    for kind in semantic.nodes().ancestor_kinds(id) {
        match kind {
            AstKind::Function(f) => return (!f.declare).then_some(f.node_id.get()),
            AstKind::ArrowFunctionExpression(a) => return Some(a.node_id.get()),
            _ => (),
        }
    }
    None
}

pub(super) fn locals(semantic: &Semantic<'_>) -> Vec<(SymbolId, NodeId, NodeId)> {
    let mut found = Vec::new();
    for node in semantic.nodes().iter() {
        let AstKind::VariableDeclaration(d) = node.kind() else {
            continue;
        };
        if d.kind != VariableDeclarationKind::Var || d.declare {
            continue;
        }
        let Some(body) = owner(node.id(), semantic) else {
            continue;
        };
        for local in &d.declarations {
            if let BindingPattern::BindingIdentifier(id) = &local.id
                && let Some(symbol) = id.symbol_id.get()
            {
                found.push((symbol, local.node_id.get(), body));
            }
        }
    }
    found
}

pub(super) fn symbols(semantic: &Semantic<'_>) -> BTreeSet<SymbolId> {
    locals(semantic)
        .into_iter()
        .map(|(symbol, _, _)| symbol)
        .collect()
}

pub(super) fn parameters(semantic: &Semantic<'_>) -> BTreeSet<SymbolId> {
    semantic
        .nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::FormalParameter(p) => match &p.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

pub(super) fn iteration_symbols(semantic: &Semantic<'_>) -> BTreeSet<SymbolId> {
    semantic
        .nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::ForOfStatement(source) => {
                iteration::var_iteration_binding(&source.left).and_then(|id| id.symbol_id.get())
            }
            AstKind::ForInStatement(source) => {
                iteration::var_iteration_binding(&source.left).and_then(|id| id.symbol_id.get())
            }
            _ => None,
        })
        .collect()
}

pub(super) fn present(semantic: &Semantic<'_>) -> bool {
    !locals(semantic).is_empty()
}

pub(crate) fn validate(semantic: &Semantic<'_>, async_entry: bool) -> Result<(), String> {
    let mut count = 0;
    for node in semantic.nodes().iter() {
        let AstKind::VariableDeclaration(d) = node.kind() else {
            continue;
        };
        if d.kind != VariableDeclarationKind::Var
            || d.declare
            || owner(node.id(), semantic).is_none()
        {
            continue;
        }
        let asynchronous = semantic
            .nodes()
            .ancestor_kinds(node.id())
            .find_map(|kind| match kind {
                AstKind::Function(f) => Some(f.r#async),
                AstKind::ArrowFunctionExpression(a) => Some(a.r#async),
                _ => None,
            })
            .unwrap_or(false);
        if async_entry || asynchronous {
            return Err(unsupported(
                d,
                "var bindings in async modules or functions are not yet admitted",
            ));
        }
        for local in &d.declarations {
            let BindingPattern::BindingIdentifier(id) = &local.id else {
                return Err(unsupported(
                    local,
                    "var requires ordinary identifier bindings",
                ));
            };
            count += 1;
            if count > 128 {
                return Err(unsupported(local, "at most 128 source locals are admitted"));
            }
            for parent in semantic.nodes().ancestor_kinds(node.id()) {
                match parent {
                    AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => break,
                    AstKind::CatchClause(c)
                        if c.param.as_ref().is_some_and(|p| {
                            matches!(&p.pattern,BindingPattern::BindingIdentifier(name) if name.name==id.name)
                                || binding_patterns::leaves(&p.pattern).any(|leaf| leaf.name == id.name)
                        }) =>
                    {
                        return Err(unsupported(local, "var cannot redeclare a catch binding"));
                    }
                    _ => (),
                }
            }
        }
    }
    Ok(())
}

pub(super) fn aliases(semantic: &Semantic<'_>) -> BTreeMap<SymbolId, SymbolId> {
    let mut result = BTreeMap::new();
    for (symbol, _, body) in locals(semantic) {
        let params = match semantic.nodes().kind(body) {
            AstKind::Function(f) => &f.params,
            AstKind::ArrowFunctionExpression(a) => &a.params,
            _ => continue,
        };
        for p in &params.items {
            if let BindingPattern::BindingIdentifier(id) = &p.pattern
                && let Some(parameter) = id.symbol_id.get()
                && parameter != symbol
                && semantic.scoping().symbol_name(symbol) == id.name.as_str()
            {
                result.insert(symbol, parameter);
            }
        }
    }
    result
}

// A top-level initialization dominates later source reads, including reads in
// subsequently created closures. Branch/loop/try bodies do not establish it.
fn straight(id: NodeId, semantic: &Semantic<'_>, declaration: bool) -> bool {
    let span = semantic.nodes().kind(id).span();
    for parent in semantic.nodes().ancestor_kinds(id) {
        match parent {
            AstKind::VariableDeclaration(_) if declaration => (),
            AstKind::ExpressionStatement(_) if !declaration => (),
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSSatisfiesExpression(_)
                if !declaration => {}
            AstKind::ForStatement(f)
                if declaration
                    && f.init.as_ref().is_some_and(|init| {
                        init.span().start <= span.start && span.end <= init.span().end
                    }) => {}
            AstKind::BlockStatement(_) => (),
            AstKind::FunctionBody(_) => return true,
            _ => return false,
        }
    }
    false
}

// Separate the initial hoisted Undefined from later writes. Never use textual
// ordering to exclude captured writes or a loop backedge. Unknown write forms
// suppress the refinement instead of inventing a Number or container proof.
pub(super) fn reads(semantic: &Semantic<'_>) -> BTreeMap<NodeId, Vec<NodeId>> {
    let mut writes = BTreeMap::<SymbolId, Vec<NodeId>>::new();
    let mut first = BTreeMap::<SymbolId, u32>::new();
    let mut owners = BTreeMap::new();
    for (symbol, local, body) in locals(semantic) {
        owners.insert(symbol, body);
        let AstKind::VariableDeclarator(d) = semantic.nodes().kind(local) else {
            unreachable!()
        };
        if let Some(init) = &d.init {
            writes.entry(symbol).or_default().push(init.node_id());
            if straight(local, semantic, true) {
                first
                    .entry(symbol)
                    .and_modify(|end| *end = (*end).min(d.span.end))
                    .or_insert(d.span.end);
            }
        }
    }
    // Iteration assignments can introduce a different kind even when the loop
    // follows an ordinary initializer. Never discard their origins via a stale
    // initializer-only refinement; zero visits preserve the prior value.
    let mut invalid = iteration_symbols(semantic);
    for (&symbol, &body) in &owners {
        for reference in semantic
            .scoping()
            .get_resolved_references(symbol)
            .filter(|r| r.is_write())
        {
            let mut write = None;
            for parent in semantic.nodes().ancestors(reference.node_id()) {
                match parent.kind() {
                    AstKind::AssignmentExpression(a) => {
                        if a.left
                            .as_simple_assignment_target()
                            .and_then(|t| target_identifier(t, 0).ok().flatten())
                            .and_then(|i| i.reference_id.get())
                            .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                            == Some(symbol)
                        {
                            write = Some(parent.id());
                            if owner(parent.id(), semantic) == Some(body)
                                && straight(parent.id(), semantic, false)
                            {
                                first
                                    .entry(symbol)
                                    .and_modify(|end| *end = (*end).min(a.span.end))
                                    .or_insert(a.span.end);
                            }
                        }
                        break;
                    }
                    AstKind::UpdateExpression(update) => {
                        write = Some(parent.id());
                        if owner(parent.id(), semantic) == Some(body)
                            && straight(parent.id(), semantic, false)
                        {
                            first
                                .entry(symbol)
                                .and_modify(|end| *end = (*end).min(update.span.end))
                                .or_insert(update.span.end);
                        }
                        break;
                    }
                    AstKind::ParenthesizedExpression(_)
                    | AstKind::TSAsExpression(_)
                    | AstKind::TSNonNullExpression(_)
                    | AstKind::TSTypeAssertion(_)
                    | AstKind::TSSatisfiesExpression(_) => (),
                    _ => break,
                }
            }
            if let Some(write) = write {
                writes.entry(symbol).or_default().push(write);
            } else {
                invalid.insert(symbol);
            }
        }
    }
    let mut result = BTreeMap::new();
    for (symbol, end) in first {
        if invalid.contains(&symbol) {
            continue;
        }
        for r in semantic
            .scoping()
            .get_resolved_references(symbol)
            .filter(|r| r.is_read())
        {
            if semantic.nodes().kind(r.node_id()).span().start >= end {
                result.insert(
                    r.node_id(),
                    writes.get(&symbol).cloned().unwrap_or_default(),
                );
            }
        }
    }
    result
}

impl Lowering<'_, '_> {
    fn box_var_input(
        &mut self,
        node: &impl GetSpan,
        input: ValueId,
        region: &mut Region,
    ) -> Result<ValueId, String> {
        let ty = self.values[input.0];
        if ty == Type::ManagedValue {
            return Ok(input);
        }
        let result = self.value(Type::ManagedValue);
        let op = match ty {
            Type::ManagedObject => Op::ManagedFromObject { result, input },
            Type::ManagedArray => Op::ManagedFromArray { result, input },
            Type::Closure(_) => Op::ManagedFromClosure { result, input },
            Type::Number | Type::Predicate | Type::General => {
                let primitive = if ty == Type::General {
                    input
                } else {
                    let primitive = self.value(Type::General);
                    Self::push(
                        region,
                        node,
                        Op::Pack {
                            result: primitive,
                            input,
                        },
                    );
                    primitive
                };
                Op::ManagedPrimitive {
                    result,
                    input: primitive,
                }
            }
            _ => {
                return Err(unsupported(
                    node,
                    "var parameter aliases require primitive or managed inputs",
                ));
            }
        };
        Self::push(region, node, op);
        Ok(result)
    }

    pub(crate) fn initialize_var_bindings(
        &mut self,
        body: NodeId,
        params: &FormalParameters<'_>,
        region: &mut Region,
    ) -> Result<(), String> {
        let semantic = self.semantic;
        let iterated = iteration_symbols(semantic);
        let mut seen = BTreeSet::new();
        for (symbol, site, owner) in locals(semantic) {
            if owner != body || !seen.insert(symbol) {
                continue;
            }
            let node = semantic.nodes().kind(site);
            let existing = self.bindings.iter().position(|b| b.symbol == symbol);
            let mut copied = None;
            if let Some(index) = existing {
                let slot = self.bindings[index].slot;
                let ty = self.slots[slot.0].ty;
                if !iterated.contains(&symbol) || ty == Type::ManagedValue {
                    continue;
                }
                // Widen parameter aliases before any body-created capture, rather
                // than replacing their cell when the loop is first reached.
                let input = self.value(ty);
                Self::push(
                    region,
                    &node,
                    Op::Load {
                        result: input,
                        slot,
                    },
                );
                copied = Some(self.box_var_input(&node, input, region)?);
            } else {
                for p in &params.items {
                    if let BindingPattern::BindingIdentifier(id) = &p.pattern
                        && id.name.as_str() == semantic.scoping().symbol_name(symbol)
                        && let Some(parameter) = id.symbol_id.get()
                        && let Some(binding) = self.bindings.iter().find(|b| b.symbol == parameter)
                    {
                        let slot = binding.slot;
                        let ty = self.slots[slot.0].ty;
                        let input = self.value(ty);
                        Self::push(
                            region,
                            &node,
                            Op::Load {
                                result: input,
                                slot,
                            },
                        );
                        copied = Some(self.box_var_input(&node, input, region)?);
                        break;
                    }
                }
            }
            let value = copied.unwrap_or_else(|| self.binding_undefined(&node, region));
            let slot = SlotId(self.slots.len());
            self.slots.push(Slot {
                ty: Type::ManagedValue,
                owner: region.id,
                mutable: true,
                parameter: false,
                span: node.span(),
            });
            Self::push(region, &node, Op::Initialize { slot, value });
            if let Some(index) = existing {
                self.bindings[index].slot = slot;
            } else {
                self.bindings.push(NumericBinding {
                    symbol,
                    slot,
                    mutable: true,
                });
            }
        }
        Ok(())
    }
}
