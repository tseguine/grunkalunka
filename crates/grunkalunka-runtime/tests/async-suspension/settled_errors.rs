use super::*;

const CYCLE: &str =
    "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";

#[test]
fn settled_snapshots_preserve_a_thrown_error_reason() {
    compare_promises(
        &[&format!(
            "{CYCLE}try{{await p;}}catch(e){{const h=async x=>{{throw x;}};try{{await h(e);}}catch(x){{return x===e;}}}}"
        )],
        &[(4.0, 2.0)],
    );
}

#[test]
fn settled_snapshots_preserve_runtime_selected_error_reason() {
    compare_promises(
        &[&format!(
            "{CYCLE}try{{await p;}}catch(e){{const h=a?async x=>{{throw x;}}:async x=>{{throw x;}};try{{await h(e);}}catch(x){{return x===e;}}}}"
        )],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn settled_snapshots_distinguish_fulfilled_errors_from_rejected_errors() {
    compare_promises(
        &[&format!(
            "{CYCLE}try{{await p;}}catch(e){{const h=a?async x=>x:async x=>{{throw x;}};try{{const value=await h(e);return a?value===e:false;}}catch(x){{return a?false:x===e;}}}}"
        )],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn settled_snapshots_keep_errors_through_captures_forwarding_and_rethrow() {
    let tails=[
        "const saved=e;const h=a?async()=>saved:async()=>{throw saved;};try{const x=await h();return a?x===e:false;}catch(x){return a?false:x===e;}",
        "const id=x=>x;const h=a?async x=>id(x):async x=>{throw id(x);};try{const x=await h(e);return a?x===e:false;}catch(x){return a?false:x===e;}",
        "const h=a?async x=>{try{throw x;}catch(y){throw y;}}:async x=>{throw x;};try{await h(e);}catch(x){return x===e;}",
        "const h=a?async x=>x:async x=>{throw x;};try{return +(await h(e));}catch(x){return +x;}",
        "const h=a?async x=>{throw x;}:async x=>{throw x;};try{await h(e);}catch(first){try{await h(e);}catch(second){return first===second&&second===e;}}",
    ].map(|tail|format!("{CYCLE}try{{await p;}}catch(e){{{tail}}}"));
    compare_promises(
        &tails.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn settled_snapshots_keep_primitive_payloads_and_status_with_error_extension() {
    let bodies=["undefined","null","false","true","a","b","-0","NaN","Infinity"].map(|value|format!("{CYCLE}try{{await p;}}catch(e){{const h=a?async x=>x:async x=>{{throw x;}};try{{const x=await h({value});return a?x:1000;}}catch(x){{return a?1000:x;}}}}"));
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}
