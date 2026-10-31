//! Array construction resumes each element on the shared expression cursor.
use super::*;
use oxc_ast::ast::{ArrayExpression, ArrayExpressionElement};

pub(super) fn literal_length(mut node: &Expression<'_>, mut depth: usize) -> Result<u32, String> {
    loop {
        if depth > 128 {
            return Err(unsupported(
                node,
                "expression nesting exceeds this experiment's limit",
            ));
        }
        if let Some(inner) = erased_inner(node) {
            node = inner;
            depth += 1;
            continue;
        }
        if let Expression::ParenthesizedExpression(w) = node {
            node = &w.expression;
            depth += 1;
            continue;
        }
        if let Expression::NumericLiteral(n) = node
            && n.value >= 0.0
            && n.value <= f64::from(u32::MAX)
            && n.value.fract() == 0.0
        {
            return Ok(n.value as u32);
        }
        return Err(unsupported(
            node,
            "array length writes require a valid unsigned integer literal",
        ));
    }
}

pub(in crate::lower::evaluate) struct ArrayWork<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub literal: &'e ArrayExpression<'a>,
    pub array: ValueId,
    pub index: usize,
    pub depth: usize,
    pub awaiting: bool,
}

impl Lowering<'_, '_> {
    pub(super) fn complete_array<'e, 'a>(
        &mut self,
        mut task: ArrayWork<'e, 'a>,
        regions: &mut Regions<'_>,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.awaiting {
            let value = values.pop().expect("array element value").id();
            Self::push(
                regions.current(),
                task.node,
                Op::ManagedArraySetOwn {
                    array: task.array,
                    key: (task.index - 1).to_string().encode_utf16().collect(),
                    value,
                },
            );
        }
        while let Some(element) = task.literal.elements.get(task.index) {
            if matches!(element, ArrayExpressionElement::Elision(_)) {
                task.index += 1;
                continue;
            }
            let Some(expression) = element.as_expression() else {
                return Err(unsupported(task.node, "array spread is not yet admitted"));
            };
            task.index += 1;
            task.awaiting = true;
            let depth = task.depth + 1;
            work.push(Work::Managed(ManagedWork::Array(task)));
            work.push(Work::Eval(expression, depth, Mode::Managed));
            return Ok(());
        }
        values.push(Value::Array(task.array));
        Ok(())
    }
}
