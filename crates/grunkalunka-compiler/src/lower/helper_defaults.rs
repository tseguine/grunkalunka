//! Primitive constant defaults use the same lazy Undefined selection as patterns.
use super::*;
use oxc_ast::{AstKind, ast::Function};
use std::collections::BTreeSet;

enum Constant {
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
    String(Vec<u16>),
}
fn constant(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Option<Constant> {
    let mut node = expression;
    let mut depth = 0;
    let mut sign = 1.0;
    let mut numeric = false;
    loop {
        let (inner, next) = closures::unwrapped_at(node, depth)?;
        node = inner;
        depth = next;
        if depth > 128 {
            return None;
        }
        match node {
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation
                ) =>
            {
                numeric = true;
                if unary.operator == UnaryOperator::UnaryNegation {
                    sign = -sign;
                }
                node = &unary.argument;
                depth += 1;
            }
            Expression::NumericLiteral(number) => {
                return Some(Constant::Number(sign * number.value));
            }
            Expression::BooleanLiteral(boolean) if !numeric => {
                return Some(Constant::Boolean(boolean.value));
            }
            Expression::NullLiteral(_) if !numeric => return Some(Constant::Null),
            Expression::StringLiteral(string) if !numeric => {
                return Some(Constant::String(string.value.encode_utf16().collect()));
            }
            Expression::Identifier(id)
                if !numeric
                    && id.name == "undefined"
                    && id.reference_id.get().is_some_and(|reference| {
                        semantic
                            .scoping()
                            .get_reference(reference)
                            .symbol_id()
                            .is_none()
                    }) =>
            {
                return Some(Constant::Undefined);
            }
            _ => return None,
        }
    }
}

pub(crate) fn validate(
    function: &Function<'_>,
    semantic: &Semantic<'_>,
    allowed: bool,
) -> Result<(), String> {
    for parameter in &function.params.items {
        if let Some(initializer) = &parameter.initializer {
            let initializer = initializer.as_ref();
            if !allowed {
                return Err(unsupported(
                    parameter,
                    "parameter modifiers, defaults and decorators are not admitted",
                ));
            }
            if constant(initializer, semantic).is_none() {
                return Err(unsupported(
                    initializer,
                    "helper defaults require primitive constants",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn present(semantic: &Semantic<'_>) -> bool {
    semantic.nodes().iter().any(|node| {
        matches!(node.kind(), AstKind::Function(f)
            if f.params.items.iter().any(|p| p.initializer.is_some()))
    })
}

impl Lowering<'_, '_> {
    pub(crate) fn initialize_helper_defaults(
        &mut self,
        function: &Function<'_>,
        output: &mut Region,
    ) -> Result<(), String> {
        for (index, parameter) in function.params.items.iter().enumerate() {
            let Some(initializer) = &parameter.initializer else {
                continue;
            };
            let initializer = initializer.as_ref();
            let slot = SlotId(index);
            if self.slots[slot.0].ty != Type::ManagedValue {
                return Err(unsupported(
                    parameter,
                    "default parameter requires managed storage",
                ));
            }
            let selected = self.value(Type::ManagedValue);
            Self::push(
                output,
                parameter,
                Op::Load {
                    result: selected,
                    slot,
                },
            );
            let undefined = self.binding_undefined(parameter, output);
            let pending = self.prepare_binding_default(parameter, output, selected, undefined);
            let mut consequent = self.region(initializer);
            let value = match constant(initializer, self.semantic).ok_or_else(|| {
                unsupported(initializer, "helper defaults require primitive constants")
            })? {
                Constant::String(units) => {
                    let value = self.value(Type::ManagedValue);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedString {
                            result: value,
                            units,
                        },
                    );
                    value
                }
                constant => {
                    let primitive = self.value(Type::General);
                    match constant {
                        Constant::Number(value) => {
                            let scalar = self.value(Type::Number);
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Constant {
                                    result: scalar,
                                    value,
                                },
                            );
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Pack {
                                    result: primitive,
                                    input: scalar,
                                },
                            );
                        }
                        Constant::Boolean(value) => {
                            let scalar = self.value(Type::Predicate);
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Boolean {
                                    result: scalar,
                                    value,
                                },
                            );
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Pack {
                                    result: primitive,
                                    input: scalar,
                                },
                            );
                        }
                        Constant::Null | Constant::Undefined => Self::push(
                            &mut consequent,
                            initializer,
                            Op::Nullish {
                                result: primitive,
                                value: if matches!(constant, Constant::Null) {
                                    ir::Nullish::Null
                                } else {
                                    ir::Nullish::Undefined
                                },
                            },
                        ),
                        Constant::String(_) => unreachable!(),
                    }
                    let value = self.value(Type::ManagedValue);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedPrimitive {
                            result: value,
                            input: primitive,
                        },
                    );
                    value
                }
            };
            let value = self.finish_binding_default(parameter, output, pending, consequent, value);
            Self::push(output, parameter, Op::Store { slot, value });
        }
        Ok(())
    }

    // Reflection counts parameters before the first default, independently of
    // the full callable ABI. Mixed lengths need a future dynamic reflection path.
    pub(super) fn default_function_length(
        &self,
        object: &Expression<'_>,
        fallback: usize,
    ) -> Result<usize, String> {
        if !present(self.semantic) {
            return Ok(fallback);
        }
        let (names, arrows) = self.stored.origins(object.node_id());
        let mut lengths = BTreeSet::new();
        for node in self.semantic.nodes().iter() {
            match node.kind() {
                AstKind::Function(f)
                    if f.id
                        .as_ref()
                        .and_then(|id| id.symbol_id.get())
                        .is_some_and(|id| names.contains(&id)) =>
                {
                    lengths.insert(
                        f.params
                            .items
                            .iter()
                            .take_while(|p| p.initializer.is_none())
                            .count(),
                    );
                }
                AstKind::ArrowFunctionExpression(a) if arrows.contains(&node.id()) => {
                    lengths.insert(a.params.items.len());
                }
                _ => (),
            }
        }
        if lengths.len() == 1 {
            Ok(*lengths.first().unwrap())
        } else {
            Err(unsupported(
                object,
                "function length requires one proven default-parameter length",
            ))
        }
    }
}
