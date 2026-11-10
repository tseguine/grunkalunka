//! Primitive ToString on the expression cursor, including explicit String calls.
use super::*;

pub(super) enum Conversion {
    Scalar { numbers: bool },
    NumberLiteral(Vec<u16>),
}

impl Lowering<'_, '_> {
    fn string_operand(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Result<(Mode, Option<Conversion>), String> {
        if self.stored.string(node.node_id()) {
            return Ok((Mode::Managed, None));
        }
        if let Some(literal) = closures::unwrapped(node, depth)
            && static_keys::number(literal).is_some()
        {
            let units =
                static_keys::literal(literal).expect("Number literal has a property spelling");
            return Ok((Mode::Number, Some(Conversion::NumberLiteral(units))));
        }
        if self.stored.scalar_string_operand(node.node_id()) {
            let numbers = self.stored.may_be_number(node.node_id());
            return Ok((Mode::Managed, Some(Conversion::Scalar { numbers })));
        }
        Err(unsupported(
            node,
            "String conversion requires a supported primitive value",
        ))
    }

    pub(super) fn schedule_string_operand<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let (mode, conversion) = self.string_operand(node, depth)?;
        if let Some(conversion) = conversion {
            work.push(Work::StringOperand(node, conversion));
        }
        work.push(Work::Eval(node, depth, mode));
        Ok(())
    }

    pub(super) fn schedule_string_call<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        depth: usize,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let arguments = invocation::plain_arguments(&call.arguments);
        if arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "String accepts at most 1000 fixed arguments",
            ));
        }
        let mut args = arguments.into_iter();
        let Some(first) = args.next() else {
            let result = self.value(Type::ManagedValue);
            Self::push(
                regions.current(),
                node,
                Op::ManagedString {
                    result,
                    units: Vec::new(),
                },
            );
            values.push(Value::Managed(result));
            return Ok(());
        };
        let first_depth = depth + first.depth + 1;
        let first = first
            .as_expression()
            .ok_or_else(|| unsupported(node, "String requires fixed arguments"))?;
        let (mode, conversion) = self.string_operand(first, first_depth)?;
        if let Some(conversion) = conversion {
            work.push(Work::StringOperand(first, conversion));
        }
        // Keep the first value on the cursor while all remaining arguments run.
        for actual in args.rev() {
            let arg = actual
                .as_expression()
                .ok_or_else(|| unsupported(node, "String requires fixed arguments"))?;
            work.push(Work::Eval(arg, depth + actual.depth + 1, Mode::Discard));
        }
        work.push(Work::Eval(first, first_depth, mode));
        Ok(())
    }

    pub(super) fn finish_string_operand(
        &mut self,
        node: &Expression<'_>,
        conversion: Conversion,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let input = values.pop().expect("String conversion operand").id();
        let result = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            node,
            match conversion {
                Conversion::Scalar { numbers } => Op::ManagedScalarString {
                    result,
                    input,
                    numbers,
                },
                Conversion::NumberLiteral(units) => Op::ManagedString { result, units },
            },
        );
        values.push(Value::Managed(result));
    }
}
