//! Direct structured Wasm emission. Every computation and source load is
//! materialized immediately: later source-cell stores cannot change a value.
use std::fmt::Write;

use crate::ir::{self, Binary, Compare, Exit, Function, Op, Region, Type, Unary, ValueId};

struct Emitter<'a> {
    function: &'a Function,
    output: String,
    break_label: Option<usize>,
}

impl Emitter<'_> {
    fn get(&mut self, value: ValueId) {
        writeln!(
            self.output,
            "local.get {}",
            self.function.slots.len() + value.0
        )
        .unwrap();
    }
    fn set(&mut self, value: ValueId) {
        writeln!(
            self.output,
            "local.set {}",
            self.function.slots.len() + value.0
        )
        .unwrap();
    }

    fn region(&mut self, region: &Region) {
        for operation in &region.operations {
            match &operation.kind {
                Op::Not { result, input } => {
                    self.get(*input);
                    self.output.push_str("i32.eqz\n");
                    self.set(*result);
                }
                Op::Boolean { result, value } => {
                    writeln!(self.output, "i32.const {}", i32::from(*value)).unwrap();
                    self.set(*result);
                }
                Op::DoWhile { body, condition } => {
                    let label = body.id.0;
                    writeln!(self.output, "block $exit{label}\nloop $do{label}").unwrap();
                    let outer = self.break_label.replace(label);
                    self.region(body);
                    self.break_label = outer;
                    self.region(condition);
                    writeln!(self.output, "br_if $do{label}\nend\nend").unwrap();
                }
                Op::Constant { result, value } => {
                    let literal = if value.is_nan() {
                        "nan".to_string()
                    } else if *value == f64::INFINITY {
                        "inf".to_string()
                    } else if *value == f64::NEG_INFINITY {
                        "-inf".to_string()
                    } else {
                        value.to_string()
                    };
                    writeln!(self.output, "f64.const {literal}").unwrap();
                    self.set(*result);
                }
                Op::Load { result, slot } => {
                    writeln!(self.output, "local.get {}", slot.0).unwrap();
                    self.set(*result);
                }
                Op::Initialize { slot, value } | Op::Store { slot, value } => {
                    self.get(*value);
                    writeln!(self.output, "local.set {}", slot.0).unwrap();
                }
                Op::Unary {
                    result,
                    operator,
                    input,
                } => {
                    self.get(*input);
                    if matches!(operator, Unary::Negate) {
                        self.output.push_str("f64.neg\n");
                    }
                    self.set(*result);
                }
                Op::Binary {
                    result,
                    operator,
                    left,
                    right,
                } => {
                    self.get(*left);
                    self.get(*right);
                    self.output.push_str(match operator {
                        Binary::Add => "f64.add\n",
                        Binary::Subtract => "f64.sub\n",
                        Binary::Multiply => "f64.mul\n",
                        Binary::Divide => "f64.div\n",
                    });
                    self.set(*result);
                }
                Op::Compare {
                    result,
                    operator,
                    left,
                    right,
                } => {
                    self.get(*left);
                    self.get(*right);
                    self.output.push_str(match operator {
                        Compare::Less => "f64.lt\n",
                        Compare::LessEqual => "f64.le\n",
                        Compare::Greater => "f64.gt\n",
                        Compare::GreaterEqual => "f64.ge\n",
                        Compare::Equal => "f64.eq\n",
                        Compare::NotEqual => "f64.ne\n",
                    });
                    self.set(*result);
                }
                Op::Truthy { result, input } => {
                    self.get(*input);
                    // abs(x) > 0 is false exactly for +/-0 and NaN.
                    self.output.push_str("f64.abs\nf64.const 0\nf64.gt\n");
                    self.set(*result);
                }
                Op::Block(child) => self.region(child),
                Op::If {
                    result,
                    condition,
                    consequent,
                    alternate,
                } => {
                    self.get(*condition);
                    self.output
                        .push_str(match result.map(|id| self.function.values[id.0]) {
                            Some(Type::Number) => "if (result f64)\n",
                            Some(Type::Predicate) => "if (result i32)\n",
                            None => "if\n",
                        });
                    self.region(consequent);
                    self.output.push_str("else\n");
                    self.region(alternate);
                    self.output.push_str("end\n");
                    if let Some(result) = result {
                        self.set(*result);
                    }
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    let label = condition.id.0;
                    writeln!(self.output, "block $exit{label}\nloop $loop{label}").unwrap();
                    // The entire condition region executes on every visit, including
                    // the final failed test. Its values never escape to the body or step.
                    self.region(condition);
                    writeln!(self.output, "i32.eqz\nbr_if $exit{label}").unwrap();
                    let outer = self.break_label.replace(label);
                    self.region(body);
                    self.break_label = outer;
                    if let Some(step) = step {
                        self.region(step);
                    }
                    writeln!(self.output, "br $loop{label}\nend\nend").unwrap();
                }
            }
        }
        match region.exit {
            Exit::Fallthrough => {}
            Exit::Break => {
                writeln!(
                    self.output,
                    "br $exit{}",
                    self.break_label.expect("verified loop break")
                )
                .unwrap();
            }
            Exit::Yield(value) => self.get(value),
            Exit::Return(value) => {
                self.get(value);
                self.output.push_str("return\n");
            }
        }
    }
}

pub(crate) fn emit(function: &Function) -> Result<String, String> {
    ir::verify(function)?;
    let mut emitter = Emitter {
        function,
        break_label: None,
        output: "(module (func (export \"run\") (param f64 f64) (result f64)\n".to_string(),
    };
    // Source cells and typed value snapshots occupy disjoint physical locals.
    for _ in function.slots.iter().skip(2) {
        emitter.output.push_str("(local f64)\n");
    }
    for ty in &function.values {
        emitter.output.push_str(match ty {
            Type::Number => "(local f64)\n",
            Type::Predicate => "(local i32)\n",
        });
    }
    emitter.region(&function.body);
    // The verifier proved return coverage. This only helps Wasm validation after
    // nested void branches whose arms return; it supplies no source semantics.
    emitter.output.push_str("unreachable\n))\n");
    Ok(emitter.output)
}
