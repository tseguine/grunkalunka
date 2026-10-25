use grunkalunka_json::{ErrorKind, JsonValue, Limits, parse};
use std::io::Write;
use std::process::{Command, Stdio};

fn snapshot(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "[0]".into(),
        JsonValue::Boolean(b) => format!("[1,{}]", u8::from(*b)),
        JsonValue::Number(n) => format!("[2,\"{:016x}\"]", n.to_bits()),
        JsonValue::String(units) => format!("[3,{units:?}]"),
        JsonValue::Array(values) => format!(
            "[4,[{}]]",
            values.iter().map(snapshot).collect::<Vec<_>>().join(",")
        ),
        JsonValue::Object(entries) => {
            // Structural comparison ignores enumeration order; dedicated tests pin first insertion.
            let mut sorted = entries.iter().collect::<Vec<_>>();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            format!(
                "[5,[{}]]",
                sorted
                    .iter()
                    .map(|(key, value)| format!("[{key:?},{}]", snapshot(value)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

#[test]
fn native_parser_matches_node_json_parse_on_original_utf16_sources() {
    let valid = [
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1e400",
        "-1e400",
        "1e-4000",
        "5e-324",
        "\"\"",
        "\"\\ud800\"",
        "\"\\ud834\\udd1e\"",
        "\"a\\u0000b\"",
        "{\"a\":1,\"a\":2}",
        "{\"a\":1,\"10\":\"ten\",\"2\":\"two\",\"01\":\"leading\",\"a\":2}",
        "{\"__proto__\":{\"polluted\":true},\"constructor\":1}",
        "{\"data\":[true,false,null,0,-0,\"x\",{\"a\":2}]}",
        "\t\r\n [1,2] ",
        "{\"\\ud800\":1,\"\\ud800\":2}",
        "\"\\/\\b\\f\\n\\r\\t\\\\\\\"\"",
    ];
    let invalid = [
        "",
        "undefined",
        "NaN",
        "Infinity",
        "01",
        "+1",
        ".5",
        "1.",
        "[1,]",
        "{\"x\":1,}",
        "{'x':1}",
        "{x:1}",
        "/*x*/0",
        "\u{a0}0",
        "\"\\uD80\"",
        "\"x\ny\"",
        "-01",
        "1e",
        "1e+",
        "--1",
        "0x1",
        "1_0",
        "[1 2]",
        "[,1]",
        "{\"a\"1}",
        "{\"a\":}",
        "true false",
        "\"x\"true",
    ];
    let mut cases = valid
        .iter()
        .map(|s| (s.encode_utf16().collect::<Vec<_>>(), true))
        .collect::<Vec<_>>();
    cases.extend(
        invalid
            .iter()
            .map(|s| (s.encode_utf16().collect::<Vec<_>>(), false)),
    );
    cases.push((vec![34, 0xd800, 34], true));
    // Long decimal conversion and boundary rounding must not use an integer-sized
    // intermediate or treat syntactically valid overflow as an invalid number.
    for text in [
        "9".repeat(3000),
        format!("-0.{}1", "0".repeat(3000)),
        format!("1e+{}400", "0".repeat(1000)),
        "1.7976931348623157e308".into(),
        "1.7976931348623159e308".into(),
        "2.4703282292062327e-324".into(),
        "2.4703282292062328e-324".into(),
        "\"\u{2028}\u{2029}\u{ffff}\"".into(),
    ] {
        cases.push((text.encode_utf16().collect(), true));
    }
    for unit in 0..32 {
        cases.push((vec![34, unit, 34], false));
    }
    let mut state = 0x4a534f4e5f555446_u64;
    for _ in 0..2048 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let sign = if state & 1 == 0 { "" } else { "-" };
        let exponent = (state % 801) as i32 - 400;
        let text = format!(
            "{sign}{}.{}e{exponent:+}",
            state % 1_000_000_000 + 1,
            state.rotate_left(23) % 1_000_000_000
        );
        cases.push((text.encode_utf16().collect(), true));
    }
    for count in 0..=64 {
        let text = format!(
            "{}{{\"a\":\"\\ud800\",\"a\":-0}}{}",
            "[".repeat(count),
            "]".repeat(count)
        );
        cases.push((text.encode_utf16().collect(), true));
    }
    let mut records = Vec::new();
    for (units, valid) in &cases {
        let result = parse(
            units,
            Limits {
                max_input_units: 4096,
                max_depth: 128,
                max_values: 1024,
                max_work: 16384,
            },
        );
        if *valid {
            records.push(format!(
                "{{units:{units:?},expected:{}}}",
                snapshot(&result.unwrap())
            ));
        } else {
            assert_eq!(result.unwrap_err().kind, ErrorKind::Syntax, "{units:?}");
            records.push(format!("{{units:{units:?},invalid:true}}"));
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function units(s){{return Array.from({{length:s.length}},(_,i)=>s.charCodeAt(i));}}
function snapshot(v){{if(v===null)return[0];if(typeof v==='boolean')return[1,Number(v)];if(typeof v==='number'){{view.setFloat64(0,v);return[2,view.getBigUint64(0).toString(16).padStart(16,'0')];}}if(typeof v==='string')return[3,units(v)];if(Array.isArray(v))return[4,v.map(snapshot)];return[5,Object.keys(v).sort().map(key=>[units(key),snapshot(v[key])])];}}
for(const c of cases){{const text=String.fromCharCode(...c.units);if(c.invalid)assert.throws(()=>JSON.parse(text),SyntaxError);else assert.deepEqual(snapshot(JSON.parse(text)),c.expected,JSON.stringify(c.units));}}
console.log('PASS '+cases.length+' original UTF16 JSON sources');
"#,
        records.join(",")
    );
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
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("PASS {} original UTF16 JSON sources", cases.len())
    );
}
