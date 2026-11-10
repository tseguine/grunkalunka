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
