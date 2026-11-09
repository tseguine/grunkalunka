//! Constant and earlier-parameter defaults share lazy Undefined selection.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{ArrowFunctionExpression, FormalParameters, Function},
};
use std::collections::BTreeSet;

enum Constant {
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
    String(Vec<u16>),
}

// Compare resolved symbols in the same parameter list. Source names, body var
// bindings and outer captures cannot establish this initialization-order proof.
fn earlier_parameter<'n, 'a>(
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(&'n IdentifierReference<'a>, Vec<UnaryOperator>)> {
    let mut node = expression;
    let mut depth = 0;
    let mut operators = Vec::new();
    let identifier = loop {
        let (inner, next) = closures::unwrapped_at(node, depth)?;
        node = inner;
        depth = next;
        match node {
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::Void
                        | UnaryOperator::LogicalNot
                        | UnaryOperator::UnaryPlus
                        | UnaryOperator::UnaryNegation
                ) =>
            {
                operators.push(unary.operator);
                node = &unary.argument;
                depth += 1;
            }
            Expression::Identifier(id) => break id.as_ref(),
            _ => return None,
        }
    };
    let symbol = semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()?;
    let parameters = semantic
        .nodes()
        .ancestor_kinds(expression.node_id())
        .find_map(|kind| match kind {
            AstKind::FormalParameters(parameters) => Some(parameters),
            _ => None,
        })?;
    let index = parameters.items.iter().position(|parameter| {
        parameter
            .initializer
            .as_ref()
            .is_some_and(|value| value.node_id() == expression.node_id())
    })?;
    parameters.items[..index].iter().any(|parameter| {
        matches!(&parameter.pattern, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol))
    }).then_some((identifier,operators))
}

// An initializer may read an already initialized parameter in either arm
// of a pure selection. Resolve identity in this parameter list, never by name:
// body var aliases, outer captures, self and later parameters are excluded.
enum DefaultOperand<'n, 'a> {
    Constant(Constant),
    Earlier(&'n IdentifierReference<'a>),
    // A direct own-property read on a source-proven earlier object argument.
    Field {
        receiver: &'n IdentifierReference<'a>,
        key: Vec<u16>,
    },
}

fn earlier_identifier<'n, 'a>(
    initializer: &Expression<'a>,
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<&'n IdentifierReference<'a>> {
    let (node, _) = closures::unwrapped_at(expression, 0)?;
    let Expression::Identifier(identifier) = node else {
        return None;
    };
    let identifier = identifier.as_ref();
    let symbol = semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()?;
    let parameters = semantic
        .nodes()
        .ancestor_kinds(initializer.node_id())
        .find_map(|kind| match kind {
            AstKind::FormalParameters(parameters) => Some(parameters),
            _ => None,
        })?;
    let index = parameters.items.iter().position(|parameter| {
        parameter
            .initializer
            .as_ref()
            .is_some_and(|value| value.node_id() == initializer.node_id())
    })?;
    parameters.items[..index]
        .iter()
        .any(|parameter| {
            matches!(&parameter.pattern, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol))
        })
        .then_some(identifier)
}

// Member defaults are pure only in the admitted data-object domain. An
// identifier must resolve to an earlier formal parameter, and the key must
// be static. Reject inherited intrinsic names instead of silently replacing
// prototype semantics with an own-property lookup.
fn earlier_field<'n, 'a>(
    initializer: &Expression<'a>,
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(&'n IdentifierReference<'a>, Vec<u16>)> {
    let (node, _) = closures::unwrapped_at(expression, 0)?;
    let (object, key) = match node {
        Expression::StaticMemberExpression(member) if !member.optional => {
            (&member.object, member.property.name.encode_utf16().collect())
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            (&member.object, static_keys::literal(&member.expression)?)
        }
        _ => return None,
    };
    if crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &key) {
        return None;
    }
    Some((earlier_identifier(initializer, object, semantic)?, key))
}

