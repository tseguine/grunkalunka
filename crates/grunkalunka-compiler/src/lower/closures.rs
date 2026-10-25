//! Local, non-escaping closures are specialized into their caller. Captures name
//! existing slots, so copying this metadata never snapshots captured values.
use super::*;
use oxc_ast::{AstKind, ast::ArrowFunctionExpression};
use oxc_syntax::node::NodeId;

#[derive(Clone)]
pub(crate) struct LocalClosure {
    pub symbol: Option<SymbolId>,
    pub arrow: NodeId,
    pub captures: Vec<NumericBinding>,
}

pub(super) fn unwrapped<'e, 'a>(
    mut expression: &'e Expression<'a>,
    mut depth: usize,
) -> Option<&'e Expression<'a>> {
    while depth <= 128 {
        if let Some(inner) = erased_inner(expression) {
            expression = inner;
        } else if let Expression::ParenthesizedExpression(wrapper) = expression {
            expression = &wrapper.expression;
        } else {
            return Some(expression);
        }
        depth += 1;
    }
    None
}

impl<'s, 'a> Lowering<'s, 'a> {
    pub(super) fn arrow(&self, id: NodeId) -> &'a ArrowFunctionExpression<'a> {
        let AstKind::ArrowFunctionExpression(arrow) = self.semantic.nodes().kind(id) else {
            unreachable!("closure metadata comes from the original arrow AST");
        };
        arrow
    }

    pub(super) fn closure_target(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<LocalClosure> {
        match unwrapped(expression, depth)? {
            Expression::ArrowFunctionExpression(arrow) => Some(LocalClosure {
                symbol: None,
                arrow: arrow.node_id.get(),
                captures: self.bindings.clone(),
            }),
            Expression::Identifier(identifier) => {
                let symbol = self
                    .semantic
                    .scoping()
                    .get_reference(identifier.reference_id.get()?)
                    .symbol_id()?;
                self.closures
                    .iter()
                    .find(|closure| closure.symbol == Some(symbol))
                    .cloned()
            }
            _ => None,
        }
    }

    pub(super) fn check_arrow(&self, arrow: &ArrowFunctionExpression<'_>) -> Result<(), String> {
        if arrow.r#async || arrow.get_expression().is_none() {
            return Err(unsupported(
                arrow,
                "only synchronous expression-bodied local arrows are admitted",
            ));
        }
        if arrow.params.rest.is_some() || arrow.params.items.len() > 1000 {
            return Err(unsupported(
                arrow,
                "local arrows require at most 1000 ordinary numeric parameters",
            ));
        }
        for parameter in &arrow.params.items {
            if !matches!(parameter.pattern, BindingPattern::BindingIdentifier(_))
                || parameter.initializer.is_some()
                || parameter.optional
                || !parameter.decorators.is_empty()
                || parameter.accessibility.is_some()
                || parameter.readonly
                || parameter.r#override
            {
                return Err(unsupported(
                    parameter,
                    "local arrow parameters cannot have defaults, destructuring or modifiers",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn bind_arrow_parameters(
        &mut self,
        arrow: &ArrowFunctionExpression<'_>,
        arguments: Vec<ValueId>,
        region: &mut Region,
    ) -> Result<(), String> {
        for (parameter, mut value) in arrow.params.items.iter().zip(arguments) {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                unreachable!("checked arrow parameter");
            };
            let symbol = identifier
                .symbol_id
                .get()
                .ok_or("missing arrow parameter binding")?;
            let ty = if self.general_bindings.contains(&symbol) {
                Type::General
            } else {
                Type::Number
            };
            if ty == Type::General {
                let packed = self.value(ty);
                Self::push(
                    region,
                    parameter,
                    Op::Pack {
                        result: packed,
                        input: value,
                    },
                );
                value = packed;
            }
            let slot = SlotId(self.slots.len());
            self.slots.push(Slot {
                ty,
                owner: region.id,
                mutable: true,
                parameter: false,
                span: parameter.span(),
            });
            Self::push(region, parameter, Op::Initialize { slot, value });
            self.bindings.push(NumericBinding {
                symbol,
                slot,
                mutable: true,
            });
        }
        Ok(())
    }

    pub(super) fn check_local_closure(&self, closure: &LocalClosure) -> Result<(), String> {
        let arrow = self.arrow(closure.arrow);
        self.check_arrow(arrow)?;
        // Validate unused bodies too, without running effects or publishing their
        // scratch values/slots in the real function's verified IR.
        let mut check = self.clone();
        check.bindings = closure.captures.clone();
        let mut region = check.region(arrow);
        let mut arguments = Vec::new();
        for parameter in &arrow.params.items {
            let value = check.value(Type::Number);
            Self::push(
                &mut region,
                parameter,
                Op::Constant {
                    result: value,
                    value: 0.0,
                },
            );
            arguments.push(value);
        }
        check.bind_arrow_parameters(arrow, arguments, &mut region)?;
        check.evaluate(
            arrow.get_expression().expect("checked concise body"),
            &mut region,
            0,
            evaluate::Mode::Number,
        )?;
        Ok(())
    }
}
