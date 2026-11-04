//! Source-flow observations independent of whether the runtime admits a body.
use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    string: bool,
    container: Option<Type>,
    callable: bool,
    other: bool,
}

const STRING: Observation = Observation {
    string: true,
    container: None,
    callable: false,
    other: false,
};
const UNKNOWN: Observation = Observation {
    string: false,
    container: None,
    callable: false,
    other: true,
};

fn observe_source(source: &str) -> Observation {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(parsed.diagnostics.is_empty(), "{source}");
    let built = SemanticBuilder::new_compiler()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert!(built.diagnostics.is_empty(), "{source}");
    let entry = built
        .semantic
        .nodes()
        .iter()
        .find_map(|node| match node.kind() {
            AstKind::Function(function) => function
                .id
                .as_ref()
                .filter(|name| name.name == "run")
                .and_then(|name| name.symbol_id.get()),
            _ => None,
        })
        .expect("one run entry");
    let observations = built
        .semantic
        .nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::VariableDeclarator(declaration)
                if matches!(&declaration.id, BindingPattern::BindingIdentifier(name) if name.name == "observed") =>
            {
                declaration.init.as_ref().map(GetNodeId::node_id)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 1, "one observed initializer: {source}");
    let observed = observations[0];
    let fields = Fields::collect(&built.semantic, entry);
    Observation {
        string: fields.string(observed),
        container: fields.container(observed),
        callable: fields.functions(observed).is_some(),
        other: fields
            .values
            .get(&observed)
            .is_some_and(|origins| origins.contains(&Origin::Other)),
    }
}

fn observe(body: &str) -> Observation {
    observe_source(&format!("export function run(a,b){{{body}}}"))
}

#[test]
fn iteration_keeps_literal_kinds_and_nonempty_branch_values() {
    for body in [
        "for(const item of ['first','second']){const observed=item;}return a;",
        "for(const item of (a?[]:['text'])){const observed=item;}return a;",
        "for(const item of (a?['text']:[])){const observed=item;}return a;",
        "const row={values:['text']};for(const item of row.values){const observed=item;}return a;",
        "for(const row of [['text'],[]])for(const item of row){const observed=item;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for (body, ty) in [
        (
            "for(const item of [{label:'text'},{}]){const observed=item;}return a;",
            Type::ManagedObject,
        ),
        (
            "for(const item of [[a],[b],[]]){const observed=item;}return a;",
            Type::ManagedArray,
        ),
    ] {
        assert_eq!(
            observe(body),
            Observation {
                string: false,
                container: Some(ty),
                callable: false,
                other: false,
            },
            "{body}"
        );
    }
}

#[test]
fn iteration_keeps_replacements_and_ignores_nonindex_properties() {
    for prefix in [
        "items[0]='replacement';",
        "const alias=items;alias[1]='replacement';",
        "const identity=x=>x;identity(items)[0]='replacement';",
        "const replace=()=>{items[1]='replacement';};replace();",
        "const replace=(target,value)=>{target[0]=value;return target;};replace(items,'replacement');",
        "items.named=a;items['01']=b;items[4294967295]=a;",
    ] {
        let body = format!(
            "const items=['first','second'];{prefix}for(const item of items){{const observed=item;}}return a;"
        );
        assert_eq!(observe(&body), STRING, "{body}");
    }
    assert_eq!(
        observe(
            "const items=[{label:'first'}];items[0]={label:'second'};for(const item of items){const observed=item.label;}return a;"
        ),
        STRING,
    );
}

#[test]
fn iteration_values_flow_through_helper_inputs_results_and_captures() {
    for source in [
        "function identity(value){return value;}export function run(a,b){for(const item of ['text']){const observed=identity(item);}return a;}",
        "function source(values){return values;}export function run(a,b){for(const item of source(['text'])){const observed=item;}return a;}",
        "function inspect(values){for(const item of values){const observed=item;}return 0;}export function run(a,b){return inspect(a?[]:['text']);}",
        "export function run(a,b){const items=['text'];const source=()=>items;for(const item of source()){const read=()=>item;const observed=read();}return a;}",
        "export function run(a,b){for(let item of ['text']){const change=()=>{item='replacement';};change();const read=()=>item;const observed=read();}return a;}",
    ] {
        assert_eq!(observe_source(source), STRING, "{source}");
    }
}

