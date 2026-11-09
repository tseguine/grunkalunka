//! Literal property names shared by source admission and field-origin analysis.
use super::*;
use oxc_ast::ast::PropertyKey;
use oxc_syntax::number::ToJsString;

pub(super) fn property(key: &PropertyKey<'_>, computed: bool, depth: usize) -> Option<Vec<u16>> {
    if let PropertyKey::StaticIdentifier(identifier) = key
        && !computed
    {
        return Some(identifier.name.encode_utf16().collect());
    }
    literal(closures::unwrapped(key.as_expression()?, depth)?)
}

pub(super) fn literal(expr: &Expression<'_>) -> Option<Vec<u16>> {
    match expr {
        Expression::StringLiteral(text) => return Some(text.value.encode_utf16().collect()),
        Expression::BooleanLiteral(value) => {
            let name = if value.value { "true" } else { "false" };
            return Some(name.encode_utf16().collect());
        }
        Expression::NullLiteral(_) => return Some("null".encode_utf16().collect()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            let [quasi] = template.quasis.as_slice() else {
                return None;
            };
            return Some(quasi.value.cooked.as_ref()?.encode_utf16().collect());
        }
        _ => {}
    }
    let number = number(expr)?;
    // Both zeros name the same property. Other values use ECMAScript Number::toString.
    let name = if number == 0.0 {
        "0".to_owned()
    } else {
        number.to_js_string()
    };
    Some(name.encode_utf16().collect())
}

pub(super) fn number(expr: &Expression<'_>) -> Option<f64> {
    match expr {
        Expression::NumericLiteral(value) => Some(value.value),
        Expression::UnaryExpression(unary) => {
            let Expression::NumericLiteral(value) = &unary.argument else {
                return None;
            };
            match unary.operator {
                UnaryOperator::UnaryNegation => Some(-value.value),
                UnaryOperator::UnaryPlus => Some(value.value),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(super) fn length_receiver<'e, 'a>(
    node: &'e Expression<'a>,
    depth: usize,
) -> Option<&'e Expression<'a>> {
    match node {
        Expression::StaticMemberExpression(m) if !m.optional && m.property.name == "length" => {
            Some(&m.object)
        }
        Expression::ComputedMemberExpression(m)
            if !m.optional
                && closures::unwrapped_at(&m.expression, depth + 1).is_some_and(
                    |(key, _)| matches!(key,Expression::StringLiteral(s) if s.value=="length"),
                ) =>
        {
            Some(&m.object)
        }
        _ => None,
    }
}

// Keep the prototype names consistent with each fixed fragment's position.
// Unknown operands may contribute any String. A literal below an operator
// other than concatenation provides no evidence about the resulting name.
fn constructed_own_name(expression: &Expression<'_>, depth: usize) -> bool {
    let mut candidates = crate::prototype_keys::OBJECT.to_vec();
    let mut pending = vec![(expression, depth, true, true)];
    while let Some((node, depth, first, last)) = pending.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        let mut fragment = |text: Option<&str>, prefix: bool, suffix: bool| {
            candidates.retain(|name| {
                text.is_some_and(|text| match (prefix, suffix) {
                    (true, true) => *name == text,
                    (true, false) => name.starts_with(text),
                    (false, true) => name.ends_with(text),
                    (false, false) => name.contains(text),
                })
            });
        };
        match node {
            Expression::StringLiteral(s) => fragment(s.value.as_str(), first, last),
            Expression::TemplateLiteral(t) => {
                for (i, quasi) in t.quasis.iter().enumerate() {
                    let Some(cooked) = &quasi.value.cooked else {
                        return false;
                    };
                    fragment(
                        cooked.as_str(),
                        first && i == 0,
                        last && i + 1 == t.quasis.len(),
                    );
                }
            }
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                pending.push((&b.right, depth + 1, false, last));
                pending.push((&b.left, depth + 1, first, false));
            }
            _ => {}
        }
        if candidates.is_empty() {
            return true;
        }
    }
    false
}

// A plain record can use its own-property table when every possible String
// key misses the pristine Object prototype. Validate operands separately;
// this walk only describes the result of a successful key evaluation.
pub(super) fn own_string_name(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    depth: usize,
) -> bool {
    use oxc_ast::AstKind;
    use std::collections::BTreeSet;

    let mut pending = vec![(expression, depth)];
    let mut followed = BTreeSet::new();
    while let Some((node, depth)) = pending.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        match node {
            Expression::StringLiteral(_) => {
                let Some(name) = literal(node) else {
                    return false;
                };
                if crate::prototype_keys::contains(crate::prototype_keys::OBJECT, &name) {
                    return false;
                }
            }
            Expression::TemplateLiteral(_) | Expression::BinaryExpression(_)
                if constructed_own_name(node, depth) => {}
            Expression::ConditionalExpression(c) => {
                pending.push((&c.consequent, depth + 1));
                pending.push((&c.alternate, depth + 1));
            }
            Expression::SequenceExpression(seq) => {
                let Some(last) = seq.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::Identifier(id) => {
                let Some(symbol) = id
                    .reference_id
                    .get()
                    .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                else {
                    return false;
                };
                let decl = semantic.symbol_declaration(symbol);
                let AstKind::VariableDeclarator(var) = decl.kind() else {
                    return false;
                };
                let AstKind::VariableDeclaration(parent) = semantic.nodes().parent_kind(decl.id())
                else {
                    return false;
                };
                if !matches!(var.id, BindingPattern::BindingIdentifier(_))
                    || var.span.end > id.span.start
                    || parent.kind != VariableDeclarationKind::Const
                {
                    return false;
                }
                if followed.insert(symbol) {
                    let Some(init) = var.init.as_ref() else {
                        return false;
                    };
                    pending.push((init, depth + 1));
                }
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn ordinary_record_keys_exclude_inherited_names_and_unchecked_sources() {
        for (key, admitted) in [
            ("a?'x':'y'", true),
            ("a?'push':'length'", true),
            ("a?'x':'__proto__'", false),
            ("a?'toString':'x'", false),
            ("a?`constructor`:'x'", false),
            ("'user:'+s", true),
            ("s+'.row'", true),
            ("s+'!'+s", true),
            ("`user:${s}`", true),
            ("`to${s}`", false),
            ("'to'+s", false),
            ("s+'String'", false),
            ("`${s}`", false),
            ("(':'+s).length", false),
            ("a as 'x'", false),
            ("(Math.random(),'x')", false),
        ] {
            let source = format!(
                "export function run(a,b){{const o={{x:a}},s=a?'String':'x',name={key};return o[name]===a?1:0;}}"
            );
            assert_eq!(crate::compile(&source, true).is_ok(), admitted, "{source}");
        }
        for body in [
            "const o={};const k=a?'x':'__proto__';o[k]=b;return a;",
            "const o={};let k='x';k='constructor';return o[k]===undefined?1:0;",
            "const o={};const {x:k}={x:'constructor'};return o[k]===undefined?1:0;",
            "const o={x:'abc'};const k=a?'x':'y';o[k]=b;return o.x.length;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
    }
}
