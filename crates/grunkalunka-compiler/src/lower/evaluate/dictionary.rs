//! Runtime dictionary keys are snapshots on the ordinary expression cursor.
use super::*;

pub(super) enum Task<'e, 'a> {
    ReadReceiver {
        node: &'e Expression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
    },
    ReadKey {
        node: &'e Expression<'a>,
        object: ValueId,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_dictionary_read<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::ComputedMemberExpression(member) = node else {
            return Ok(false);
        };
        if member.optional || !self.stored.dictionary(member.object.node_id()) {
            return Ok(false);
        }
        if matches!(
            closures::unwrapped_at(&member.expression, depth + 1),
            Some((Expression::StringLiteral(_), _))
        ) {
            return Ok(false);
        }
        if !self.stored.string(member.expression.node_id()) {
            return Err(unsupported(
                node,
                "runtime dictionary keys require a known String",
            ));
        }
        work.push(Work::Dictionary(Task::ReadReceiver {
            node,
            key: &member.expression,
            depth: depth + 1,
        }));
        work.push(Work::Eval(&member.object, depth + 1, Mode::Object));
        Ok(true)
    }

    pub(super) fn resume_dictionary<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        match task {
            Task::ReadReceiver { node, key, depth } => {
                let object = values.pop().expect("dictionary receiver").id();
                work.push(Work::Dictionary(Task::ReadKey { node, object }));
                work.push(Work::Eval(key, depth, Mode::Managed));
            }
            Task::ReadKey { node, object } => {
                let key = values.pop().expect("dictionary key").id();
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedGetOwnDynamic {
                        result,
                        object,
                        key,
                    },
                );
                values.push(Value::Managed(result));
            }
        }
        Ok(())
    }
}
