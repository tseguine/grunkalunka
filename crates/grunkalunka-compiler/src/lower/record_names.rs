//! String names proven to miss the pristine Object prototype.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct RecordNames {
    reads: BTreeMap<NodeId, SymbolId>,
    nullable: BTreeSet<SymbolId>,
    global_undefined: BTreeSet<NodeId>,
    sources: BTreeMap<SymbolId, NameSources>,
}

// Limit only the extra type precision: larger sets keep the generic own read.
const MAX_CHOICES: usize = 32;

pub(super) enum Domain {
    Unknown,
    AnyOwn,
    Fields(BTreeSet<Vec<u16>>),
}

impl Domain {
    pub(super) fn string_own_names(&self) -> bool {
        matches!(self, Self::Fields(names) if names.iter().all(|name|
            name.iter().copied().eq("length".encode_utf16()) || binding_patterns::string_index(name).is_some()))
    }

    pub(super) fn array_own_names(&self) -> bool {
        matches!(self, Self::Fields(names) if names.iter().all(|name|
            !crate::prototype_keys::contains(crate::prototype_keys::ARRAY, name)))
    }
}

// Retain the values which can reach a property read through logical choices.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Selected {
    empty: bool,
    text: bool,
    nullish: bool,
}

impl Selected {
    fn includes(self, name: &[u16]) -> bool {
        if name.is_empty() {
            self.empty
        } else {
            self.text
        }
    }

    fn logical_left(self, operator: LogicalOperator) -> Self {
        match operator {
            LogicalOperator::And => Selected {
                text: false,
                ..self
            },
            LogicalOperator::Or => Selected {
                empty: false,
                nullish: false,
                ..self
            },
            LogicalOperator::Coalesce => Selected {
                nullish: false,
                ..self
            },
        }
    }

    fn intersect(self, other: Self) -> Self {
        Self {
            empty: self.empty && other.empty,
            text: self.text && other.text,
            nullish: self.nullish && other.nullish,
        }
    }
}

struct NameSources {
    literals: Option<BTreeSet<Vec<u16>>>,
    dependencies: BTreeSet<(SymbolId, Selected)>,
}

fn add_literal(names: &mut Option<BTreeSet<Vec<u16>>>, literal: Option<Vec<u16>>) {
    if let Some(set) = names {
        if let Some(literal) = literal {
            set.insert(literal);
            if set.len() <= MAX_CHOICES {
                return;
            }
        }
        *names = None;
    }
}

impl RecordNames {
    fn selected_symbol(&self, id: NodeId, selected: Selected) -> Option<SymbolId> {
        self.reads
            .get(&id)
            .copied()
            .filter(|symbol| !selected.nullish || !self.nullable.contains(symbol))
    }

    pub(super) fn contains(&self, expression: &Expression<'_>, depth: usize) -> bool {
        name_from(
            expression,
            depth,
            &self.global_undefined,
            |id, selected| self.selected_symbol(id.node_id(), selected).is_some(),
            || false,
            |_| {},
        )
    }

    pub(super) fn domain(&self, expression: &Expression<'_>, depth: usize) -> Domain {
        let mut names = Some(BTreeSet::new());
        let mut pending = Vec::new();
        if !name_from(
            expression,
            depth,
            &self.global_undefined,
            |id, selected| {
                if let Some(symbol) = self.selected_symbol(id.node_id(), selected) {
                    pending.push((symbol, selected));
                    true
                } else {
                    false
                }
            },
            || false,
            |literal| add_literal(&mut names, literal),
        ) {
            return Domain::Unknown;
        }
        let Some(mut names) = names else {
            return Domain::AnyOwn;
        };
        let mut visited = BTreeSet::new();
        while let Some((symbol, selected)) = pending.pop() {
            if (!selected.empty && !selected.text) || !visited.insert((symbol, selected)) {
                continue;
            }
            let source = &self.sources[&symbol];
            let Some(literals) = &source.literals else {
                return Domain::AnyOwn;
            };
            names.extend(
                literals
                    .iter()
                    .filter(|name| selected.includes(name))
                    .cloned(),
            );
            if names.len() > MAX_CHOICES
                || visited.len() > MAX_CHOICES
                || source.dependencies.len() > MAX_CHOICES
            {
                return Domain::AnyOwn;
            }
            pending.extend(
                source
                    .dependencies
                    .iter()
                    .map(|(dep, edge)| (*dep, selected.intersect(*edge))),
            );
        }
        if names.is_empty() {
            Domain::AnyOwn
        } else {
            Domain::Fields(names)
        }
    }