#[test]
fn iteration_retains_unknown_for_holes_and_structural_mutations() {
    for body in [
        "for(const item of ['text',,]){const observed=item;}return a;",
        "for(const item of ['text',undefined]){const observed=item;}return a;",
        "for(const item of (a?['text']:b)){const observed=item;}return a;",
        "const items=['text'];items.length=1;for(const item of items){const observed=item;}return a;",
        "const items=['text'];items.length=0;for(const item of items){const observed=item;}return a;",
        "const items=['text'];items.length=2;for(const item of items){const observed=item;}return a;",
        "const items=['first','second'];items.pop();for(const item of items){const observed=item;}return a;",
        "const items=['text'];items[1]='next';for(const item of items){const observed=item;}return a;",
        "const items=['text'];items[3]='later';for(const item of items){const observed=item;}return a;",
        "const items=['text'];items[a]='replacement';for(const item of items){const observed=item;}return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn iteration_retains_alias_writes_and_mutable_capture_alternatives() {
    for body in [
        "const items=['text'];const alias=items;alias[0]=a;for(const item of items){const observed=item;}return a;",
        "const items=['text'];const mutate=()=>{items[0]=a;};mutate();for(const item of items){const observed=item;}return a;",
        "const items=['text'];const identity=x=>x;identity(items)[0]=a;for(const item of items){const observed=item;}return a;",
        "const items=['text'];const replace=(target,value)=>{target[0]=value;return target;};for(const item of replace(items,a)){const observed=item;}return a;",
        "const items=['text'];for(const item of items){const observed=item;}items[0]=a;return a;",
        "for(let item of ['text']){item=a;const observed=item;}return a;",
        "for(let item of ['text']){const change=()=>{item=a;};change();const read=()=>item;const observed=read();}return a;",
        "const items=[{label:'text'}];const row=items[0];row.label=a;for(const item of items){const observed=item.label;}return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn iteration_function_members_preserve_targets_and_nonfunction_alternatives() {
    let callable = Observation {
        string: false,
        container: None,
        callable: true,
        other: false,
    };
    for source in [
        "export function run(a,b){for(const item of [{call:()=>a},{call:()=>b}]){const observed=item.call;}return a;}",
        "function first(x){return x;}function second(x){return x+1;}export function run(a,b){for(const item of [{call:first},{call:second}]){const observed=item.call;}return a;}",
        "export function run(a,b){for(const item of [[()=>a],[()=>b]]){const observed=item[0];}return a;}",
        "export function run(a,b){for(const item of [()=>a,()=>b]){const holder={call:item};const observed=holder.call;}return a;}",
    ] {
        assert_eq!(observe_source(source), callable, "{source}");
    }
    for body in [
        "for(const item of [{call:()=>a},{call:b}]){const observed=item.call;}return a;",
        "for(const item of [{call:()=>a},{}]){const observed=item.call;}return a;",
        "const items=[{call:()=>a}];items[0].call=b;for(const item of items){const observed=item.call;}return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn iteration_does_not_drop_unresolved_cycles_beside_known_values() {
    for body in [
        "const pending=()=>pending();for(const item of ['known',pending()]){const observed=item;}return a;",
        "const pending=()=>pending();for(const item of ['known',a?'replacement':pending()]){const observed=item;}return a;",
        "const left=()=>right();const right=()=>left();for(const item of ['known',a?'replacement':left()]){const observed=item;}return a;",
        "const pending=()=>pending();for(const item of (a?['known']:pending())){const observed=item;}return a;",
        "const pending=()=>pending();const items=['known'];items[0]=pending();for(const item of items){const observed=item;}return a;",
        "const pending=()=>pending();const items=['known'];items[0]=a?'replacement':pending();for(const item of items){const observed=item;}return a;",
        "const pending=()=>pending();for(const item of [{label:a?'known':pending()}]){const observed=item.label;}return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn iteration_analysis_handles_nested_sources_on_a_small_native_stack() {
    let mut body = String::from("{const observed=item63;}");
    for index in (0..64).rev() {
        body = format!("for(const item{index} of ['text']){body}");
    }
    body.push_str("return a;");
    let observed = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || observe(&body))
        .unwrap()
        .join()
        .expect("iteration provenance must not overflow the native stack");
    assert_eq!(observed, STRING);
}
