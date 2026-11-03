//! Disposable closure checks must resume their caller on the source cursor,
//! publishing checked metadata and source costs without publishing scratch IR.
use super::BRIDGED_CLOSURE_CHECKS;
use crate::ir::{FunctionId, Op, SlotId, ValueId};

fn on_small_stack(body: &str, asynchronous: bool, typescript: bool) -> (Result<(), String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_CLOSURE_CHECKS.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            let bridged = BRIDGED_CLOSURE_CHECKS.with(std::cell::Cell::get);
            (result, bridged)
        })
        .expect("2 MiB closure checking thread")
        .join()
        .expect("closure checking must not exhaust the native stack")
}

#[test]
fn ordinary_unchecked_callees_share_the_source_scheduler() {
    for body in [
        "return(x=>x+a)(b);",
        "return(x=>{return(y=>y+x)(b);})(a);",
        "return(x=>{return(y=>{if(y)return y+x;return x;})(b);})(a);",
        "return((fn,x)=>fn(x))(x=>x+a,b);",
        "return(o=>o.x===a?a:b)({x:a});",
        "return +(x=>x?null:b)(a);",
        "return(x=>x>a)(b)?a:b;",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, false, typescript);
            assert!(result.is_ok(), "{body}: {result:?}");
            assert!(
                bridged > 0,
                "unchecked callee bypassed the source cursor: {body}"
            );
        }
    }
}

#[test]
fn awaited_unchecked_callees_share_the_source_scheduler() {
    for body in [
        "return await(async x=>{await b;return x+a;})(b);",
        "return await(async o=>{await b;return o.x===a?a:b;})({x:a});",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, true, typescript);
            assert!(result.is_ok(), "{body}: {result:?}");
            assert!(
                bridged > 0,
                "unchecked awaited callee bypassed the source cursor: {body}"
            );
        }
    }
}

#[test]
fn unchecked_callees_preserve_validation_and_first_error_order() {
    for (asynchronous, body, expected) in [
        (
            false,
            "return(x=>Math.noSuch(x))(Math.noOther(a));",
            "unsupported at bytes 35..49: expression is outside the numeric experiment",
        ),
        (
            false,
            "return((x,y)=>Math.noSuch(x))(a);",
            "unsupported at bytes 39..53: expression is outside the numeric experiment",
        ),
        (
            false,
            "return((x,y)=>x+y)(a);",
            "unsupported at bytes 31..46: local call is missing declared arguments",
        ),
        (
            false,
            "return(x=>Math.noSuch(x))(...a);",
            "unsupported at bytes 35..49: expression is outside the numeric experiment",
        ),
        (
            false,
            "return(x=>x+a)(...b);",
            "unsupported at bytes 31..45: spread call arguments are not admitted",
        ),
        (
            false,
            "return(x=>{const y=y;return x;})(a);",
            "unsupported at bytes 44..45: reference is not an initialized numeric parameter or local",
        ),
        (
            false,
            "let make=x=>()=>x;return make((x=>Math.noSuch(x))(a))();",
            "unsupported at bytes 59..73: expression is outside the numeric experiment",
        ),
        (
            true,
            "return await(async x=>{await b;return Math.noSuch(x);})(Math.noOther(a));",
            "unsupported at bytes 69..83: expression is outside the numeric experiment",
        ),
        (
            true,
            "return await(async(x,y)=>{await b;return x+y;})(a);",
            "unsupported at bytes 43..81: local call is missing declared arguments",
        ),
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, asynchronous, typescript);
            assert_eq!(
                result.unwrap_err(),
                expected,
                "{body}, typescript={typescript}"
            );
            assert!(
                bridged > 0,
                "failed check bypassed the source cursor: {body}"
            );
        }
    }
}

#[test]
fn checking_children_preserve_existing_block_admission_without_a_new_child_limit() {
    for (block, depth, admitted) in [(true, 8, true), (true, 9, false), (false, 10, true)] {
        let mut expression = String::from("a");
        for index in 0..depth {
            expression = if block {
                format!("(x{index}=>{{return {expression};}})(b)")
            } else {
                format!("(x{index}=>{expression})(b)")
            };
        }
        let body = format!("return {expression};");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            assert_eq!(
                result.is_ok(),
                admitted,
                "block={block}, depth={depth}: {result:?}"
            );
            if let Err(error) = result {
                assert_eq!(
                    error,
                    "unsupported at bytes 137..152: at most 8 nested block-arrow calls are admitted"
                );
            }
            assert!(bridged > 0, "nested checks bypassed the source cursor");
        }
    }
}

#[test]
fn checking_children_publish_only_referenced_captures_and_real_values() {
    for typescript in [false, true] {
        let module =
            crate::lower_source("export function run(a,b){return(x=>x+a)(b);}", typescript)
                .unwrap();
        assert_eq!(
            module.functions.len(),
            1,
            "scratch bodies cannot become lifted functions"
        );
        let entry = &module.functions[0];
        assert_eq!(
            entry.slots.len(),
            3,
            "only the real parameter slot is published"
        );
        assert_eq!(
            entry.values.len(),
            4,
            "synthetic definition inputs must be discarded"
        );
        assert!(matches!(
            &entry.body.operations[0].kind,
            Op::CaptureCells { cells } if cells == &[SlotId(0)]
        ));
        assert!(
            matches!(
                entry.body.operations[1].kind,
                Op::Load {
                    result: ValueId(0),
                    slot: SlotId(1)
                }
            ),
            "the first real value must still be the argument snapshot"
        );
    }
}

#[test]
fn scratch_lifted_bodies_do_not_shift_real_function_identities() {
    for typescript in [false, true] {
        let module = crate::lower_source(
            "export function run(a,b){return(x=>{let unused=()=>x;return x;})(a);}",
            typescript,
        )
        .unwrap();
        assert_eq!(
            module.functions.len(),
            2,
            "the scratch closure must be discarded"
        );
        let mut lifted = Vec::new();
        crate::ir::visit_operations(&module.functions[0].body, &mut |op| {
            if let Op::MakeClosure { function, .. } = op {
                lifted.push(*function);
            }
        });
        assert_eq!(
            lifted,
            [FunctionId(1)],
            "real lifting follows the original identity order"
        );
    }
}
