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
        if call.arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "at most 1000 array search arguments are admitted",
            ));
        }
        let from = match call.arguments.get(1).and_then(|arg| arg.as_expression()) {
            None => Start::Omitted,
            Some(arg) if crate::lower::string_methods::undefined(arg, self.semantic) => {
                Start::Undefined
            }
            Some(_) => Start::Number,
        };
        work.push(Work::ArraySearch {
            node,
            kind,
            has_needle: !call.arguments.is_empty(),
            from,
        });
        for (index, arg) in call.arguments.iter().enumerate().rev() {
            let arg = arg
                .as_expression()
                .ok_or_else(|| unsupported(node, "spread call arguments are not admitted"))?;
            let mode = match index {
                0 => Mode::Managed,
                1 if matches!(from, Start::Number) => Mode::Number,
                _ => Mode::Discard,
            };
            work.push(Work::Eval(arg, depth + 1, mode));
        }
        work.push(Work::Eval(receiver, depth + 1, Mode::Array));
        Ok(())
    }
}