    pub(super) fn collect(semantic: &Semantic<'_>) -> Self {
        let scope = semantic.scoping();
        let global_undefined = scope
            .root_unresolved_references()
            .get("undefined")
            .into_iter()
            .flat_map(|ids| ids.iter())
            .map(|id| scope.get_reference(*id).node_id())
            .collect();
        let nodes = semantic.nodes();
        let mut sources = BTreeMap::new();
        let mut candidates = BTreeSet::new();
        let mut nullable = BTreeSet::new();
        let mut users: BTreeMap<SymbolId, Vec<(SymbolId, Selected)>> = BTreeMap::new();
        for node in nodes.iter() {
            let AstKind::VariableDeclarator(decl) = node.kind() else {
                continue;
            };
            let AstKind::VariableDeclaration(parent) = nodes.parent_kind(node.id()) else {
                continue;
            };
            let BindingPattern::BindingIdentifier(binding) = &decl.id else {
                continue;
            };
            let (Some(init), Some(symbol)) = (decl.init.as_ref(), binding.symbol_id.get()) else {
                continue;
            };
            if !matches!(
                parent.kind,
                VariableDeclarationKind::Let | VariableDeclarationKind::Const
            ) {
                continue;
            }

            let mut dependencies = BTreeSet::new();
            let mut read = |id: &IdentifierReference<'_>, selected| {
                let Some(dep) = id
                    .reference_id
                    .get()
                    .and_then(|r| scope.get_reference(r).symbol_id())
                else {
                    return false;
                };
                if semantic.symbol_declaration(dep).kind().span().end > id.span.start {
                    return false;
                }
                dependencies.insert((dep, selected));
                true
            };
            let mut literals = Some(BTreeSet::new());
            let mut may_be_nullish = false;
            let mut observe_nullish = || {
                may_be_nullish = true;
                true
            };
            let mut observe = |name| add_literal(&mut literals, name);
            if !name_from(
                init,
                0,
                &global_undefined,
                &mut read,
                &mut observe_nullish,
                &mut observe,
            ) {
                continue;
            }
            // A plain or logical assignment either keeps the old value or
            // stores its original RHS. Other writes invalidate the proof.
            let safe_writes = scope.get_resolved_references(symbol).filter(|r| r.is_write()).all(|r| {
                let AstKind::AssignmentExpression(assign) = nodes.parent_kind(r.node_id()) else { return false };
                (assign.operator == AssignmentOperator::Assign || assign.operator.is_logical())
                    && matches!(assign.left.as_simple_assignment_target(),Some(SimpleAssignmentTarget::AssignmentTargetIdentifier(id)) if id.node_id()==r.node_id())
                    && name_from(&assign.right, 0, &global_undefined, &mut read, &mut observe_nullish, &mut observe)
            });
            if !safe_writes {
                continue;
            }
            candidates.insert(symbol);
            if may_be_nullish {
                nullable.insert(symbol);
            }
            for &(dep, selected) in &dependencies {
                users.entry(dep).or_default().push((symbol, selected));
            }
            sources.insert(
                symbol,
                NameSources {
                    literals,
                    dependencies,
                },
            );
        }
        // Invalidate dependent bindings via reverse edges. Cycles of safe
        // copies retain their invariant because each initializer was checked.
        let mut invalid = users
            .keys()
            .filter(|id| !candidates.contains(id))
            .copied()
            .collect::<Vec<_>>();
        while let Some(dep) = invalid.pop() {
            for &(user, _) in users.get(&dep).into_iter().flatten() {
                if candidates.remove(&user) {
                    invalid.push(user);
                }
            }
        }
        // Nullish values reach a dependent binding only along edges whose
        // logical operator can select them. Propagate across every write.
        let mut pending: Vec<_> = nullable.iter().copied().collect();
        while let Some(dep) = pending.pop() {
            for &(user, selected) in users.get(&dep).into_iter().flatten() {
                if selected.nullish && nullable.insert(user) {
                    pending.push(user);
                }
            }
        }
        let mut reads = BTreeMap::new();
        for symbol in candidates {
            let end = semantic.symbol_declaration(symbol).kind().span().end;
            reads.extend(
                scope
                    .get_resolved_references(symbol)
                    .filter(|r| r.is_read() && nodes.kind(r.node_id()).span().start >= end)
                    .map(|r| (r.node_id(), symbol)),
            );
        }
        Self {
            reads,
            global_undefined,
            sources,
            nullable,
        }
    }
}

// Keep the prototype names consistent with each fixed fragment's position.
// Unknown operands may contribute any String. A literal below an operator
// other than concatenation provides no evidence about the resulting name.
fn constructed_own_name(expression: &Expression<'_>, depth: usize) -> bool {
    let mut candidates = crate::prototype_keys::OBJECT.to_vec();
    let mut pending = vec![(expression, depth, true, true)];
    while let Some((node, depth, first, last)) = pending.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        let mut fragment = |text: Option<&str>, prefix: bool, suffix: bool| {
            candidates.retain(|name| {
                text.is_some_and(|text| match (prefix, suffix) {
                    (true, true) => *name == text,
                    (true, false) => name.starts_with(text),
                    (false, true) => name.ends_with(text),
                    (false, false) => name.contains(text),
                })
            });
        };
        match node {
            Expression::StringLiteral(s) => fragment(s.value.as_str(), first, last),
            Expression::TemplateLiteral(t) => {
                for (i, quasi) in t.quasis.iter().enumerate() {
                    let Some(cooked) = &quasi.value.cooked else {
                        return false;
                    };
                    fragment(
                        cooked.as_str(),
                        first && i == 0,
                        last && i + 1 == t.quasis.len(),
                    );
                }
            }
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                pending.push((&b.right, depth + 1, false, last));
                pending.push((&b.left, depth + 1, first, false));
            }
            _ => {}
        }
        if candidates.is_empty() {
            return true;
        }
    }
    false
}

