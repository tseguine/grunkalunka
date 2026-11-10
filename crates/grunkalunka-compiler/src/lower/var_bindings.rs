//! Function-scoped var cells; declarations write existing cells, never rebind them.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{AssignmentExpression, FormalParameters},
};
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

fn names<'n, 'a>(pattern: &'n BindingPattern<'a>) -> Vec<&'n oxc_ast::ast::BindingIdentifier<'a>> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => vec![id.as_ref()],
        _ => binding_patterns::leaves(pattern).collect(),
    }
}

pub(super) fn pattern_symbols(semantic: &Semantic<'_>) -> BTreeSet<SymbolId> {
    locals(semantic)
        .into_iter()
        .filter_map(|(symbol, site, _)| {
            let AstKind::VariableDeclarator(local) = semantic.nodes().kind(site) else {
                unreachable!()
            };
            (!matches!(local.id, BindingPattern::BindingIdentifier(_))).then_some(symbol)
        })
        .collect()
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
            for id in names(&local.id) {
                if let Some(symbol) = id.symbol_id.get() {
                    found.push((symbol, local.node_id.get(), body));
                }
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
        .filter_map(|node| {
            let left = match node.kind() {
                AstKind::ForOfStatement(source) => &source.left,
                AstKind::ForInStatement(source) => &source.left,
                _ => return None,
            };
            let oxc_ast::ast::ForStatementLeft::VariableDeclaration(d) = left else {
                return None;
            };
            if d.kind != VariableDeclarationKind::Var || d.declare {
                return None;
            }
            Some(
                d.declarations
                    .iter()
                    .flat_map(|local| names(&local.id))
                    .filter_map(|id| id.symbol_id.get())
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
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
            for id in names(&local.id) {
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
            let selected = if matches!(d.id, BindingPattern::BindingIdentifier(_)) {
                vec![init.node_id()]
            } else {
                names(&d.id)
                    .into_iter()
                    .filter(|id| id.symbol_id.get() == Some(symbol))
                    .map(|id| id.node_id.get())
                    .collect()
            };
            writes.entry(symbol).or_default().extend(selected);
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

struct InitializedRegion {
    site: NodeId,
    after: u32,
    block: Option<oxc_span::Span>,
    sources: Vec<NodeId>,
}

fn lexical_store<'a>(
    write: NodeId,
    semantic: &Semantic<'a>,
) -> Option<&'a AssignmentExpression<'a>> {
    let AstKind::AssignmentExpression(assign) = semantic.nodes().parent_kind(write) else {
        return None;
    };
    (assign.operator == AssignmentOperator::Assign
        && matches!(assign.left.as_simple_assignment_target(),Some(SimpleAssignmentTarget::AssignmentTargetIdentifier(id)) if id.node_id()==write))
        .then_some(assign)
}

fn enclosing_statement_block(statement: NodeId, semantic: &Semantic<'_>) -> Option<oxc_span::Span> {
    match semantic.nodes().parent_kind(statement) {
        AstKind::BlockStatement(block) => Some(block.span),
        AstKind::FunctionBody(body) => Some(body.span),
        _ => None,
    }
}

fn single_store_region(write: NodeId, semantic: &Semantic<'_>) -> Option<InitializedRegion> {
    let assign = lexical_store(write, semantic)?;
    let site = assign.node_id();
    let statement = semantic.nodes().parent_id(site);
    let block = if straight(site, semantic, false) {
        None
    } else {
        if !matches!(
            semantic.nodes().kind(statement),
            AstKind::ExpressionStatement(_)
        ) {
            return None;
        }
        Some(enclosing_statement_block(statement, semantic)?)
    };
    Some(InitializedRegion {
        site,
        after: assign.span.end,
        block,
        sources: vec![assign.right.node_id()],
    })
}

// An arm supplies its value only when its final statement is the store.
// Earlier effects or abrupt completions still execute through normal lowering.
fn store_arm(write: NodeId, semantic: &Semantic<'_>) -> Option<(NodeId, NodeId, NodeId)> {
    let assign = lexical_store(write, semantic)?;
    let nodes = semantic.nodes();
    let statement = nodes.parent_id(assign.node_id());
    if !matches!(nodes.kind(statement), AstKind::ExpressionStatement(_)) {
        return None;
    }
    let arm = match nodes.parent_kind(statement) {
        AstKind::BlockStatement(block) if block.body.last()?.node_id() == statement => {
            block.node_id()
        }
        _ => statement,
    };
    let AstKind::IfStatement(choice) = nodes.parent_kind(arm) else {
        return None;
    };
    Some((choice.node_id(), arm, assign.right.node_id()))
}

fn exhaustive_store_region(
    first: NodeId,
    second: NodeId,
    semantic: &Semantic<'_>,
) -> Option<InitializedRegion> {
    let (site, left_arm, left_value) = store_arm(first, semantic)?;
    let (other, right_arm, right_value) = store_arm(second, semantic)?;
    if site != other || left_arm == right_arm {
        return None;
    }
    Some(InitializedRegion {
        site,
        after: semantic.nodes().kind(site).span().end,
        block: Some(enclosing_statement_block(site, semantic)?),
        sources: vec![left_value, right_value],
    })
}

/// Refine reads reached after the only write, or after both arms of an if/else store.
pub(super) fn lexical_assignment_reads(
    semantic: &Semantic<'_>,
    reads: &mut BTreeMap<NodeId, Vec<NodeId>>,
) {
    let nodes = semantic.nodes();
    let scope = semantic.scoping();
    for node in nodes.iter() {
        let AstKind::VariableDeclarator(decl) = node.kind() else {
            continue;
        };
        if !matches!(nodes.parent_kind(node.id()), AstKind::VariableDeclaration(d) if d.kind == VariableDeclarationKind::Let)
        {
            continue;
        }
        let BindingPattern::BindingIdentifier(name) = &decl.id else {
            continue;
        };
        let Some(symbol) = name.symbol_id.get() else {
            continue;
        };
        let mut writes = scope
            .get_resolved_references(symbol)
            .filter(|r| r.is_write());
        let Some(first) = writes.next() else { continue };
        let region = if let Some(second) = writes.next() {
            if writes.next().is_some() {
                continue;
            }
            exhaustive_store_region(first.node_id(), second.node_id(), semantic)
        } else {
            single_store_region(first.node_id(), semantic)
        };
        let Some(region) = region else { continue };
        let Some(body) = owner(node.id(), semantic) else {
            continue;
        };
        if nodes.kind(region.site).span().start < decl.span.end
            || owner(region.site, semantic) != Some(body)
        {
            continue;
        }
        for read in scope
            .get_resolved_references(symbol)
            .filter(|r| r.is_read())
        {
            let id = read.node_id();
            let span = nodes.kind(id).span();
            if span.start >= region.after
                && region.block.is_none_or(|b| b.contains_inclusive(span))
                && owner(id, semantic) == Some(body)
            {
                reads.insert(id, region.sources.clone());
            }
        }
    }
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
        let mut iterated = iteration_symbols(semantic);
        iterated.extend(pattern_symbols(semantic));
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
