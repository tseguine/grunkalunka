//! Iterative template evaluation using cooked UTF-16 fragments and primitive substitutions.
use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub template: &'e oxc_ast::ast::TemplateLiteral<'a>,
    pub result: Option<ValueId>,
    pub index: usize,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_string_template<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if let Some(left) = task.result {
            let right = values.pop().expect("template substitution").id();
            let result = self.value(Type::ManagedValue);
            Self::push(
                regions.current(),
                task.node,
                Op::ManagedStringConcat {
                    result,
                    left,
                    right,
                },
            );
            task.result = Some(result);
        }
        let cooked = task.template.quasis[task.index]
            .value
            .cooked
            .as_ref()
            .ok_or_else(|| unsupported(task.node, "template requires valid cooked text"))?;
        let right = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedString {
                result: right,
                units: cooked.encode_utf16().collect(),
            },
        );
        let result = if let Some(left) = task.result {
            let result = self.value(Type::ManagedValue);
            Self::push(
                regions.current(),
                task.node,
                Op::ManagedStringConcat {
                    result,
                    left,
                    right,
                },
            );
            result
        } else {
            right
        };
        if let Some(expression) = task.template.expressions.get(task.index) {
            task.result = Some(result);
            task.index += 1;
            let depth = task.depth + 1;
            work.push(Work::StringTemplate(task));
            self.schedule_string_operand(expression, depth, work)?;
        } else {
            values.push(Value::Managed(result));
        }
        Ok(())
    }
}