// A plain record can use its own-property table when every possible String
// key misses the pristine Object prototype. Validate operands separately;
// this walk only describes the result of a successful key evaluation.
fn name_from<'a>(
    expression: &Expression<'a>,
    depth: usize,
    global_undefined: &BTreeSet<NodeId>,
    mut valid_read: impl FnMut(&IdentifierReference<'a>, Selected) -> bool,
    mut accept_nullish: impl FnMut() -> bool,
    mut observe: impl FnMut(Option<Vec<u16>>),
) -> bool {
    let all = Selected {
        empty: true,
        text: true,
        nullish: true,
    };
    let mut pending = vec![(expression, depth, all)];
    while let Some((node, depth, selected)) = pending.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        match node {
            Expression::Identifier(id) if global_undefined.contains(&id.node_id()) => {
                if selected.nullish && !accept_nullish() {
                    return false;
                }
            }
            Expression::NullLiteral(_) if !selected.nullish || accept_nullish() => {}
            Expression::UnaryExpression(u)
                if u.operator == UnaryOperator::Void && (!selected.nullish || accept_nullish()) => {
            }
            Expression::StringLiteral(_) => {
                let Some(name) = static_keys::literal(node) else {
                    return false;
                };
                if !selected.includes(&name) {
                    continue;
                }
                if crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &name) {
                    return false;
                }
                observe(Some(name));
            }
            Expression::TemplateLiteral(_) | Expression::BinaryExpression(_)
                if constructed_own_name(node, depth) =>
            {
                observe(static_keys::literal(node));
            }
            Expression::LogicalExpression(logical) => {
                pending.push((
                    &logical.left,
                    depth + 1,
                    selected.logical_left(logical.operator),
                ));
                pending.push((&logical.right, depth + 1, selected));
            }
            Expression::AssignmentExpression(assign) => {
                let Some(SimpleAssignmentTarget::AssignmentTargetIdentifier(id)) =
                    assign.left.as_simple_assignment_target()
                else {
                    return false;
                };
                if let Some(op) = assign.operator.to_logical_operator() {
                    if !valid_read(id, selected.logical_left(op)) {
                        return false;
                    }
                } else if assign.operator != AssignmentOperator::Assign {
                    return false;
                }
                pending.push((&assign.right, depth + 1, selected));
            }
            Expression::ConditionalExpression(c) => {
                pending.push((&c.consequent, depth + 1, selected));
                pending.push((&c.alternate, depth + 1, selected));
            }
            Expression::SequenceExpression(seq) => {
                let Some(last) = seq.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1, selected));
            }
            Expression::Identifier(id) if valid_read(id, selected) => {}
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn finite_field_selection_preserves_misses_and_writes() {
        for body in [
            "const o={x:v=>v};const k=a?'x':'missing';return o[k](b);",
            "const o={f:x=>x,y:2},k=a?'y':'f';o[k]=b;return o.f(a);",
            "const o={f:x=>x,y:2},k=a?'y':'f';delete o[k];return o.f(a);",
            "const o={f:x=>x};let k='f';const alias=o;alias[k]=0;return o.f(a);",
            "const o={x:v=>v,y:3};const k=a?'x':'y';return o[k](b);",
            "const o={x:v=>v};const k='x';const alias=o;alias.x=3;return o[k](b);",
            "const o={x:'yes'};const k='x';delete o.x;return o[k].length;",
            "const o={x:v=>v,y:(v,w)=>v+w};const k=a?'x':'y';return o[k](b);",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
    }

    #[test]
    fn every_initializer_and_write_must_preserve_record_names() {
        for (bindings, admitted) in [
            ("let k='x';if(a)k='y';", true),
            ("let k='x',j=k;k=j;j=k;", true),
            ("let k='x';const change=()=>k='y';change();", true),
            ("let k='x';k='constructor';", false),
            ("let k='x',j=k;k=j;j='constructor';", false),
            ("let k='x';const change=()=>k='__proto__';", false),
            ("let k='x';[k]=['toString'];", false),
            ("let k='x';for(k of ['toString']){}", false),
            ("let k='x';for(k in {constructor:1}){}", false),
            ("let k='to';k+='String';", false),
            ("let k={} as 'x';", false),
        ] {
            let source = format!(
                "export function run(a,b){{{bindings}const o={{x:b}};return o[k]===b?1:0;}}"
            );
            let result = crate::compile(&source, true);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
        }
    }
}
