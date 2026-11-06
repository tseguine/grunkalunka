//! Preserve the receiver and needle while the remaining arguments evaluate.
use super::*;

impl Lowering<'_, '_> {
    pub(super) fn schedule_array_includes<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        receiver: &'e Expression<'a>,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if call.arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "at most 1000 array includes arguments are admitted",
            ));
        }
        let from = call
            .arguments
            .get(1)
            .and_then(|arg| arg.as_expression())
            .filter(|arg| !crate::lower::string_methods::undefined(arg, self.semantic));
        work.push(Work::ArrayIncludes {
            node,
            has_needle: !call.arguments.is_empty(),
            has_from: from.is_some(),
        });
        for (index, arg) in call.arguments.iter().enumerate().rev() {
            let arg = arg
                .as_expression()
                .ok_or_else(|| unsupported(node, "spread call arguments are not admitted"))?;
            let mode = match index {
                0 => Mode::Managed,
                1 if from.is_some() => Mode::Number,
                _ => Mode::Discard,
            };
            work.push(Work::Eval(arg, depth + 1, mode));
        }
        work.push(Work::Eval(receiver, depth + 1, Mode::Array));
        Ok(())
    }
}