fn pure_operand<'n, 'a>(
    initializer: &Expression<'a>,
    operand: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<DefaultOperand<'n, 'a>> {
    if let Some(value) = constant(operand, semantic) {
        return Some(DefaultOperand::Constant(value));
    }
    if let Some((receiver, key)) = earlier_field(initializer, operand, semantic) {
        return Some(DefaultOperand::Field { receiver, key });
    }
    Some(DefaultOperand::Earlier(earlier_identifier(
        initializer, operand, semantic,
    )?))
}

// Pure numeric expressions in defaults may read initialized earlier parameters.
// This postorder instruction list has no native recursive descent.
enum NumericDefaultTerm<'n, 'a> {
    Constant(f64),
    Earlier(&'n IdentifierReference<'a>),
    Negate,
    Binary(Binary),
}

fn earlier_numeric<'n, 'a>(
    initializer: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<Vec<NumericDefaultTerm<'n, 'a>>> {
    enum Task<'n, 'a> {
        Visit(&'n Expression<'a>, usize),
        Negate,
        Binary(Binary),
    }
    let mut pending = vec![Task::Visit(initializer, 0)];
    let mut program = Vec::new();
    let mut has_parameter = false;
    let mut has_binary = false;
    while let Some(task) = pending.pop() {
        match task {
            Task::Visit(expression, depth) => {
                let (node, depth) = closures::unwrapped_at(expression, depth)?;
                if depth > 128 {
                    return None;
                }
                if let Some(Constant::Number(value)) = constant(node, semantic) {
                    program.push(NumericDefaultTerm::Constant(value));
                    continue;
                }
                match node {
                    Expression::BinaryExpression(binary) => {
                        let operator = match binary.operator {
                            BinaryOperator::Addition => Binary::Add,
                            BinaryOperator::Subtraction => Binary::Subtract,
                            BinaryOperator::Multiplication => Binary::Multiply,
                            BinaryOperator::Division => Binary::Divide,
                            BinaryOperator::Remainder => Binary::Remainder,
                            _ => return None,
                        };
                        has_binary = true;
                        pending.push(Task::Binary(operator));
                        pending.push(Task::Visit(&binary.right, depth + 1));
                        pending.push(Task::Visit(&binary.left, depth + 1));
                    }
                    Expression::UnaryExpression(unary)
                        if matches!(
                            unary.operator,
                            UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation
                        ) =>
                    {
                        if unary.operator == UnaryOperator::UnaryNegation {
                            pending.push(Task::Negate);
                        }
                        pending.push(Task::Visit(&unary.argument, depth + 1));
                    }
                    _ => {
                        let identifier = earlier_identifier(initializer, node, semantic)?;
                        has_parameter = true;
                        program.push(NumericDefaultTerm::Earlier(identifier));
                    }
                }
            }
            Task::Negate => program.push(NumericDefaultTerm::Negate),
            Task::Binary(operator) => program.push(NumericDefaultTerm::Binary(operator)),
        }
    }
    (has_binary && has_parameter).then_some(program)
}

// String defaults are built from source-proven String parameters and literal
// fragments. Reuse the ordinary managed String concatenation operation; do not
// silently apply ToPrimitive/ToString to unknown parameters or objects.
enum StringDefaultTerm<'n, 'a> {
    Constant(Vec<u16>),
    Earlier(&'n IdentifierReference<'a>),
    Concat,
}

fn earlier_string<'n, 'a>(
    initializer: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<Vec<StringDefaultTerm<'n, 'a>>> {
    enum Task<'n, 'a> {
        Visit(&'n Expression<'a>, usize),
        Literal(Vec<u16>),
        Concat,
    }
    let mut pending = vec![Task::Visit(initializer, 0)];
    let mut program = Vec::new();
    let mut has_parameter = false;
    let mut has_concat = false;
    while let Some(task) = pending.pop() {
        match task {
            Task::Visit(expression, depth) => {
                let (node, depth) = closures::unwrapped_at(expression, depth)?;
                if depth > 128 {
                    return None;
                }
                if let Some(Constant::String(units)) = constant(node, semantic) {
                    program.push(StringDefaultTerm::Constant(units));
                    continue;
                }
                match node {
                    Expression::BinaryExpression(binary)
                        if binary.operator == BinaryOperator::Addition =>
                    {
                        has_concat = true;
                        pending.push(Task::Concat);
                        pending.push(Task::Visit(&binary.right, depth + 1));
                        pending.push(Task::Visit(&binary.left, depth + 1));
                    }
                    Expression::TemplateLiteral(template) => {
                        if template.quasis.len() != template.expressions.len() + 1 {
                            return None;
                        }
                        let mut parts = Vec::new();
                        for (index, quasi) in template.quasis.iter().enumerate() {
                            let cooked = quasi.value.cooked.as_ref()?;
                            parts.push(Task::Literal(cooked.encode_utf16().collect()));
                            if let Some(expression) = template.expressions.get(index) {
                                parts.push(Task::Visit(expression, depth + 1));
                            }
                        }
                        has_concat |= parts.len() > 1;
                        // Postorder concatenation emits every fragment and expression
                        // in left-to-right order, without recursive source traversal.
                        for part in parts.drain(1..).rev() {
                            pending.push(Task::Concat);
                            pending.push(part);
                        }
                        pending.push(parts.pop()?);
                    }
                    _ => {
                        let identifier = earlier_identifier(initializer, node, semantic)?;
                        has_parameter = true;
                        program.push(StringDefaultTerm::Earlier(identifier));
                    }
                }
            }
            Task::Literal(units) => program.push(StringDefaultTerm::Constant(units)),
            Task::Concat => program.push(StringDefaultTerm::Concat),
        }
    }
    (has_parameter && has_concat).then_some(program)
}

// Numeric policy comparisons are intentionally restricted to four-kind
// primitive parameters and pure Number constants. Source-proven Strings use
// lexicographic relational comparison, so they must never enter this lane.
enum NumericComparisonOperand<'n, 'a> {
    Constant(f64),
    Earlier(&'n IdentifierReference<'a>),
}

fn comparison_operand<'n, 'a>(
    initializer: &Expression<'a>,
    operand: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<NumericComparisonOperand<'n, 'a>> {
    if let Some(value) = constant(operand, semantic) {
        return match value {
            Constant::Number(number) => Some(NumericComparisonOperand::Constant(number)),
            _ => None,
        };
    }
    Some(NumericComparisonOperand::Earlier(earlier_identifier(
        initializer, operand, semantic,
    )?))
}

fn earlier_comparison<'n, 'a>(
    initializer: &Expression<'a>,
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(
    Compare,
    NumericComparisonOperand<'n, 'a>,
    NumericComparisonOperand<'n, 'a>,
)> {
    let (node, _) = closures::unwrapped_at(expression, 0)?;
    let Expression::BinaryExpression(binary) = node else {
        return None;
    };
    let operator = match binary.operator {
        BinaryOperator::LessThan => Compare::Less,
        BinaryOperator::LessEqualThan => Compare::LessEqual,
        BinaryOperator::GreaterThan => Compare::Greater,
        BinaryOperator::GreaterEqualThan => Compare::GreaterEqual,
        // Equality requires same-kind and loose-coercion rules, not merely
        // Number conversion. Keep it in the general expression lowering.
        _ => return None,
    };
    let left = comparison_operand(initializer, &binary.left, semantic)?;
    let right = comparison_operand(initializer, &binary.right, semantic)?;
    Some((operator, left, right))
}

// Strict equality is useful for routing A1 actions by name, flag and identity.
// Both operands must be pure literals or resolved earlier parameters. Keep
// coercive == / != separate: they do not share strict identity semantics.
fn earlier_strict_equality<'n, 'a>(
    initializer: &Expression<'a>,
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(bool, DefaultOperand<'n, 'a>, DefaultOperand<'n, 'a>)> {
    let (node, _) = closures::unwrapped_at(expression, 0)?;
    let Expression::BinaryExpression(binary) = node else {
        return None;
    };
    let negated = match binary.operator {
        BinaryOperator::StrictEquality => false,
        BinaryOperator::StrictInequality => true,
        _ => return None,
    };
    let left = pure_operand(initializer, &binary.left, semantic)?;
    let right = pure_operand(initializer, &binary.right, semantic)?;
    if !matches!(&left, DefaultOperand::Earlier(_) | DefaultOperand::Field { .. })
        && !matches!(&right, DefaultOperand::Earlier(_) | DefaultOperand::Field { .. })
    {
        return None;
    }
    Some((negated, left, right))
}

enum DefaultCondition<'n, 'a> {
    Truthy(DefaultOperand<'n, 'a>),
    NumericComparison(
        Compare,
        NumericComparisonOperand<'n, 'a>,
        NumericComparisonOperand<'n, 'a>,
    ),
    StrictEquality(bool, DefaultOperand<'n, 'a>, DefaultOperand<'n, 'a>),
}

// Source-order logical default chains model common A1 policy precedence:
// value ?? override ?? fallback and enabled && flag && value. Each right
// operand is a pure constant or a resolved earlier parameter. Only the
// first left operand may be read before selection; nested evaluation is lazy.
fn earlier_logical_chain<'n, 'a>(
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(
    DefaultOperand<'n, 'a>,
    Vec<(LogicalOperator, DefaultOperand<'n, 'a>)>,
)> {
    let mut node = expression;
    let mut depth = 0;
    let mut steps = Vec::new();
    loop {
        let (inner, next) = closures::unwrapped_at(node, depth)?;
        node = inner;
        depth = next;
        let Expression::LogicalExpression(logical) = node else {
            break;
        };
        if depth >= 128 {
            return None;
        }
        steps.push((
            logical.operator,
            pure_operand(expression, &logical.right, semantic)?,
        ));
        node = &logical.left;
        depth += 1;
    }
    if steps.is_empty() {
        return None;
    }
    let original = pure_operand(expression, node, semantic)?;
    if !matches!(&original, DefaultOperand::Earlier(_) | DefaultOperand::Field { .. }) {
        return None;
    }
    steps.reverse();
    Some((original, steps))
}

// A conditional can select either an earlier source value or a pure
// constant. Both arms are validated before emitting runtime selection.
fn earlier_conditional<'n, 'a>(
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(
    DefaultCondition<'n, 'a>,
    DefaultOperand<'n, 'a>,
    DefaultOperand<'n, 'a>,
)> {
    let (node, _) = closures::unwrapped_at(expression, 0)?;
    let Expression::ConditionalExpression(conditional) = node else {
        return None;
    };
    let test = if let Some(operand) = pure_operand(expression, &conditional.test, semantic)
        .filter(|operand| matches!(operand, DefaultOperand::Earlier(_) | DefaultOperand::Field { .. }))
    {
        DefaultCondition::Truthy(operand)
    } else if let Some((operator, left, right)) =
        earlier_comparison(expression, &conditional.test, semantic)
    {
        DefaultCondition::NumericComparison(operator, left, right)
    } else {
        let (negated, left, right) =
            earlier_strict_equality(expression, &conditional.test, semantic)?;
        DefaultCondition::StrictEquality(negated, left, right)
    };
    Some((
        test,
        pure_operand(expression, &conditional.consequent, semantic)?,
        pure_operand(expression, &conditional.alternate, semantic)?,
    ))
}

impl Constant {
    fn truthy(&self) -> bool {
        match self {
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::Boolean(value) => *value,
            Self::String(units) => !units.is_empty(),
            Self::Null | Self::Undefined => false,
        }
    }

    fn nullish(&self) -> bool {
        matches!(self, Self::Null | Self::Undefined)
    }
}

// Defaults must be independent of the caller and free of effects. Evaluate
// every operand, including unselected branches, to reject hidden calls, name
// reads or assignments rather than silently treating them as pure constants.
fn constant(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Option<Constant> {
    enum Task<'n, 'a> {
        Visit(&'n Expression<'a>, usize),
        Unary(UnaryOperator),
        Binary(BinaryOperator),
        Logical(LogicalOperator),
        Conditional,
    }
    let mut work = vec![Task::Visit(expression, 0)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Task::Visit(expression, depth) => {
                let (node, depth) = closures::unwrapped_at(expression, depth)?;
                if depth > 128 {
                    return None;
                }
                let value = match node {
                    Expression::UnaryExpression(unary)
                        if matches!(
                            unary.operator,
                            UnaryOperator::UnaryPlus
                                | UnaryOperator::UnaryNegation
                                | UnaryOperator::LogicalNot
                                | UnaryOperator::Void
                        ) =>
                    {
                        work.push(Task::Unary(unary.operator));
                        work.push(Task::Visit(&unary.argument, depth + 1));
                        continue;
                    }
                    Expression::BinaryExpression(binary)
                        if matches!(
                            binary.operator,
                            BinaryOperator::Addition
                                | BinaryOperator::Subtraction
                                | BinaryOperator::Multiplication
                                | BinaryOperator::Division
                                | BinaryOperator::Remainder
                        ) =>
                    {
                        work.push(Task::Binary(binary.operator));
                        work.push(Task::Visit(&binary.right, depth + 1));
                        work.push(Task::Visit(&binary.left, depth + 1));
                        continue;
                    }
                    Expression::LogicalExpression(logical) => {
                        work.push(Task::Logical(logical.operator));
                        work.push(Task::Visit(&logical.right, depth + 1));
                        work.push(Task::Visit(&logical.left, depth + 1));
                        continue;
                    }
                    Expression::ConditionalExpression(conditional) => {
                        work.push(Task::Conditional);
                        work.push(Task::Visit(&conditional.alternate, depth + 1));
                        work.push(Task::Visit(&conditional.consequent, depth + 1));
                        work.push(Task::Visit(&conditional.test, depth + 1));
                        continue;
                    }
                    Expression::NumericLiteral(n) => Constant::Number(n.value),
                    Expression::BooleanLiteral(b) => Constant::Boolean(b.value),
                    Expression::NullLiteral(_) => Constant::Null,
                    Expression::StringLiteral(s) => {
                        Constant::String(s.value.encode_utf16().collect())
                    }
                    Expression::Identifier(id)
                        if id.reference_id.get().is_some_and(|r| {
                            semantic.scoping().get_reference(r).symbol_id().is_none()
                        }) =>
                    {
                        match id.name.as_str() {
                            "undefined" => Constant::Undefined,
                            "Infinity" => Constant::Number(f64::INFINITY),
                            "NaN" => Constant::Number(f64::NAN),
                            _ => return None,
                        }
                    }
                    _ => return None,
                };
                values.push(value);
            }
            Task::Unary(op) => {
                let operand = values.pop()?;
                let value = match (op, operand) {
                    (UnaryOperator::Void, _) => Constant::Undefined,
                    (UnaryOperator::LogicalNot, operand) => Constant::Boolean(!operand.truthy()),
                    (UnaryOperator::UnaryPlus, Constant::Number(n)) => Constant::Number(n),
                    (UnaryOperator::UnaryNegation, Constant::Number(n)) => Constant::Number(-n),
                    _ => return None,
                };
                values.push(value);
            }
            Task::Binary(op) => {
                let right = values.pop()?;
                let left = values.pop()?;
                let value = match (op, left, right) {
                    (
                        BinaryOperator::Addition,
                        Constant::String(mut left),
                        Constant::String(right),
                    ) => {
                        left.extend(right);
                        Constant::String(left)
                    }
                    (op, Constant::Number(left), Constant::Number(right)) => {
                        Constant::Number(match op {
                            BinaryOperator::Addition => left + right,
                            BinaryOperator::Subtraction => left - right,
                            BinaryOperator::Multiplication => left * right,
                            BinaryOperator::Division => left / right,
                            BinaryOperator::Remainder => left % right,
                            _ => unreachable!("planned constant arithmetic"),
                        })
                    }
                    _ => return None,
                };
                values.push(value);
            }
            Task::Logical(op) => {
                let right = values.pop()?;
                let left = values.pop()?;
                let take_right = match op {
                    LogicalOperator::And => left.truthy(),
                    LogicalOperator::Or => !left.truthy(),
                    LogicalOperator::Coalesce => left.nullish(),
                };
                values.push(if take_right { right } else { left });
            }
            Task::Conditional => {
                let alternate = values.pop()?;
                let consequent = values.pop()?;
                let test = values.pop()?;
                values.push(if test.truthy() { consequent } else { alternate });
            }
        }
    }
    values.pop()
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
            if constant(initializer, semantic).is_none()
                && earlier_parameter(initializer, semantic).is_none()
                && earlier_field(initializer, initializer, semantic).is_none()
                && earlier_logical_chain(initializer, semantic).is_none()
                && earlier_conditional(initializer, semantic).is_none()
                && earlier_numeric(initializer, semantic).is_none()
                && earlier_string(initializer, semantic).is_none()
                && earlier_comparison(initializer, initializer, semantic).is_none()
                && earlier_strict_equality(initializer, initializer, semantic).is_none()
            {
                return Err(unsupported(
                    initializer,
                    "helper defaults require primitive constants",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_arrow(
    arrow: &ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
    allowed: bool,
) -> Result<(), String> {
    for parameter in &arrow.params.items {
        if let Some(initializer) = &parameter.initializer
            && (!allowed
                || (constant(initializer.as_ref(), semantic).is_none()
                    && earlier_parameter(initializer.as_ref(), semantic).is_none()
                    && earlier_field(initializer.as_ref(), initializer.as_ref(), semantic).is_none()
                    && earlier_logical_chain(initializer.as_ref(), semantic).is_none()
                    && earlier_conditional(initializer.as_ref(), semantic).is_none()
                    && earlier_numeric(initializer.as_ref(), semantic).is_none()
                    && earlier_string(initializer.as_ref(), semantic).is_none()
                    && earlier_comparison(initializer.as_ref(), initializer.as_ref(), semantic)
                        .is_none()
                    && earlier_strict_equality(initializer.as_ref(), initializer.as_ref(), semantic)
                        .is_none()))
        {
            return Err(unsupported(
                parameter,
                "local arrow parameters cannot have defaults, destructuring or modifiers",
            ));
        }
    }
    Ok(())
}

pub(super) fn present(semantic: &Semantic<'_>) -> bool {
    semantic.nodes().iter().any(|node| match node.kind() {
        AstKind::Function(f) => f.params.items.iter().any(|p| p.initializer.is_some()),
        AstKind::ArrowFunctionExpression(a) => {
            a.params.items.iter().any(|p| p.initializer.is_some())
        }
        _ => false,
    })
}

impl Lowering<'_, '_> {
    pub(crate) fn initialize_helper_defaults(
        &mut self,
        function: &Function<'_>,
        output: &mut Region,
    ) -> Result<(), String> {
        self.initialize_parameter_defaults(&function.params, output)
    }

    pub(super) fn initialize_parameter_defaults(
        &mut self,
        parameters: &FormalParameters<'_>,
        output: &mut Region,
    ) -> Result<(), String> {
        for (index, parameter) in parameters.items.iter().enumerate() {
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
            let value = self.select_parameter_default(parameter, initializer, selected, output)?;
            Self::push(output, parameter, Op::Store { slot, value });
        }
        Ok(())
    }

    pub(super) fn select_parameter_default(
        &mut self,
        parameter: &impl GetSpan,
        initializer: &Expression<'_>,
        mut selected: ValueId,
        output: &mut Region,
    ) -> Result<ValueId, String> {
        if self.values[selected.0] != Type::ManagedValue {
            let primitive = match self.values[selected.0] {
                Type::General => selected,
                Type::Number | Type::Predicate => {
                    let packed = self.value(Type::General);
                    Self::push(
                        output,
                        parameter,
                        Op::Pack {
                            result: packed,
                            input: selected,
                        },
                    );
                    packed
                }
                _ => {
                    return Err(unsupported(
                        parameter,
                        "default parameter requires represented primitive storage",
                    ));
                }
            };
            selected = self.value(Type::ManagedValue);
            Self::push(
                output,
                parameter,
                Op::ManagedPrimitive {
                    result: selected,
                    input: primitive,
                },
            );
        }
        let undefined = self.binding_undefined(parameter, output);
        let pending = self.prepare_binding_default(parameter, output, selected, undefined);
        let mut consequent = self.region(initializer);
        if let Some((test, when_true, when_false)) =
            earlier_conditional(initializer, self.semantic)
        {
            let condition = match test {
                DefaultCondition::Truthy(operand) => {
                    let source =
                        self.emit_default_operand(initializer, &mut consequent, operand)?;
                    let result = self.value(Type::Predicate);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedTruthy {
                            result,
                            input: source,
                        },
                    );
                    result
                }
                DefaultCondition::NumericComparison(operator, left, right) => {
                    self.emit_numeric_comparison(
                        initializer,
                        &mut consequent,
                        operator,
                        left,
                        right,
                    )?
                }
                DefaultCondition::StrictEquality(negated, left, right) => {
                    self.emit_strict_equality(
                        initializer,
                        &mut consequent,
                        negated,
                        left,
                        right,
                    )?
                }
            };
            let mut true_region = self.region(initializer);
            let true_value =
                self.emit_default_operand(initializer, &mut true_region, when_true)?;
            true_region.exit = Exit::Yield(true_value);
            let mut false_region = self.region(initializer);
            let false_value =
                self.emit_default_operand(initializer, &mut false_region, when_false)?;
            false_region.exit = Exit::Yield(false_value);
            let result = self.value(Type::ManagedValue);
            Self::push(
                &mut consequent,
                initializer,
                Op::If {
                    result: Some(result),
                    condition,
                    consequent: true_region,
                    alternate: false_region,
                },
            );
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, result));
        }
        if let Some((start, steps)) = earlier_logical_chain(initializer, self.semantic) {
            let mut original = self.emit_default_operand(initializer, &mut consequent, start)?;
            for (operator, fallback) in steps {
                let condition = self.value(Type::Predicate);
                Self::push(
                    &mut consequent,
                    initializer,
                    if operator == LogicalOperator::Coalesce {
                        Op::ManagedIsNullish {
                            result: condition,
                            input: original,
                        }
                    } else {
                        Op::ManagedTruthy {
                            result: condition,
                            input: original,
                        }
                    },
                );
                let mut selected = self.region(initializer);
                let fallback = self.emit_default_operand(initializer, &mut selected, fallback)?;
                selected.exit = Exit::Yield(fallback);
                let mut kept = self.region(initializer);
                kept.exit = Exit::Yield(original);
                let (if_true, if_false) = if operator == LogicalOperator::Or {
                    (kept, selected)
                } else {
                    (selected, kept)
                };
                let result = self.value(Type::ManagedValue);
                Self::push(
                    &mut consequent,
                    initializer,
                    Op::If {
                        result: Some(result),
                        condition,
                        consequent: if_true,
                        alternate: if_false,
                    },
                );
                original = result;
            }
            return Ok(self.finish_binding_default(
                parameter, output, pending, consequent, original,
            ));
        }
        if let Some((receiver, key)) = earlier_field(initializer, initializer, self.semantic) {
            let value = self.emit_default_field(initializer, &mut consequent, receiver, key)?;
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        if let Some((identifier, operators)) = earlier_parameter(initializer, self.semantic) {
            let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
            let ty = self.slots[slot.0].ty;
            let input = self.value(ty);
            Self::push(
                &mut consequent,
                initializer,
                Op::Load {
                    result: input,
                    slot,
                },
            );
            let mut value = if ty == Type::ManagedValue {
                input
            } else {
                let result = self.value(Type::ManagedValue);
                let op = match ty {
                    Type::ManagedArray => Op::ManagedFromArray { result, input },
                    Type::ManagedObject => Op::ManagedFromObject { result, input },
                    Type::Number | Type::Predicate | Type::General => {
                        let primitive = if ty == Type::General {
                            input
                        } else {
                            let packed = self.value(Type::General);
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Pack {
                                    result: packed,
                                    input,
                                },
                            );
                            packed
                        };
                        Op::ManagedPrimitive {
                            result,
                            input: primitive,
                        }
                    }
                    _ => {
                        return Err(unsupported(
                            initializer,
                            "earlier parameter default requires a represented value",
                        ));
                    }
                };
                Self::push(&mut consequent, initializer, op);
                result
            };
            let mut primitive = self.stored.primitive(identifier.node_id());
            for operator in operators.into_iter().rev() {
                if operator == UnaryOperator::Void {
                    value = self.binding_undefined(initializer, &mut consequent);
                    primitive = true;
                    continue;
                }
                let scalar = if operator == UnaryOperator::LogicalNot {
                    let truth = self.value(Type::Predicate);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedTruthy {
                            result: truth,
                            input: value,
                        },
                    );
                    let result = self.value(Type::Predicate);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::Not {
                            result,
                            input: truth,
                        },
                    );
                    result
                } else {
                    if !primitive {
                        return Err(unsupported(
                            initializer,
                            "numeric parameter defaults require a proven four-kind primitive operand",
                        ));
                    }
                    let general = self.value(Type::General);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedToPrimitive {
                            result: general,
                            input: value,
                        },
                    );
                    let number = self.value(Type::Number);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::NumberFromGeneral {
                            result: number,
                            input: general,
                        },
                    );
                    if operator == UnaryOperator::UnaryNegation {
                        let result = self.value(Type::Number);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::Unary {
                                result,
                                operator: Unary::Negate,
                                input: number,
                            },
                        );
                        result
                    } else {
                        number
                    }
                };
                let general = self.value(Type::General);
                Self::push(
                    &mut consequent,
                    initializer,
                    Op::Pack {
                        result: general,
                        input: scalar,
                    },
                );
                value = self.value(Type::ManagedValue);
                Self::push(
                    &mut consequent,
                    initializer,
                    Op::ManagedPrimitive {
                        result: value,
                        input: general,
                    },
                );
                primitive = true;
            }
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        // Both Number and String additions share + syntax. Only select the
        // String planner with positive source proofs; otherwise allow the
        // existing numeric planner to handle primitive numeric expressions.
        if let Some(program) = earlier_string(initializer, self.semantic).filter(|program| {
            program.iter().all(|term| match term {
                StringDefaultTerm::Earlier(identifier) => self.stored.string(identifier.node_id()),
                _ => true,
            })
        }) {
            let mut strings = Vec::new();
            for term in program {
                match term {
                    StringDefaultTerm::Constant(units) => {
                        strings.push(self.emit_constant_default(
                            initializer,
                            &mut consequent,
                            Constant::String(units),
                        ));
                    }
                    StringDefaultTerm::Earlier(identifier) => {
                        strings.push(self.load_earlier_value(
                            identifier,
                            initializer,
                            &mut consequent,
                        )?);
                    }
                    StringDefaultTerm::Concat => {
                        let right = strings.pop().expect("validated String right");
                        let left = strings.pop().expect("validated String left");
                        let result = self.value(Type::ManagedValue);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::ManagedStringConcat {
                                result,
                                left,
                                right,
                            },
                        );
                        strings.push(result);
                    }
                }
            }
            let value = strings.pop().expect("validated String result");
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        if let Some((operator, left, right)) =
            earlier_comparison(initializer, initializer, self.semantic)
        {
            let condition = self.emit_numeric_comparison(
                initializer,
                &mut consequent,
                operator,
                left,
                right,
            )?;
            let general = self.value(Type::General);
            Self::push(
                &mut consequent,
                initializer,
                Op::Pack {
                    result: general,
                    input: condition,
                },
            );
            let value = self.value(Type::ManagedValue);
            Self::push(
                &mut consequent,
                initializer,
                Op::ManagedPrimitive {
                    result: value,
                    input: general,
                },
            );
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        if let Some((negated, left, right)) =
            earlier_strict_equality(initializer, initializer, self.semantic)
        {
            let condition = self.emit_strict_equality(
                initializer,
                &mut consequent,
                negated,
                left,
                right,
            )?;
            let general = self.value(Type::General);
            Self::push(
                &mut consequent,
                initializer,
                Op::Pack {
                    result: general,
                    input: condition,
                },
            );
            let value = self.value(Type::ManagedValue);
            Self::push(
                &mut consequent,
                initializer,
                Op::ManagedPrimitive {
                    result: value,
                    input: general,
                },
            );
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        if let Some(program) = earlier_numeric(initializer, self.semantic) {
            let mut numbers = Vec::new();
            for term in program {
                match term {
                    NumericDefaultTerm::Constant(value) => {
                        let result = self.value(Type::Number);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::Constant { result, value },
                        );
                        numbers.push(result);
                    }
                    NumericDefaultTerm::Earlier(identifier) => {
                        if !self.stored.primitive(identifier.node_id()) {
                            return Err(unsupported(
                                initializer,
                                "numeric default requires a proven four-kind primitive parameter",
                            ));
                        }
                        let managed =
                            self.load_earlier_value(identifier, initializer, &mut consequent)?;
                        let general = self.value(Type::General);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::ManagedToPrimitive {
                                result: general,
                                input: managed,
                            },
                        );
                        let result = self.value(Type::Number);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::NumberFromGeneral {
                                result,
                                input: general,
                            },
                        );
                        numbers.push(result);
                    }
                    NumericDefaultTerm::Negate => {
                        let input = numbers.pop().expect("validated numeric operand");
                        let result = self.value(Type::Number);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::Unary {
                                result,
                                operator: Unary::Negate,
                                input,
                            },
                        );
                        numbers.push(result);
                    }
                    NumericDefaultTerm::Binary(operator) => {
                        let right = numbers.pop().expect("validated numeric right");
                        let left = numbers.pop().expect("validated numeric left");
                        let result = self.value(Type::Number);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::Binary {
                                result,
                                operator,
                                left,
                                right,
                            },
                        );
                        numbers.push(result);
                    }
                }
            }
            let number = numbers.pop().expect("validated numeric result");
            let general = self.value(Type::General);
            Self::push(
                &mut consequent,
                initializer,
                Op::Pack {
                    result: general,
                    input: number,
                },
            );
            let value = self.value(Type::ManagedValue);
            Self::push(
                &mut consequent,
                initializer,
                Op::ManagedPrimitive {
                    result: value,
                    input: general,
                },
            );
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        let folded = constant(initializer, self.semantic).ok_or_else(|| {
            unsupported(initializer, "helper defaults require primitive constants")
        })?;
        let value = self.emit_constant_default(initializer, &mut consequent, folded);
        Ok(self.finish_binding_default(parameter, output, pending, consequent, value))
    }

    fn emit_strict_equality(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        negated: bool,
        left: DefaultOperand<'_, '_>,
        right: DefaultOperand<'_, '_>,
    ) -> Result<ValueId, String> {
        // The operands are already proven pure. Nonetheless, materialize them
        // in source order so references retain identity and no coercion occurs.
        let left = self.emit_default_operand(node, region, left)?;
        let right = self.emit_default_operand(node, region, right)?;
        let equal = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::ManagedEqual {
                result: equal,
                left,
                right,
            },
        );
        if negated {
            let result = self.value(Type::Predicate);
            Self::push(
                region,
                node,
                Op::Not {
                    result,
                    input: equal,
                },
            );
            Ok(result)
        } else {
            Ok(equal)
        }
    }

    fn emit_numeric_comparison_operand(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        operand: NumericComparisonOperand<'_, '_>,
    ) -> Result<ValueId, String> {
        match operand {
            NumericComparisonOperand::Constant(number) => {
                let result = self.value(Type::Number);
                Self::push(
                    region,
                    node,
                    Op::Constant {
                        result,
                        value: number,
                    },
                );
                Ok(result)
            }
            NumericComparisonOperand::Earlier(identifier) => {
                if !self.stored.primitive(identifier.node_id()) {
                    return Err(unsupported(
                        node,
                        "numeric comparison default requires a proven four-kind primitive parameter",
                    ));
                }
                let managed = self.load_earlier_value(identifier, node, region)?;
                let general = self.value(Type::General);
                Self::push(
                    region,
                    node,
                    Op::ManagedToPrimitive {
                        result: general,
                        input: managed,
                    },
                );
                let result = self.value(Type::Number);
                Self::push(
                    region,
                    node,
                    Op::NumberFromGeneral {
                        result,
                        input: general,
                    },
                );
                Ok(result)
            }
        }
    }

    fn emit_numeric_comparison(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        operator: Compare,
        left: NumericComparisonOperand<'_, '_>,
        right: NumericComparisonOperand<'_, '_>,
    ) -> Result<ValueId, String> {
        let left = self.emit_numeric_comparison_operand(node, region, left)?;
        let right = self.emit_numeric_comparison_operand(node, region, right)?;
        let result = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::Compare {
                result,
                operator,
                left,
                right,
            },
        );
        Ok(result)
    }

    fn load_earlier_value(
        &mut self,
        identifier: &IdentifierReference<'_>,
        node: &Expression<'_>,
        region: &mut Region,
    ) -> Result<ValueId, String> {
        let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
        let ty = self.slots[slot.0].ty;
        let input = self.value(ty);
        Self::push(
            region,
            node,
            Op::Load {
                result: input,
                slot,
            },
        );
        let original = if ty == Type::ManagedValue {
            input
        } else {
            let result = self.value(Type::ManagedValue);
            let op = match ty {
                Type::ManagedArray => Op::ManagedFromArray { result, input },
                Type::ManagedObject => Op::ManagedFromObject { result, input },
                Type::Number | Type::Predicate | Type::General => {
                    let primitive = if ty == Type::General {
                        input
                    } else {
                        let packed = self.value(Type::General);
                        Self::push(
                            region,
                            node,
                            Op::Pack {
                                result: packed,
                                input,
                            },
                        );
                        packed
                    };
                    Op::ManagedPrimitive {
                        result,
                        input: primitive,
                    }
                }
                _ => {
                    return Err(unsupported(
                        node,
                        "earlier parameter default requires a represented value",
                    ));
                }
            };
            Self::push(region, node, op);
            result
        };
        Ok(original)
    }

    fn emit_default_field(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        receiver: &IdentifierReference<'_>,
        key: Vec<u16>,
    ) -> Result<ValueId, String> {
        // A positive source-origin proof excludes nullish/primitive receivers.
        // Arrays and ordinary objects have own data slots; inherited accessors
        // and methods are not the same operation as an own-property lookup.
        let kind = self.stored.container(receiver.node_id()).ok_or_else(|| {
            unsupported(node, "property parameter defaults require a proven object or array")
        })?;
        if !matches!(kind, Type::ManagedObject | Type::ManagedArray) {
            return Err(unsupported(
                node,
                "property parameter defaults require a proven object or array",
            ));
        }
        if kind == Type::ManagedArray
            && crate::prototype_keys::contains(crate::prototype_keys::ARRAY, &key)
        {
            return Err(unsupported(
                node,
                "array prototype properties are not pure own-property defaults",
            ));
        }
        let boxed = self.load_earlier_value(receiver, node, region)?;
        let container = self.value(kind);
        Self::push(
            region,
            node,
            if kind == Type::ManagedArray {
                Op::ManagedToArray {
                    result: container,
                    input: boxed,
                }
            } else {
                Op::ManagedToObject {
                    result: container,
                    input: boxed,
                }
            },
        );
        let value = self.value(Type::ManagedValue);
        Self::push(
            region,
            node,
            if kind == Type::ManagedArray {
                Op::ManagedArrayGetOwn {
                    result: value,
                    array: container,
                    key,
                }
            } else {
                Op::ManagedGetOwn {
                    result: value,
                    object: container,
                    key,
                }
            },
        );
        Ok(value)
    }

    fn emit_default_operand(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        operand: DefaultOperand<'_, '_>,
    ) -> Result<ValueId, String> {
        match operand {
            DefaultOperand::Constant(value) => Ok(self.emit_constant_default(node, region, value)),
            DefaultOperand::Earlier(identifier) => self.load_earlier_value(identifier, node, region),
            DefaultOperand::Field { receiver, key } => {
                self.emit_default_field(node, region, receiver, key)
            }
        }
    }

    fn emit_constant_default(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        constant: Constant,
    ) -> ValueId {
        match constant {
            Constant::String(units) => {
                let value = self.value(Type::ManagedValue);
                Self::push(
                    region,
                    node,
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
                            region,
                            node,
                            Op::Constant {
                                result: scalar,
                                value,
                            },
                        );
                        Self::push(
                            region,
                            node,
                            Op::Pack {
                                result: primitive,
                                input: scalar,
                            },
                        );
                    }
                    Constant::Boolean(value) => {
                        let scalar = self.value(Type::Predicate);
                        Self::push(
                            region,
                            node,
                            Op::Boolean {
                                result: scalar,
                                value,
                            },
                        );
                        Self::push(
                            region,
                            node,
                            Op::Pack {
                                result: primitive,
                                input: scalar,
                            },
                        );
                    }
                    Constant::Null | Constant::Undefined => Self::push(
                        region,
                        node,
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
                    region,
                    node,
                    Op::ManagedPrimitive {
                        result: value,
                        input: primitive,
                    },
                );
                value
            }
        }
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
                    lengths.insert(
                        a.params
                            .items
                            .iter()
                            .take_while(|p| p.initializer.is_none())
                            .count(),
                    );
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
