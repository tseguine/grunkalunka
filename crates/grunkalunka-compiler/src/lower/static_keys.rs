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
            ("(':'+s).length", true),
            ("({}) as 'x'", false),
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
