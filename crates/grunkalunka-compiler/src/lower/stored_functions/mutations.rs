//! Conservative allocation effects shared by source member-kind proofs.
use super::*;

pub(super) enum Mutation {
    KeyDelete { receiver: NodeId, key: Key },
    AnyKeyDelete(NodeId),
    IndexDelete(NodeId),
    Reorder(NodeId),
    AnyKeyWrite { receiver: NodeId, value: NodeId },
}

pub(super) fn delete(unary: &oxc_ast::ast::UnaryExpression<'_>) -> Option<Mutation> {
    if unary.operator != UnaryOperator::Delete {
        return None;
    }
    let (receiver, key) = match closures::unwrapped(&unary.argument, 0)? {
        Expression::StaticMemberExpression(member) => (
            &member.object,
            Some(member.property.name.encode_utf16().collect()),
        ),
        Expression::ComputedMemberExpression(member) => (
            &member.object,
            closures::unwrapped(&member.expression, 0).and_then(static_keys::literal),
        ),
        Expression::ChainExpression(chain) => {
            let (receiver, key) = evaluate::optional_member::parts(chain, 0)?;
            let key = match key {
                evaluate::optional_member::Key::Literal(key) => Some(key),
                evaluate::optional_member::Key::Uint32(_)
                | evaluate::optional_member::Key::Runtime(_) => None,
            };
            (receiver, key)
        }
        _ => return None,
    };
    Some(match key {
        Some(key) => Mutation::KeyDelete {
            receiver: receiver.node_id(),
            key,
        },
        None => Mutation::AnyKeyDelete(receiver.node_id()),
    })
}

pub(super) fn call(call: &CallExpression<'_>) -> Option<Mutation> {
    let (operation, receiver) = array_mutations::operation(call)?;
    match operation {
        array_mutations::Operation::ToReversed => None,
        // A push writes a runtime-selected index; invalidate stale element
        // proofs conservatively when it may be observed through aliases.
        array_mutations::Operation::Push => Some(Mutation::AnyKeyDelete(receiver.node_id())),
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
