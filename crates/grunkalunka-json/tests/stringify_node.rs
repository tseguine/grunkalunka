use grunkalunka_json::{JsonValue, Limits, StringifyLimits, parse, stringify};
use std::io::Write;
use std::process::{Command, Stdio};

fn limits() -> StringifyLimits {
    StringifyLimits {
        max_output_units: 262144,
        max_depth: 128,
        max_values: 4096,
        max_work: 1048576,
    }
}

fn node(script: String, expected: String) {
    let mut child = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
}

const PRELUDE: &str = r#"
const assert=require('node:assert/strict'),view=new DataView(new ArrayBuffer(8));
function num(hex){view.setBigUint64(0,BigInt('0x'+hex));return view.getFloat64(0);}
function str(units){let s='';for(let i=0;i<units.length;i+=4096)s+=String.fromCharCode(...units.slice(i,i+4096));return s;}
function units(s){return Array.from({length:s.length},(_,i)=>s.charCodeAt(i));}
"#;

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn raw_binary64_output_matches_node_json_stringify_exactly() {
    let mut inputs = vec![
        0,
        1,
        1 << 63,
        0x7ff0000000000000,
        0xfff0000000000000,
        0x7ff0000000000001,
        0x7ff8000000000000,
        0xfff8000012345678,
        f64::MAX.to_bits(),
        (-f64::MAX).to_bits(),
    ];
    for boundary in [
        1e-7_f64,
        1e-6,
        1e20,
        1e21,
        1e23,
        0.1,
        1.0,
        9007199254740992.0,
        1000000000000000128.0,
    ] {
        for bits in boundary.to_bits() - 1..=boundary.to_bits() + 1 {
            inputs.extend([bits, bits | (1 << 63)]);
        }
    }
    let mut state = 0x4a534f4e5f525955_u64;
    for _ in 0..100000 {
        inputs.push(next(&mut state));
    }
    let records = inputs
        .iter()
        .map(|bits| {
            let output = stringify(&JsonValue::Number(f64::from_bits(*bits)), limits()).unwrap();
            format!("['{bits:016x}',{output:?}]")
        })
        .collect::<Vec<_>>()
        .join(",");
    let message = format!("PASS {} binary64 JSON outputs", inputs.len());
    node(
        format!(
            "{PRELUDE}\nfor(const [hex,expected] of [{records}])assert.deepEqual(units(JSON.stringify(num(hex))),expected,hex);console.log('{message}');"
        ),
        message,
    );
}

fn expression(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".into(),
        JsonValue::Boolean(b) => b.to_string(),
        JsonValue::Number(n) => format!("num('{:016x}')", n.to_bits()),
        JsonValue::String(units) => format!("str({units:?})"),
        JsonValue::Array(values) => format!(
            "[{}]",
            values.iter().map(expression).collect::<Vec<_>>().join(",")
        ),
        // Avoid object-literal __proto__ semantics and preserve creation order.
        JsonValue::Object(entries) => format!(
            "Object.fromEntries([{}])",
            entries
                .iter()
                .map(|(key, value)| format!("[str({key:?}),{}]", expression(value)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

#[test]
fn all_code_units_and_structured_data_match_node_output_units() {
    let mut cases = vec![
        JsonValue::Array(vec![]),
        JsonValue::Object(vec![]),
        JsonValue::String((0..=u16::MAX).collect()),
        JsonValue::String(vec![0xd800, 0xd834, 0xdd1e, 0xdc00, 0xdfff, 0xdbff]),
        JsonValue::Object(
            [
                "a",
                "10",
                "2",
                "01",
                "0",
                "4294967294",
                "4294967295",
                "4294967296",
                "9999999999",
                "-0",
                "-1",
                "+1",
                "1.0",
                "1e0",
                "100000000000000000000",
                "__proto__",
                "toJSON",
                "",
            ]
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key.encode_utf16().collect(), JsonValue::Number(i as f64)))
            .collect(),
        ),
    ];
    for source in [
        "{\"a\":1,\"a\":2,\"2\":-0,\"__proto__\":null}",
        "{\"\\ud800\":1,\"x\":{\"9\":true,\"0\":false}}",
        "[true,false,null,1e400,-1e400,5e-324]",
        "{\"data\":[{\"k\":\"\\u0000\\ud800\"},{}]}",
        "{\"constructor\":1,\"toString\":false,\"valueOf\":null}",
    ] {
        cases.push(
            parse(
                &source.encode_utf16().collect::<Vec<_>>(),
                Limits {
                    max_input_units: 4096,
                    max_depth: 128,
                    max_values: 4096,
                    max_work: 65536,
                },
            )
            .unwrap(),
        );
    }
    let mut state = 0x4a534f4e5f535452_u64;
    for _ in 0..1024 {
        let len = (next(&mut state) % 64) as usize;
        cases.push(JsonValue::String(
            (0..len).map(|_| next(&mut state) as u16).collect(),
        ));
    }
    for _ in 0..128 {
        let mut entries = (0..17)
            .map(|i| {
                let bits = next(&mut state);
                let key = if i % 2 == 0 {
                    i.to_string()
                } else {
                    format!("k{i}")
                };
                (
                    key.encode_utf16().collect(),
                    JsonValue::Array(vec![
                        JsonValue::Number(f64::from_bits(bits)),
                        JsonValue::String(vec![bits as u16]),
                    ]),
                )
            })
            .collect::<Vec<_>>();
        for i in (1..entries.len()).rev() {
            let j = (next(&mut state) % (i + 1) as u64) as usize;
            entries.swap(i, j);
        }
        cases.push(JsonValue::Object(entries));
    }
    let mut nested = JsonValue::Object(vec![(vec![97], JsonValue::Null)]);
    for _ in 0..=64 {
        cases.push(nested.clone());
        nested = JsonValue::Array(vec![nested]);
    }
    let records = cases
        .iter()
        .map(|value| {
            format!(
                "[{}, {:?}]",
                expression(value),
                stringify(value, limits()).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let message = format!("PASS {} structured UTF16 JSON outputs", cases.len());
    node(
        format!(
            "{PRELUDE}\nlet index=0;for(const [value,expected] of [{records}])assert.deepEqual(units(JSON.stringify(value)),expected,String(index++));console.log('{message}');"
        ),
        message,
    );
}
