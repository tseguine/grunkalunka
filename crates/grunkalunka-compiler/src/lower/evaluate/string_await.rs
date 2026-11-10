//! Await preserves a boxed primitive value across a real reaction job.
use super::managed::ManagedWork;
use super::*;

impl Lowering<'_, '_> {
    pub(super) fn schedule_boxed_primitive_await<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::AwaitExpression(awaited) = node else {
            return Ok(false);
        };
        if !matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard)
            || !self
                .stored
                .boxed_scalar_with_string(awaited.argument.node_id())
        {
            return Ok(false);
        }
        if !self.allow_await {
            return Err(unsupported(node, "await requires the async profile"));
        }
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else if matches!(mode, Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
        }
        work.push(Work::BoxedPrimitiveAwait(node));
        work.push(Work::Eval(&awaited.argument, depth + 1, Mode::Managed));
        Ok(true)
    }

    pub(super) fn resume_boxed_primitive_await(
        &mut self,
        node: &Expression<'_>,
        regions: &mut Regions,
        values: &[Value],
    ) {
        // Keep the original boxed value live across the suspension. Its possible
        // kinds are all primitive, so Promise resolution cannot call user code
        // or replace its value. A fulfilled numeric token provides the same
        // one-reaction-job suspension through the existing private await ABI;
        // its resumed value is deliberately unused. The source operand above
        // still evaluates exactly once, and may throw or suspend itself.
        debug_assert!(matches!(values.last(), Some(Value::Managed(_))));
        let input = self.value(Type::Number);
        Self::push(
            regions.current(),
            node,
            Op::Constant {
                result: input,
                value: 0.0,
            },
        );
        let result = self.value(Type::Number);
        Self::push_await(
            regions.current(),
            node,
            None,
            Op::Await {
                result,
                input,
                rejected: false,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn discarded_string_awaits_keep_the_existing_site_budget() {
        for (count, admitted) in [(32, true), (33, false)] {
            let source = format!(
                "export async function run(a,b){{{}return a;}}",
                "await 'text';".repeat(count)
            );
            let result = crate::compile_async(&source, false);
            assert_eq!(result.is_ok(), admitted, "{count}: {:?}", result.err());
            if let Err(error) = result {
                assert!(error.contains("32 await sites"), "{error}");
            }
        }
    }

    #[test]
    fn string_awaits_require_positive_primitive_origins() {
        for (body, admitted) in [
            ("const text=await 'abc';return text.length;", true),
            ("return (await (await 'abc')).length;", true),
            ("let text='x';text+=await 'y';return text.length;", true),
            ("const o={s:'x'};return (o.s+=await 'y').length;", true),
            ("if(await '')return a;return b;", true),
            ("const text=await a;return text.length;", false),
            ("const text=await (a?'abc':b);return text.length;", false),
            (
                "const text=await {then:resolve=>resolve('abc')};return text.length;",
                false,
            ),
            ("const o=await {s:'abc'};return o.s.length;", false),
            (
                "const input=a?'x':{then:r=>r('y')};const value=await input;return value==='x'?1:0;",
                false,
            ),
            (
                "const text=await Promise.resolve('abc');return text.length;",
                false,
            ),
            ("const text=await 'abc';return text+b;", false),
            (
                "const get=async()=>{await a;return 'abc';};const text=await get();return text.length;",
                false,
            ),
        ] {
            let source = format!("export async function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile_async(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
                if let Err(error) = result {
                    assert!(!error.contains("internal IR"), "{source}: {error}");
                }
            }
        }
        assert!(crate::compile_async("export async function run(a,b){const text=await (a as unknown as string);return text.length;}", true).is_err());
        assert!(
            crate::compile(
                "export function run(a,b){return (await 'abc').length;}",
                false
            )
            .is_err()
        );
    }
}
