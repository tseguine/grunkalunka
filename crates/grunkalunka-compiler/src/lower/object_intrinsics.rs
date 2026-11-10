//! Intrinsic recognition is independent of representation planning.
use super::*;

fn global_value(id: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> bool {
    id.reference_id.get().is_some_and(|r| {
        semantic
            .scoping()
            .get_reference(r)
            .symbol_id()
            .is_none_or(|s| !semantic.scoping().symbol_flags(s).is_value())
    })
}

pub(super) fn instance_test(
    id: &IdentifierReference<'_>,
    semantic: &Semantic<'_>,
) -> Option<ir::ManagedTest> {
    let test = match id.name.as_str() {
        "Array" => ir::ManagedTest::Array,
        "Function" => ir::ManagedTest::Function,
        "Object" => ir::ManagedTest::InheritsObject,
        _ => return None,
    };
    global_value(id, semantic).then_some(test)
}

fn array_constructor(callee: &Expression<'_>, semantic: &Semantic<'_>, depth: usize) -> bool {
    matches!(closures::unwrapped(callee,depth),Some(Expression::Identifier(id)) if id.name=="Array" && global_value(id,semantic))
}

pub(super) fn array_constructor_args<'e, 'a>(
    expr: &'e Expression<'a>,
    semantic: &Semantic<'_>,
    depth: usize,
) -> Option<&'e [oxc_ast::ast::Argument<'a>]> {
    match expr {
        Expression::CallExpression(call)
            if !call.optional && array_constructor(&call.callee, semantic, depth + 1) =>
        {
            Some(call.arguments.as_slice())
        }
        Expression::NewExpression(call) if array_constructor(&call.callee, semantic, depth + 1) => {
            Some(call.arguments.as_slice())
        }
        _ => None,
    }
}

pub(super) fn array_site_args<'a>(
    kind: oxc_ast::AstKind<'a>,
    semantic: &Semantic<'_>,
) -> Option<&'a [oxc_ast::ast::Argument<'a>]> {
    match kind {
        oxc_ast::AstKind::CallExpression(call)
            if !call.optional && array_constructor(&call.callee, semantic, 0) =>
        {
            Some(call.arguments.as_slice())
        }
        oxc_ast::AstKind::NewExpression(call) if array_constructor(&call.callee, semantic, 0) => {
            Some(call.arguments.as_slice())
        }
        _ => None,
    }
}

pub(super) fn array_constructor_site(kind: oxc_ast::AstKind<'_>, semantic: &Semantic<'_>) -> bool {
    array_site_args(kind, semantic).is_some()
}

fn method_name<'e>(
    call: &'e CallExpression<'_>,
    semantic: &Semantic<'_>,
    owner: &str,
) -> Option<&'e str> {
    if call.optional {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(invocation::callee(call), 0)?;
    let (object, name) = match callee {
        Expression::StaticMemberExpression(m) if !m.optional => {
            (&m.object, m.property.name.as_str())
        }
        Expression::ComputedMemberExpression(m) if !m.optional => {
            let (Expression::StringLiteral(key), _) = closures::unwrapped_at(&m.expression, 0)?
            else {
                return None;
            };
            (&m.object, key.value.as_str()?)
        }
        _ => return None,
    };
    let (Expression::Identifier(id), _) = closures::unwrapped_at(object, 0)? else {
        return None;
    };
    (id.name == owner && global_value(id, semantic)).then_some(name)
}

pub(super) fn is_method(call: &CallExpression<'_>, semantic: &Semantic<'_>, name: &str) -> bool {
    method_name(call, semantic, "Object") == Some(name)
}

pub(super) fn is_array_test(call: &CallExpression<'_>, semantic: &Semantic<'_>) -> bool {
    method_name(call, semantic, "Array") == Some("isArray")
}

pub(super) fn enumeration(
    call: &CallExpression<'_>,
    semantic: &Semantic<'_>,
) -> Option<ir::OwnEnumeration> {
    match method_name(call, semantic, "Object")? {
        "keys" => Some(ir::OwnEnumeration::Keys),
        "values" => Some(ir::OwnEnumeration::Values),
        "entries" => Some(ir::OwnEnumeration::Entries),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sparse_construction_requires_a_valid_numeric_length() {
        for body in [
            "return new Array(-1).length;",
            "return Array(1.5).length;",
            "return Array(4294967296).length;",
            "return new Array(a as 0).length;",
            "const Array=x=>x;return new Array(0).length;",
            "return Array(...a).length;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, true).is_err(), "{source}");
        }
    }

    #[test]
    fn instanceof_requires_a_resolved_intrinsic_constructor() {
        for source in [
            "export function run(a,b){const Object=x=>x;return {} instanceof Object?1:0;}",
            "export function run(a,b){const Function=x=>x;return (x=>x) instanceof Function?1:0;}",
            "export function run(a,b){const Array=x=>x;return [] instanceof Array?1:0;}",
            "export function run(a,b){return [] instanceof (a?Array:Object)?1:0;}",
            "export function run(a,b){return [] instanceof (a=b,Array)?1:0;}",
            "export function run(a,b){return [] instanceof Array.prototype?1:0;}",
            "export function run(a,b){Array.prototype={};return [] instanceof Array?1:0;}",
        ] {
            assert!(crate::compile(source, false).is_err(), "{source}");
        }
        let js = "export function run(a,b){const x=a?[b]:{};return x instanceof Array?1:0;}";
        let ts = "type Array=never;export function run(a:string,b:boolean):never{const x=a?[b]:{};return x instanceof (Array as never)?1:0;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
}
