//! Conservative allocation effects shared by source member-kind proofs.
use super::*;

pub(super) enum Mutation {
    KeyDelete { receiver: NodeId, keys: Vec<Key> },
    AnyKeyDelete(NodeId),
    IndexDelete(NodeId),
    Reorder(NodeId),
    AnyKeyWrite { receiver: NodeId, value: NodeId },
}

fn affected_names(key: iteration::PropertyKey<'_, '_>, names: &RecordNames) -> Option<Vec<Key>> {
    match key {
        iteration::PropertyKey::Literal(key) => Some(vec![key]),
        iteration::PropertyKey::Runtime(expr) => {
            if let Some(key) = closures::unwrapped(expr, 0).and_then(static_keys::literal) {
                return Some(vec![key]);
            }
            match names.domain(expr, 0) {
                record_names::Domain::Fields(fields) => Some(fields.into_iter().collect()),
                _ => None,
            }
        }
    }
}

pub(super) fn write(
    receiver: NodeId,
    key: iteration::PropertyKey<'_, '_>,
    value: NodeId,
    names: &RecordNames,
    writes: &mut Vec<(NodeId, Key, NodeId)>,
    mutations: &mut Vec<Mutation>,
) {
    if let Some(keys) = affected_names(key, names) {
        writes.extend(keys.into_iter().map(|key| (receiver, key, value)));
    } else {
        mutations.push(Mutation::AnyKeyWrite { receiver, value });
    }
}

pub(super) fn delete(
    unary: &oxc_ast::ast::UnaryExpression<'_>,
    uint32_keys: &Uint32Keys,
    names: &RecordNames,
) -> Option<Mutation> {
    if unary.operator != UnaryOperator::Delete {
        return None;
    }
    let (receiver, key) = match closures::unwrapped(&unary.argument, 0)? {
        Expression::StaticMemberExpression(member) => (
            &member.object,
            iteration::PropertyKey::Literal(member.property.name.encode_utf16().collect()),
        ),
        Expression::ComputedMemberExpression(member) => (
            &member.object,
            iteration::PropertyKey::Runtime(&member.expression),
        ),
        Expression::ChainExpression(chain) => {
            let (receiver, key) = evaluate::optional_member::parts(chain, 0, uint32_keys)?;
            (
                receiver,
                match key {
                    evaluate::optional_member::Key::Literal(key) => {
                        iteration::PropertyKey::Literal(key)
                    }
                    evaluate::optional_member::Key::Uint32(expr)
                    | evaluate::optional_member::Key::Runtime(expr) => {
                        iteration::PropertyKey::Runtime(expr)
                    }
                },
            )
        }
        _ => return None,
    };
    let receiver = receiver.node_id();
    Some(match affected_names(key, names) {
        Some(keys) => Mutation::KeyDelete { receiver, keys },
        None => Mutation::AnyKeyDelete(receiver),
    })
}

pub(super) fn call(call: &CallExpression<'_>) -> Option<Mutation> {
    let (operation, receiver) = array_mutations::operation(call)?;
    match operation {
        array_mutations::Operation::ToReversed => None,
        // Appends and prepends can change runtime-selected indexed elements;
        // conservatively invalidate stale element proofs through aliases.
        array_mutations::Operation::Push | array_mutations::Operation::Unshift => {
            Some(Mutation::AnyKeyDelete(receiver.node_id()))
        }
        array_mutations::Operation::Pop | array_mutations::Operation::Shift => {
            Some(Mutation::IndexDelete(receiver.node_id()))
        }
        array_mutations::Operation::Reverse => Some(Mutation::Reorder(receiver.node_id())),
    }
}

pub(super) fn array_index(key: &[u16]) -> Option<u32> {
    if key.is_empty() || key.len() > 10 || (key.len() > 1 && key[0] == u16::from(b'0')) {
        return None;
    }
    let mut value = 0_u64;
    for unit in key {
        let digit = unit
            .checked_sub(u16::from(b'0'))
            .filter(|digit| *digit < 10)?;
        value = value * 10 + u64::from(digit);
    }
    (value < u64::from(u32::MAX)).then_some(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    fn field_proofs(body: &str) -> (bool, Option<Type>, bool) {
        let source = format!("export function run(a,b){{{body}}}");
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &source, SourceType::mjs()).parse();
        assert!(parsed.diagnostics.is_empty());
        let built = SemanticBuilder::new_compiler()
            .with_build_nodes(true)
            .build(&parsed.program);
        assert!(built.diagnostics.is_empty());
        let entry = built
            .semantic
            .nodes()
            .iter()
            .find_map(|node| match node.kind() {
                AstKind::Function(function) => function
                    .id
                    .as_ref()
                    .filter(|name| name.name == "run")
                    .and_then(|name| name.symbol_id.get()),
                _ => None,
            })
            .unwrap();
        let read = built
            .semantic
            .nodes()
            .iter()
            .find_map(|node| match node.kind() {
                AstKind::StaticMemberExpression(member)
                    if member.property.name == "field" && !member.optional =>
                {
                    Some(node.id())
                }
                _ => None,
            })
            .unwrap();
        let fields = Fields::collect(&built.semantic, entry);
        (
            fields.string(read),
            fields.container(read),
            fields.functions(read).is_some(),
        )
    }

    #[test]
    fn dictionary_mutation_facts_widen_aliases_and_call_arguments() {
        // Inspect the source analysis independently of admission: ordinary
        // objects still reject dynamic writes, but no consumer of these facts
        // may infer a field kind after such a write has been observed.
        for (initial, expected) in [
            ("'safe'", (true, None, false)),
            ("[]", (false, Some(Type::ManagedArray), false)),
            ("{}", (false, Some(Type::ManagedObject), false)),
            ("()=>a", (false, None, true)),
        ] {
            let prefix = format!("const holder={{field:{initial}}};const alias=holder;");
            assert_eq!(
                field_proofs(&format!("{prefix}const observed=holder.field;return a;")),
                expected,
            );
            assert_eq!(
                field_proofs(&format!(
                    "{prefix}const key='field';alias[key]=a;const observed=holder.field;return a;"
                )),
                (false, None, false),
            );
        }
        assert_eq!(
            field_proofs(
                "const holder={field:'safe'};const mutate=(target,key,value)=>target[key]=value;mutate(holder,'field',a);const observed=holder.field;return a;"
            ),
            (false, None, false),
        );
        assert_eq!(
            field_proofs(
                "const holder={field:'safe'};const key='field';holder[key]='new';const observed=holder.field;return a;"
            ),
            (true, None, false),
        );
    }

    #[test]
    fn optional_deletion_widens_field_proofs_through_aliases_and_helpers() {
        for initial in ["'safe'", "[]", "{}", "()=>a"] {
            assert_eq!(
                field_proofs(&format!(
                    "const holder={{field:{initial}}};const alias=holder;delete alias?.field;const observed=holder.field;return a;"
                )),
                (false, None, false),
            );
            assert_eq!(
                field_proofs(&format!(
                    "const holder={{field:{initial}}};const remove=target=>delete target?.field;remove(holder);const observed=holder.field;return a;"
                )),
                (false, None, false),
            );
        }
    }
}
