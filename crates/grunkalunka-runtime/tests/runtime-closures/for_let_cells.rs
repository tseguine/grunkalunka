//! Per-iteration captured lexical environments in the existing numeric ABI.
use super::*;

#[test]
fn captured_for_let_headers_preserve_independent_iteration_cells() {
    compare_functions(&[
        // Reproducer from steering #334: native JS returns 2, while the old
        // shared header pointer incorrectly returned 33.
        "let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){const make=()=>i;if(i===0)first=make;last=make;}return first()*10+last();",
        // A continue must enter the step (and the next iteration's cell)
        // without changing the bindings saved by older closures.
        "let first=()=>a;let last=()=>b;for(let i=0;i<4;i++){const f=()=>i;if(i===0)first=f;if(i===1)continue;last=f;}return first()*10+last();",
        // Updates in the body intentionally affect that iteration's cell;
        // the following iteration copies the value only after the body.
        "let first=()=>a;let last=()=>b;for(let i=0;i<3;){const f=()=>i;if(i===0)first=f;last=f;i++;}return first()*10+last();",
        // A saved closure may mutate its original cell after the loop; it
        // must not mutate other iterations or the loop's final binding.
        "let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){const f=()=>++i;if(i===0)first=f;last=f;}return first()*10+last();",
    ]);
}
