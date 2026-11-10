//! Array construction resumes each element on the shared expression cursor.
use super::*;
use crate::lower::iteration::LiteralElement;

pub(in crate::lower::evaluate) struct ArrayWork<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub elements: Vec<LiteralElement<'e, 'a>>,
    pub array: ValueId,
    pub index: usize,
    pub depth: usize,
    pub awaiting: bool,
}

impl Lowering<'_, '_> {
    pub(super) fn complete_array<'e, 'a>(
        &mut self,
        mut task: ArrayWork<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.awaiting {
            let value = values.pop().expect("array element value").id();
            Self::push(
                regions.current(),
                task.node,
                Op::ManagedArraySetOwn {
                    array: task.array,
                    key: (task.index - 1).to_string().encode_utf16().collect(),
                    value,
                },
            );
        }
        while let Some(element) = task.elements.get(task.index).copied() {
            let (expression, extra) = match element {
                LiteralElement::Hole => {
                    task.index += 1;
                    continue;
                }
                LiteralElement::Undefined => {
                    let primitive = self.value(Type::General);
                    Self::push(
                        regions.current(),
                        task.node,
                        Op::Nullish {
                            result: primitive,
                            value: ir::Nullish::Undefined,
                        },
                    );
                    let value = self.value(Type::ManagedValue);
                    Self::push(
                        regions.current(),
                        task.node,
                        Op::ManagedPrimitive {
                            result: value,
                            input: primitive,
                        },
                    );
                    Self::push(
                        regions.current(),
                        task.node,
                        Op::ManagedArraySetOwn {
                            array: task.array,
                            key: task.index.to_string().encode_utf16().collect(),
                            value,
                        },
                    );
                    task.index += 1;
                    continue;
                }
                LiteralElement::Value(expression, extra) => (expression, extra),
            };
            task.index += 1;
            task.awaiting = true;
            let depth = task.depth + extra;
            work.push(Work::Managed(ManagedWork::Array(task)));
            work.push(Work::Eval(expression, depth, Mode::Managed));
            return Ok(());
        }
        values.push(Value::Array(task.array));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mixed_literal_spreads_preserve_positions_and_erased_annotations() {
        for body in [
            "const xs=[a,...[b,3],4];return xs[0]+xs[1]+xs[2]+xs[3];",
            "const xs=[a,...[],b];return xs.length;",
            "const xs=[,...[,a],,b];return xs.length;",
            "const xs=[a,...[...[b],3],4];return xs[2];",
            "const xs=[a,...[b,b],x=>x+1];return xs[3](a);",
            "const xs=[a,...['x',' y '],b];return xs[2].trim().length;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                assert!(crate::compile(&source, typed).is_ok(), "{source}");
            }
        }
        let js = "export function run(a,b){const xs=[a,...[b,3]];return xs[1];}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[a,...([b,3] as never)];return xs[1];}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
        for source in [
            "export function run(a,b){const src=[a];return [b,...src].length;}",
            "export function run(a,b){return [a,...'x'].length;}",
            "export function run(a,b){const xs=[a,...['x','y']];xs[2]=b;return xs[2].trim().length;}",
        ] {
            for typed in [false, true] {
                assert!(crate::compile(source, typed).is_err(), "{source}");
            }
        }
    }
    #[test]
    fn mixed_literal_spread_depth_is_bounded_on_small_native_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for depth in [16, 129] {
                    let elements = (0..depth).fold("a".to_string(), |s, _| format!("...[{s}]"));
                    let source = format!(
                        "export function run(a,b){{const xs=[b,{elements}];return xs.length;}}"
                    );
                    let result = crate::compile(&source, false);
                    assert_eq!(result.is_ok(), depth == 16, "{depth}: {:?}", result.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
