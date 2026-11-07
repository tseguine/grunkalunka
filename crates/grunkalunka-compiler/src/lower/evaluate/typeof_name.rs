//! Unresolvable references in the current closed source environment.
use super::*;

pub(super) fn unresolvable(node: &Expression<'_>, semantic: &Semantic<'_>, depth: usize) -> bool {
    let Some((Expression::Identifier(identifier), _)) = closures::unwrapped_at(node, depth) else {
        return false;
    };
    identifier.reference_id.get().is_some_and(|reference| {
        semantic
            .scoping()
            .get_reference(reference)
            .symbol_id()
            .is_none()
            && !environment_name(identifier.name.as_str())
    })
}

fn environment_name(name: &str) -> bool {
    // Semantic resolution covers lexical/module bindings, not intrinsic globals.
    // Never infer absence for ECMAScript global names merely because their value
    // operations are not implemented. ECMA-262 sections 19.1--19.4 and Annex B.2.1:
    // https://tc39.es/ecma262/multipage/global-object.html
    // Intl/WebAssembly and common host namespaces also keep their admission guard.
    matches!(
        name,
        "globalThis"
            | "Infinity"
            | "NaN"
            | "undefined"
            | "eval"
            | "isFinite"
            | "isNaN"
            | "parseFloat"
            | "parseInt"
            | "decodeURI"
            | "decodeURIComponent"
            | "encodeURI"
            | "encodeURIComponent"
            | "escape"
            | "unescape"
            | "AggregateError"
            | "Array"
            | "ArrayBuffer"
            | "AsyncDisposableStack"
            | "BigInt"
            | "BigInt64Array"
            | "BigUint64Array"
            | "Boolean"
            | "DataView"
            | "Date"
            | "DisposableStack"
            | "Error"
            | "EvalError"
            | "FinalizationRegistry"
            | "Float16Array"
            | "Float32Array"
            | "Float64Array"
            | "Function"
            | "Int8Array"
            | "Int16Array"
            | "Int32Array"
            | "Iterator"
            | "Map"
            | "Number"
            | "Object"
            | "Promise"
            | "Proxy"
            | "RangeError"
            | "ReferenceError"
            | "RegExp"
            | "Set"
            | "SharedArrayBuffer"
            | "String"
            | "SuppressedError"
            | "Symbol"
            | "SyntaxError"
            | "TypeError"
            | "Uint8Array"
            | "Uint8ClampedArray"
            | "Uint16Array"
            | "Uint32Array"
            | "URIError"
            | "WeakMap"
            | "WeakRef"
            | "WeakSet"
            | "Atomics"
            | "JSON"
            | "Math"
            | "Reflect"
            | "Intl"
            | "WebAssembly"
            | "arguments"
            | "global"
            | "process"
            | "require"
            | "module"
            | "exports"
            | "Buffer"
            | "console"
            | "window"
            | "document"
            | "self"
            | "navigator"
            | "location"
            | "fetch"
            | "Deno"
            | "Bun"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn absent_typeof_references_keep_the_original_depth_limit() {
        for (wraps, admitted) in [(127, true), (128, false)] {
            let source = format!(
                "export function run(a,b){{const kind=typeof {}missingFeature{};return a;}}",
                "(".repeat(wraps),
                ")".repeat(wraps)
            );
            let result = crate::compile(&source, false);
            assert_eq!(
                result.is_ok(),
                admitted,
                "wraps {wraps}: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn typeof_absent_names_keeps_getvalue_and_tdz_boundaries() {
        for (body, admitted) in [
            ("return typeof missingFeature==='undefined'?1:0;", true),
            ("return typeof ((missingFeature))==='undefined'?1:0;", true),
            (
                "let missingFeature=a;return typeof missingFeature==='number'?1:0;",
                true,
            ),
            ("let x;return typeof x==='undefined'?1:0;", true),
            ("const kind=typeof x;let x=a;return kind.length;", false),
            ("let x=typeof x;return a;", false),
            ("typeof x;const x=a;return a;", false),
            ("return typeof (0,missingFeature)==='undefined'?1:0;", false),
            ("return typeof missingFeature.x==='undefined'?1:0;", false),
            ("return typeof (missingFeature=a)==='number'?1:0;", false),
            ("return typeof arguments==='object'?1:0;", false),
            ("return typeof Math==='undefined'?1:0;", false),
            ("return typeof Promise==='undefined'?1:0;", false),
            ("return typeof Symbol==='undefined'?1:0;", false),
            ("return typeof globalThis==='undefined'?1:0;", false),
            ("return typeof process==='undefined'?1:0;", false),
            ("return typeof document==='undefined'?1:0;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        let source = "export function run(a:number,b:number){return typeof (missingFeature as number)==='undefined'?1:0;}";
        assert!(crate::compile(source, true).is_ok());
    }
}
