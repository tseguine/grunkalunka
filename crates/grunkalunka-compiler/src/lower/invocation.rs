//! Original-source ordinary call arguments and represented Function.call/apply.
//! Intrinsic recognition still inspects the original call, not this projection.
use super::*;
use oxc_ast::ast::Argument;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Method {
    Call,
    Apply,
}

fn method<'e, 'a>(call: &'e CallExpression<'a>) -> Option<(&'e Expression<'a>, usize, Method)> {
    let (callee, depth) = closures::unwrapped_at(&call.callee, 0)?;
    let Expression::StaticMemberExpression(member) = callee else {
        return None;
    };
    if member.optional {
        return None;
    }
    let kind = if member.property.name == "call" {
        Method::Call
    } else if member.property.name == "apply" {
        // The original literal Array is newly constructed and unobservable
        // after this call. Its ordinary element evaluation can use the same
        // source-ordered argument cursor as a fixed literal spread.
        if call.arguments.len() != 2 {
            return None;
        }
        let (array, _) = closures::unwrapped_at(call.arguments[1].as_expression()?, 0)?;
        if !matches!(array, Expression::ArrayExpression(_)) {
            return None;
        }
        Method::Apply
    } else {
        return None;
    };
    Some((&member.object, depth + 1, kind))
}

pub(super) fn callee<'e, 'a>(call: &'e CallExpression<'a>) -> &'e Expression<'a> {
    method(call).map_or(&call.callee, |(callee, _, _)| callee)
}

pub(super) fn depth(call: &CallExpression<'_>, depth: usize) -> usize {
    depth + method(call).map_or(0, |(_, extra, _)| extra)
}

pub(super) fn offset(call: &CallExpression<'_>) -> usize {
    usize::from(is_method(call) && !call.arguments.is_empty())
}

pub(super) fn is_method(call: &CallExpression<'_>) -> bool {
    method(call).is_some()
}

#[derive(Clone, Copy)]
pub(super) struct Actual<'n, 'a> {
    expression: Option<&'n Expression<'a>>,
    span: oxc_span::Span,
    pub(super) depth: usize,
}
impl<'n, 'a> Actual<'n, 'a> {
    pub(super) fn as_expression(self) -> Option<&'n Expression<'a>> {
        self.expression
    }
}
impl GetSpan for Actual<'_, '_> {
    fn span(&self) -> oxc_span::Span {
        self.span
    }
}
pub(super) struct Arguments<'n, 'a>(Vec<Actual<'n, 'a>>);
impl<'n, 'a> Arguments<'n, 'a> {
    pub(super) fn first(&self) -> Option<Actual<'n, 'a>> {
        self.get(0)
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn get(&self, index: usize) -> Option<Actual<'n, 'a>> {
        self.0.get(index).copied()
    }
    pub(super) fn iter(self) -> std::vec::IntoIter<Actual<'n, 'a>> {
        self.0.into_iter()
    }
}
impl<'n, 'a> IntoIterator for Arguments<'n, 'a> {
    type Item = Actual<'n, 'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// A literal Array is fresh and its intrinsic iterator has no user hooks in the
// admitted domain. Borrow its original expressions; never synthesize an AST.
// Opaque spreads and elisions remain explicit unsupported entries.
pub(super) fn all_arguments<'n, 'a>(call: &'n CallExpression<'a>) -> Arguments<'n, 'a> {
    use oxc_ast::ast::ArrayExpressionElement;
    let mut result = Vec::new();
    let is_apply = matches!(method(call), Some((_, _, Method::Apply)));
    let mut pending = call
        .arguments
        .iter()
        .enumerate()
        .rev()
        .map(|(index, arg)| {
            (
                arg.as_expression(),
                if is_apply && index == 1 {
                    arg.as_expression()
                } else if let Argument::SpreadElement(s) = arg {
                    Some(&s.argument)
                } else {
                    None
                },
                arg.span(),
                0usize,
                is_method(call) && index == 0,
            )
        })
        .collect::<Vec<_>>();
    while let Some((expression, spread, span, depth, prefix)) = pending.pop() {
        if let Some(source) = spread.filter(|_| !prefix)
            && let Some((Expression::ArrayExpression(array), next)) =
                closures::unwrapped_at(source, depth)
            && next < 128
        {
            pending.extend(array.elements.iter().rev().map(|element| {
                (
                    element.as_expression(),
                    if let ArrayExpressionElement::SpreadElement(s) = element {
                        Some(&s.argument)
                    } else {
                        None
                    },
                    element.span(),
                    next + 1,
                    false,
                )
            }));
            continue;
        }
        result.push(Actual {
            expression,
            span,
            depth,
        });
        if result.len() > 1000 {
            break;
        }
    }
    Arguments(result)
}
pub(super) fn arguments<'n, 'a>(call: &'n CallExpression<'a>) -> Arguments<'n, 'a> {
    let mut all = all_arguments(call);
    if offset(call) != 0 {
        all.0.remove(0);
    }
    all
}

#[cfg(test)]
mod tests {
    #[test]
    fn literal_call_spreads_cover_named_inline_and_runtime_calls() {
        for source in [
            "function f(x){return x;}export function run(a,b){return f(...[a]);}",
            "export function run(a,b){let f=x=>x;return f(...[a]);}",
            "function f(x,y){return x+y;}export function run(a,b){return f(...[a,b]);}",
            "export function run(a,b){const f=(x,y)=>x+y;return f(...[a],b);}",
            "export function run(a,b){const o={f:(x,y)=>x+y};return o.f(...[a,b]);}",
            "function f(x=3){return x;}export function run(a,b){return +f(...[]);}",
            "function f(x,y){return x+y;}export function run(a,b){return f.call(null,...[a,b]);}",
            "function f(x,y){return x+y;}export function run(a,b){return f(...[...[a],b]);}",
        ] {
            for ts in [false, true] {
                let r = crate::compile(source, ts);
                assert!(r.is_ok(), "{source}: {:?}", r.err());
            }
        }
        for source in [
            "function f(x){return x;}export function run(a,b){const xs=[a];return f(...xs);}",
            "function f(x){return x;}export function run(a,b){return f(...[,]);}",
            "function f(x){return x;}export function run(a,b){return f(...'abc');}",
            "function f(x){return x;}export function run(a,b){return f.call(...[null,a]);}",
        ] {
            assert!(crate::compile(source, false).is_err(), "{source}");
        }
        let js = "function f(x,y){return x+y;}export function run(a,b){return f(...[a,b]);}";
        let ts = "function f(x:string,y:boolean):never{return x+y;}export function run(a:string,b:boolean):never{return f(...([a,b] as never));}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
    #[test]
    fn literal_call_spread_limits_are_controlled_on_small_native_stacks() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(||{
            for count in [1000,1001] {
                let args=vec!["a";count].join(",");
                let source=format!("function f(x){{return x;}}export function run(a,b){{return f(...[{args}]);}}");
                assert_eq!(crate::compile(&source,false).is_ok(),count==1000,"{count}");
            }
            for depth in [16,129] {
                let source=(0..depth).fold("a".to_string(),|s,_|format!("...[{s}]"));
                let source=format!("function f(x){{return x;}}export function run(a,b){{return f({source});}}");
                assert_eq!(crate::compile(&source,false).is_ok(),depth==16,"{depth}");
            }
        }).unwrap().join().unwrap();
    }
}
