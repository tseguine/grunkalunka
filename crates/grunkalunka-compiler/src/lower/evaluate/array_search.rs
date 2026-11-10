//! Preserve the receiver and needle while the remaining arguments evaluate.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Start {
    Omitted,
    Undefined,
    Number,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_array_search<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        receiver: &'e Expression<'a>,
        kind: ir::ArraySearch,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let arguments = invocation::plain_arguments(&call.arguments);
        if arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "at most 1000 array search arguments are admitted",
            ));
        }
        let from = match arguments.get(1).and_then(|arg| arg.as_expression()) {
            None => Start::Omitted,
            Some(arg) if crate::lower::string_methods::undefined(arg, self.semantic) => {
                Start::Undefined
            }
            Some(_) => Start::Number,
        };
        work.push(Work::ArraySearch {
            node,
            kind,
            has_needle: arguments.first().is_some(),
            from,
        });
        for (index, arg) in arguments.into_iter().enumerate().rev() {
            let arg_depth = depth + 1 + arg.depth;
            let arg = arg
                .as_expression()
                .ok_or_else(|| unsupported(node, "spread call arguments are not admitted"))?;
            let mode = match index {
                0 => Mode::Managed,
                1 if matches!(from, Start::Number) => Mode::Number,
                _ => Mode::Discard,
            };
            work.push(Work::Eval(arg, arg_depth, mode));
        }
        work.push(Work::Eval(receiver, depth + 1, Mode::Array));
        Ok(())
    }
}
