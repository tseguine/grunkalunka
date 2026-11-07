#[test]
fn returning_switches_accept_scalar_dispatch_and_reject_fallthrough() {
    for (body, admitted) in [
        ("switch(a){case 4:return b;default:return a;}", true),
        ("switch(a){default:return b;case 4:return a;}", true),
        (
            "switch('a'){case 'b':return b;case 'a':return a;default:return 0;}",
            true,
        ),
        (
            "switch(a?true:null){case true:return a;case null:return b;default:return 0;}",
            true,
        ),
        ("switch(a){case 4:return b;}", false),
        ("switch(a){case 4:;default:return a;}", false),
        ("switch(a){case 4:if(b)return b;default:return a;}", false),
        ("switch({}){case null:return b;default:return a;}", true),
        ("switch(a){case {}:return b;default:return a;}", true),
        (
            "let x=a;switch(x){case 0:break;default:return b;}return a;",
            true,
        ),
        (
            "const x=a;switch(x){case 0:return b;default:const x=b;return x;}",
            true,
        ),
        (
            "const x=a;switch(a){case x:return b;default:const x=b;return x;}",
            false,
        ),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}

#[test]
fn switch_case_chains_and_nested_dispatch_keep_native_stack_and_source_budgets() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [254, 255] {
                let cases = (0..count)
                    .map(|i| format!("case {i}:return a;"))
                    .collect::<String>();
                let source =
                    format!("export function run(a,b){{switch(a){{{cases}default:return b;}}}}");
                let result = crate::compile(&source, false);
                if count == 254 {
                    assert!(result.is_ok(), "{:?}", result.err());
                } else {
                    assert!(result.err().unwrap().contains("at most 256 statements"));
                }
            }
            for count in [63, 65] {
                let source = format!(
                    "export function run(a,b){{{}return a;{}}}",
                    "switch(a){default:".repeat(count),
                    "}".repeat(count)
                );
                let result = crate::compile(&source, false);
                if count == 63 {
                    assert!(result.is_ok(), "{:?}", result.err());
                } else {
                    assert!(
                        result
                            .err()
                            .unwrap()
                            .contains("statement nesting exceeds 64")
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn nonexhaustive_switch_preserves_normal_completion_and_terminal_case_guards() {
    for (body, admitted) in [
        ("switch(a){case 0:return b;}return a;", true),
        ("switch((a=b,a)){}return a;", true),
        ("switch(a){case 0:return b;}", false),
        ("switch(a){case 0:a=b;}return a;", true),
        ("switch(a){case 0:;case 1:return b;}return a;", false),
        (
            "const f=x=>{switch(x){case 0:return b;}return x;};return f(a);",
            true,
        ),
        ("switch(a){case 0:{const x=b;return x;}}return x;", false),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}
#[test]
fn final_switch_arms_can_complete_normally_without_enabling_fallthrough() {
    for (body, admitted) in [
        ("switch(a){case 0:a=b;}return a;", true),
        ("switch(a){case 0:return b;default:a=b;}return a;", true),
        ("switch(a){default:return b;case 0:a=b;}return a;", true),
        ("switch(a){case 0:;}return a;", true),
        ("switch(a){default:;}return a;", true),
        ("switch(a){case 0:if(b)return b;}return a;", true),
        ("switch(a){case 0:a=b;case 1:return a;}return b;", false),
        ("switch(a){default:a=b;case 0:return a;}return b;", false),
        ("while(a){switch(b){case 0:continue;}}return b;", false),
        ("while(a){switch(b){default:break;}}return b;", true),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}

#[test]
fn switch_reference_proofs_and_guards() {
    for (body, admitted) in [
        (
            "const x={};switch(x){case x:return a;default:return b;}",
            true,
        ),
        (
            "const f=()=>a;switch(f){case f:return a;default:return b;}",
            true,
        ),
        (
            "const f=async()=>a;switch(f){case f:return a;}return b;",
            true,
        ),
        ("const x=a?{}:[];switch(x){case x:return a;}return b;", true),
        ("switch(a?{}:null){default:return a;}", true),
        (
            "const x={};switch(x){case a?x:null:return b;}return a;",
            true,
        ),
        ("const x={};switch(x){case x:break;}return a;", true),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}

#[test]
fn mixed_switch_values_need_positive_source_proofs() {
    for (body, admitted) in [
        (
            "switch(a?'x':null){case null:return a;default:return b;}",
            true,
        ),
        (
            "const f=()=>a;switch(a?f:false){case f:return a;default:return b;}",
            true,
        ),
        (
            "const xs=[{},'x',null,a];switch(xs[0]){case xs[1]:return a;default:return b;}",
            true,
        ),
        ("switch(Promise.resolve(a)){default:return b;}", false),
        (
            "switch(a){case Promise.resolve(b):return a;default:return b;}",
            false,
        ),
        ("switch(a?{}:missing){default:return b;}", false),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}

#[test]
fn switch_breaks_are_normal_exits_without_case_fallthrough() {
    for (body, admitted) in [
        ("switch(a){case 0:a=b;break;default:a++;}return a;", true),
        (
            "switch(a){case 0:if(b)break;else return a;default:return b;}return a;",
            true,
        ),
        (
            "switch(a){case 0:if(b)break;default:return b;}return a;",
            false,
        ),
        ("switch(a){case 0:break;default:break;}", false),
        (
            "switch(a){case 0:break;default:break;}break;return a;",
            false,
        ),
        ("switch(a){case 0:continue;}return a;", false),
        ("while(a){switch(b){case 0:continue;}}return b;", false),
        (
            "const f=x=>{switch(x){case 0:break;default:return x;}return b;};return f(a);",
            true,
        ),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            if let Err(error) = result {
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}

#[test]
fn switch_break_targets_keep_small_stack_and_case_limits() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [254, 255] {
                let cases = (0..count)
                    .map(|i| format!("case {i}:break;"))
                    .collect::<String>();
                let source = format!("export function run(a,b){{switch(a){{{cases}}}return b;}}");
                let result = crate::compile(&source, false);
                if count == 254 {
                    assert!(result.is_ok(), "{:?}", result.err());
                } else {
                    assert!(result.err().unwrap().contains("at most 256 statements"));
                }
            }
            for count in [63, 65] {
                let source = format!(
                    "export function run(a,b){{{}break;{}return b;}}",
                    "switch(a){default:".repeat(count),
                    "}".repeat(count)
                );
                let result = crate::compile(&source, false);
                if count == 63 {
                    assert!(result.is_ok(), "{:?}", result.err());
                } else {
                    assert!(
                        result
                            .err()
                            .unwrap()
                            .contains("statement nesting exceeds 64")
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
