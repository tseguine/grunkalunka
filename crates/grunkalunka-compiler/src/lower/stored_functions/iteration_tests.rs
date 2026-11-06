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
    undefined: bool,
}

const STRING: Observation = Observation {
    string: true,
    container: None,
    callable: false,
    other: false,
    undefined: false,
};
const UNKNOWN: Observation = Observation {
    string: false,
    container: None,
    callable: false,
    other: true,
    undefined: false,
};
const MAYBE_UNDEFINED: Observation = Observation {
    string: false,
    container: None,
    callable: false,
    other: false,
    undefined: true,
};

fn observe_source(source: &str) -> Observation {
    observe_typed_source(source, false)
}

fn observe_typed_source(source: &str, typescript: bool) -> Observation {
    let allocator = Allocator::default();
    let source_type = if typescript {
        SourceType::ts()
    } else {
        SourceType::mjs()
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
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
        undefined: fields
            .values
            .get(&observed)
            .is_some_and(|origins| origins.contains(&Origin::Undefined)),
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
                undefined: false,
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
    assert_eq!(
        observe("for(const item of ['text',undefined]){const observed=item;}return a;"),
        MAYBE_UNDEFINED,
    );
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
        undefined: false,
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
        "const items=[{call:()=>a}];items[0].call=b;for(const item of items){const observed=item.call;}return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
    assert_eq!(
        observe("for(const item of [{call:()=>a},{}]){const observed=item.call;}return a;"),
        MAYBE_UNDEFINED,
    );
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

#[test]
fn nested_binding_origins_follow_projections_helpers_and_defaults() {
    for source in [
        "export function run(a,b){const {row:{label}}={row:{label:'text'}};const observed=label;return a;}",
        "export function run(a,b){const source={row:{label:'text'}};const alias=source;const {row:{label='fallback'}}=alias;const observed=label;return a;}",
        "export function run(a,b){const {row:{label='fallback'}}={row:{}};const observed=label;return a;}",
        "function identity(value){return value;}export function run(a,b){const {row:{label}}=identity({row:{label:'text'}});const read=()=>label;const observed=read();return a;}",
    ] {
        assert_eq!(observe_source(source), STRING, "{source}");
    }
    assert_eq!(
        observe("const {row:{items}}={row:{items:[a,b]}};const observed=items;return a;"),
        Observation {
            string: false,
            container: Some(Type::ManagedArray),
            callable: false,
            other: false,
            undefined: false,
        },
    );
}

#[test]
fn nested_binding_origins_keep_alias_writes_missing_paths_and_unknown_cycles() {
    for body in [
        "const source={row:{label:'text'}};const alias=source.row;alias.label=a;const {row:{label}}=source;const observed=label;return a;",
        "const source={row:{label:'text'}};const alias=source;alias.row=null;const {row:{label}}=source;const observed=label;return a;",
        "const pending=()=>pending();const {row:{label}}={row:{label:a?'text':pending()}};const observed=label;return a;",
        "const pending=()=>pending();const {row:{label}}={row:a?{label:'text'}:pending()};const observed=label;return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn iteration_patterns_project_element_origins_through_nested_shapes() {
    for source in [
        "export function run(a,b){for(const {row:{label}} of [{row:{label:'text'}}]){const observed=label;}return a;}",
        "export function run(a,b){for(const [label] of [['text']]){const observed=label;}return a;}",
        "export function run(a,b){for(const {items:[label]} of [{items:['text']}]){const observed=label;}return a;}",
        "export function run(a,b){for(const {label='fallback'} of [{}]){const observed=label;}return a;}",
        "export function run(a,b){for(const [label='fallback'] of [[]]){const observed=label;}return a;}",
        "export function run(a,b){for(const {label} of (a?[]:[{label:'text'}])){const observed=label;}return a;}",
        "function inspect(rows){for(const {label} of rows){const observed=label;}return 0;}export function run(a,b){return inspect([{label:'text'}]);}",
        "export function run(a,b){const rows=[{label:'text'}];rows[0]={label:'other'};for(const {label} of rows){const observed=label;}return a;}",
    ] {
        assert_eq!(observe_source(source), STRING, "{source}");
    }
}

#[test]
fn iteration_patterns_retain_mutation_missing_and_unresolved_alternatives() {
    for body in [
        "const rows=[{label:'text'}];const alias=rows;alias[0].label=a;for(const {label} of rows){const observed=label;}return a;",
        "const rows=[{label:'text'}];rows.length=2;for(const {label} of rows){const observed=label;}return a;",
        "for(let {label} of [{label:'text'}]){const change=()=>{label=a;};change();const observed=label;}return a;",
        "const pending=()=>pending();for(const {label} of [{label:'text'},pending()]){const observed=label;}return a;",
        "const pending=()=>pending();for(const {nested:{label}} of [{nested:a?{label:'text'}:pending()}]){const observed=label;}return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn mixed_binding_origins_follow_each_array_and_object_selection() {
    for body in [
        "const [[label]]=[['text']];const observed=label;return a;",
        "const {items:[{label}]}={items:[{label:'text'}]};const observed=label;return a;",
        "const [{items:[label]}]=[{items:['text']}];const observed=label;return a;",
        "const [[label='fallback']]=[['text']];const observed=label;return a;",
        "const {items:[label='fallback']}={items:[]};const observed=label;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const items=[{label:'text'}];const alias=items;alias[0]=null;const [{label}]=items;const observed=label;return a;",
        "const items=[['text']];const alias=items[0];alias[0]=a;const [[label]]=items;const observed=label;return a;",
        "const pending=()=>pending();const [[label]]=[[a?'text':pending()]];const observed=label;return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn binding_defaults_replace_only_proven_undefined_with_the_fallback_origin() {
    for body in [
        "const {label='text'}={};const observed=label;return a;",
        "const [label='text']=[];const observed=label;return a;",
        "const [label='text']=[,];const observed=label;return a;",
        "const [,label='text']=['prefix'];const observed=label;return a;",
        "const {label='text'}={label:undefined};const observed=label;return a;",
        "const {label='text'}={label:void a};const observed=label;return a;",
        "let missing;const {label='text'}={label:missing};const observed=label;return a;",
        "const source=a?{}:{label:'present'};const {label='text'}=source;const observed=label;return a;",
        "const source={};const alias=source;alias.label='written';const {label='text'}=source;const observed=label;return a;",
        "const source=Object.create(null);const {constructor:label='text'}=source;const observed=label;return a;",
        "const source={...{label:a},label:undefined};const {label='text'}=source;const observed=label;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    assert_eq!(
        observe_source(
            "function inspect(source){const {label='text'}=source;const observed=label;return 0;}export function run(a,b){return inspect({});}"
        ),
        STRING,
    );
    for (body, ty) in [
        (
            "const {value=[]}={};const observed=value;return a;",
            Type::ManagedArray,
        ),
        (
            "const [value={}]=[];const observed=value;return a;",
            Type::ManagedObject,
        ),
    ] {
        assert_eq!(
            observe(body),
            Observation {
                string: false,
                container: Some(ty),
                callable: false,
                other: false,
                undefined: false,
            },
            "{body}",
        );
    }
}

#[test]
fn binding_defaults_preserve_unknown_alias_mutation_and_cyclic_alternatives() {
    for body in [
        "const {label='text'}={label:null};const observed=label;return a;",
        "const {label='text'}={label:a};const observed=label;return a;",
        "const {label='text'}=a;const observed=label;return a;",
        "const undefined=a;const {label='text'}={label:undefined};const observed=label;return a;",
        "const {label=a}={};const observed=label;return a;",
        "const source={};const alias=source;alias.label=a;const {label='text'}=source;const observed=label;return a;",
        "const source={};const mutate=()=>{source.label=a;};mutate();const {label='text'}=source;const observed=label;return a;",
        "const source=Object.create(null);const key='label';source[key]=a;const {label='text'}=source;const observed=label;return a;",
        "const source=[undefined];source.length=0;const [label='text']=source;const observed=label;return a;",
        "const source=[undefined];source.pop();const [label='text']=source;const observed=label;return a;",
        "const pending=()=>pending();const {label='text'}={label:pending()};const observed=label;return a;",
        "const pending=()=>pending();const {label=pending()}={};const observed=label;return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn binding_defaults_keep_numeric_writes_to_undefined_cells() {
    for mutation in [
        "missing++;",
        "--missing;",
        "missing += a;",
        "missing -= a;",
        "missing *= a;",
        "missing /= a;",
        "missing %= a;",
        "missing **= a;",
        "missing <<= a;",
        "missing >>= a;",
        "missing >>>= a;",
        "missing &= a;",
        "missing |= a;",
        "missing ^= a;",
        "const mutate=()=>{missing++;};mutate();",
    ] {
        let body = format!(
            "let missing;{mutation}const {{label='text'}}={{label:missing}};const observed=label;return a;"
        );
        assert_eq!(observe(&body), UNKNOWN, "{body}");
    }
    for mutation in ["=a", "+=a", "++", "--"] {
        let source = format!(
            "export function run(a,b){{let missing=undefined;(missing as unknown){mutation};const {{label='text'}}={{label:missing}};const observed=label;return a;}}"
        );
        assert_eq!(observe_typed_source(&source, true), UNKNOWN, "{source}");
    }
}

#[test]
fn binding_defaults_cannot_infer_absence_from_spread_or_intrinsic_members() {
    for body in [
        "const source={...{label:a}};const {label='text'}=source;const observed=label;return a;",
        "const source={label:undefined,...{label:a}};const {label='text'}=source;const observed=label;return a;",
        "const source={label:undefined,[a]:b};const {label='text'}=source;const observed=label;return a;",
        "const source=[...a,'text'];const [label='text']=source;const observed=label;return a;",
        "const source=[];const {label='text'}={label:source.length};const observed=label;return a;",
        "const source=[];const {label='text'}={label:source.map};const observed=label;return a;",
        "const source={};const {label='text'}={label:source.constructor};const observed=label;return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn array_rest_origins_distinguish_the_receiver_from_its_copied_positions() {
    for body in [
        "const [head,...rest]=[a,b];const observed=rest;return a;",
        "const {items:[,...rest]=[a,b]}={};const observed=rest;return a;",
        "const [...rest]=Object.keys(a);const observed=rest;return a;",
        "for(const [head,...rest] of [[a,b]]){const observed=rest;}return a;",
        "const [...rest]=[];const read=()=>rest;const observed=read();return a;",
    ] {
        assert_eq!(
            observe(body),
            Observation {
                string: false,
                container: Some(Type::ManagedArray),
                callable: false,
                other: false,
                undefined: false,
            },
            "{body}",
        );
    }
    for body in [
        "const [...rest]=['text'];const observed=rest[0];return a;",
        "const [...rest]=['text'];const [label='fallback']=rest;const observed=label;return a;",
        "const [...rest]=[{label:'text'}];const observed=rest[0].label;return a;",
        "const src=[a,'x'];const [,...tail]=src;const observed=tail[0];return a;",
        "const [,...tail]=[a,,'x'];const [v='fallback']=tail;const observed=v;return a;",
        "const src=[a,'x',{label:'ready'}];const [,...mid]=src;const [,...tail]=mid;const observed=tail[0].label;return a;",
        "const src=['text'];const [...tail]=src;tail[0]=a;const observed=src[0];return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "let [...rest]=[];rest=a;const observed=rest;return a;",
        "const [...rest]=Object.keys('x');const observed=rest[0];return a;",
        "const src=[];const [x=(src[1]='text',a),...rest]=src;const observed=rest[0];return a;",
        "const src=[a,'text'];const [,...tail]=src;src.length=0;const observed=tail[0];return a;",
        "const src=[a,{label:'x'}];const [,...tail]=src;tail[0].label=b;const observed=src[1].label;return a;",
        "const src=[a,'x'];const [...root]=src;const [,...later]=root;const v=a?root:later;const [...end]=v;const observed=end[0];return a;",
    ] {
        let observation = observe(body);
        assert!(
            (observation.other || observation.undefined) && !observation.string,
            "{body}: {observation:?}"
        );
    }
}

#[test]
fn numeric_property_origins_follow_collisions_aliases_and_named_array_writes() {
    for body in [
        "const o={1:'safe'};const observed=o['1'];return a;",
        "const o={'0':'safe'};const {[-0]:value}=o;const observed=value;return a;",
        "const o={'-1.5':'safe'};const {[-1.5]:value}=o;const observed=value;return a;",
        "const items=['safe'];items[-1]=a;for(const item of items){const observed=item;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const o={1:'safe','1':a};const observed=o[1];return a;",
        "const o={1:'safe'};const alias=o;alias[0x1]=a;const {1:value}=o;const observed=value;return a;",
        "const o={[-0]:'safe'};o[0]=a;const observed=o['0'];return a;",
    ] {
        assert_eq!(observe(body), UNKNOWN, "{body}");
    }
}

#[test]
fn object_rest_fields_keep_source_kinds_and_include_writes_to_each_allocation() {
    for body in [
        "const {id,...rest}={id:a,label:'ready'};const observed=rest.label;return a;",
        "const {label,...rest}={label:a};const {label:value='default'}=rest;const observed=value;return a;",
        "const o={label:'x'};const {...r1}=o;const {...r2}=r1;const observed=r2.label;return a;",
        "const {...root}={label:'x'};const {...l}=root;const {...r}=root;const {...copy}=a?l:r;const observed=copy.label;return a;",
        "const source=a?{label:'x'}:{label:'y'};const {...rest}=source;const observed=rest.label;return a;",
        "const source={label:'x'};const {...rest}=source;rest.label=a;const observed=source.label;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "let current={label:'x'};for(let i=0;i<2;i++){const {...next}=current;current=next;}const observed=current.label;return a;",
        "const source={label:'x'};const alias=source;alias.label=b;const {...rest}=source;const observed=rest.label;return a;",
        "const source={label:'x'};const {...rest}=source;const alias=rest;alias.label=b;const observed=rest.label;return a;",
        "const source={nested:{label:'x'}};const {...rest}=source;rest.nested.label=b;const observed=source.nested.label;return a;",
        "const {label,...rest}={label:'x'};rest.label=b;const {label:value='default'}=rest;const observed=value;return a;",
        "const source={...{label:a}};const {...rest}=source;const {label:value='fallback'}=rest;const observed=value;return a;",
    ] {
        let observation = observe(body);
        assert!(
            !observation.string && observation.other,
            "{body}: {observation:?}"
        );
    }
}

#[test]
fn tail_iteration_tracks_only_the_reachable_suffix_and_its_writes() {
    for body in [
        "const [...rest]=['text'];for(const item of rest){const observed=item;}return a;",
        "const src=[a,'x','y'];src[0]=b;const [,...tail]=src;for(const item of tail){const observed=item;}return a;",
        "const src=[,a,'x'];const [,...mid]=src;const [,...tail]=mid;for(const item of tail){const observed=item;}return a;",
        "const src=a?[a,'x']:[b];const [,...tail]=src;for(const item of tail){const observed=item;}return a;",
        "const src=[a,'x'];const [,...tail]=src;src[1]='y';tail.named=b;for(const item of tail){const observed=item;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const original=[a,'x'];const [...base]=original;const [,...later]=base;const branch=a?base:later;const [...tail]=branch;for(const item of tail){const observed=item;}return a;",
        "const src=[a,,'x'];const [,...tail]=src;for(const item of tail){const observed=item;}return a;",
        "const src=[a,'x'];const [,...tail]=src;const alias=src;alias[1]=b;for(const item of tail){const observed=item;}return a;",
        "const src=[a,'x'];const [,...tail]=src;tail[2]=b;for(const item of tail){const observed=item;}return a;",
        "const src=[a,'x'];const [,...tail]=src;src.pop();for(const item of tail){const observed=item;}return a;",
        "const src=[a,'x'];const [,...tail]=src;tail.length=2;for(const item of tail){const observed=item;}return a;",
        "let src=['x'];for(let i=0;i<2;i++){const [...next]=src;src=next;}for(const item of src){const observed=item;}return a;",
    ] {
        let facts = observe(body);
        assert!(facts.other && !facts.string, "{body}: {facts:?}");
    }
}

#[test]
fn entry_pair_key_proofs_include_alias_writes_and_resizing() {
    for body in [
        "for(const [k] of Object.entries({x:a})){const observed=k;}return a;",
        "const rows=Object.entries({x:a});for(const pair of rows){const observed=pair[0];}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const rows=Object.entries({x:a});for(const pair of rows){const alias=pair;alias[0]=a;}for(const [k] of rows){const observed=k;}return a;",
        "const rows=Object.entries({x:a});for(const pair of rows)pair.pop();for(const [k] of rows){const observed=k;}return a;",
        "const rows=Object.entries({x:a});rows.length=2;for(const [k] of rows){const observed=k;}return a;",
    ] {
        let info = observe(body);
        assert!(!info.string && info.other, "{body}: {info:?}");
    }
}

#[test]
fn enumerated_names_prove_yielded_strings_but_indexed_reads_can_miss() {
    for body in [
        "const keys=Object.keys({});for(const k of keys){const observed=k;}return a;",
        "const [k='fallback']=Object.keys({x:a});const observed=k;return a;",
        "const [...tail]=Object.keys({x:a});for(const k of tail){const observed=k;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    let miss = observe("const keys=Object.keys({});const observed=keys[0];return a;");
    assert!(miss.undefined && !miss.string && !miss.other, "{miss:?}");
    for body in [
        "const keys=Object.keys({x:a});const alias=keys;alias[0]=b;for(const k of keys){const observed=k;}return a;",
        "const keys=Object.keys({x:a});keys.length=3;for(const k of keys){const observed=k;}return a;",
    ] {
        let fact = observe(body);
        assert!(fact.other && !fact.string, "{body}: {fact:?}");
    }
}

#[test]
fn enumerated_values_follow_initializers_alias_writes_and_output_mutations() {
    for body in [
        "const k=a?'x':'y';for(const v of Object.values.call(null,{fixed:'x',[k]:'y'})){const observed=v;}return a;",
        "const src=Object.create(null);src[a?'one':'two']='x';for(const [k,v] of Object.entries(src)){const observed=v;}return a;",
        "const src=[,'x'];src.named='y';for(const v of Object.values(src)){const observed=v;}return a;",
        "const [v='x']=Object.values({});const observed=v;return a;",
        "for(const [k,v] of Object.entries('xy')){const observed=v;}return a;",
        "const src={x:{label:'x'}};for(const v of Object.values(src)){const observed=v.label;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const src={x:'x'};const alias=src;alias.y=b;for(const v of Object.values(src)){const observed=v;}return a;",
        "const src={x:'x',...{y:b}};for(const [k,v] of Object.entries(src)){const observed=v;}return a;",
        "const rows=Object.entries({x:'x'});for(const row of rows)row[1]=a;for(const [k,v] of rows){const observed=v;}return a;",
        "const list=Object.values({x:'x'});list.length=2;for(const v of list){const observed=v;}return a;",
        "const src={label:'x'};const record={x:src};src.label=b;for(const v of Object.values(record)){const observed=v.label;}return a;",
    ] {
        let fact = observe(body);
        assert!(fact.other && !fact.string, "{body}: {fact:?}");
    }
}

#[test]
fn copy_spread_preserves_indexed_origins_without_aliasing_writes() {
    for body in [
        "const source=[...['text']];const [label='text']=source;const observed=label;return a;",
        "const src=['x'];const copy=[...src];copy[0]=a;const observed=src[0];return a;",
        "const src=[{label:'x'}];const copy=[...src];const observed=copy[0].label;return a;",
        "const src=[,'x'];const copy=[...src];const [v='fallback']=copy;const observed=v;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const src=[{label:'x'}];const copy=[...src];copy[0].label=a;const observed=src[0].label;return a;",
        "const src=['x'];const copy=[...src];copy.length=3;for(const v of copy){const observed=v;}return a;",
    ] {
        let fact = observe(body);
        assert!(fact.other && !fact.string, "{body}: {fact:?}");
    }
}

#[test]
fn string_spread_index_proofs_distinguish_defaults_and_possible_misses() {
    assert_eq!(
        observe("const [s='x']=[...'abc'];const observed=s;return a;"),
        STRING
    );
    let miss = observe("const xs=[...''];const observed=xs[0];return a;");
    assert!(miss.undefined && !miss.string && !miss.other, "{miss:?}");
    let write = observe("const xs=[...'x'];xs[0]=a;const [s='x']=xs;const observed=s;return a;");
    assert!(write.other && !write.string, "{write:?}");
}

#[test]
fn array_slice_values_follow_source_and_copy_aliases_without_named_fields() {
    for body in [
        "const src=['x'];src.named=a;for(const v of src.slice(b)){const observed=v;}return a;",
        "const src=[{label:'x'}];for(const {label} of src.slice()){const observed=label;}return a;",
        "const src=['x'];const copy=src.slice().slice();for(const v of copy){const observed=v;}return a;",
        "const src=['x'];const copy=src.slice();copy[0]=a;const observed=src[0];return a;",
        "const [v='fallback']=['x'].slice(a);const observed=v;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const src=['x'];const alias=src;alias[0]=a;for(const v of src.slice()){const observed=v;}return a;",
        "const src=['x'];const copy=src.slice();const alias=copy;alias[0]=a;for(const v of copy){const observed=v;}return a;",
        "const src=['x'];src.length=2;for(const v of src.slice()){const observed=v;}return a;",
        "const src=['x'];const copy=src.slice();copy.length=2;for(const v of copy){const observed=v;}return a;",
        "const src=[,'x'];for(const v of src.slice()){const observed=v;}return a;",
        "const src=[{label:'x'}];const copy=src.slice();for(const child of copy)child.label=a;const observed=src[0].label;return a;",
        "const src=[{label:'x'}];src[0].label=a;for(const child of src.slice()){const observed=child.label;}return a;",
    ] {
        let facts = observe(body);
        assert!(facts.other && !facts.string, "{body}: {facts:?}");
    }
}

#[test]
fn reversed_iteration_keeps_values_but_not_original_positions() {
    for body in [
        "const xs=['x','y'];xs.reverse();for(const v of xs){const observed=v;}return a;",
        "const xs=[{label:'x'}];for(const {label} of xs.reverse()){const observed=label;}return a;",
        "const xs=['x','y'];xs.reverse();const [,...tail]=xs;for(const v of tail){const observed=v;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const xs=[a,'x'];xs.reverse();const [,...tail]=xs;for(const v of tail){const observed=v;}return a;",
        "const xs=['x',a];xs.reverse();const observed=xs[0];return a;",
        "const xs=['x'];const alias=xs.reverse();alias[0]=a;for(const v of xs){const observed=v;}return a;",
        "const xs=[,'x'];xs.reverse();for(const v of xs){const observed=v;}return a;",
        "const xs=['x'];xs.reverse();xs.pop();for(const v of xs){const observed=v;}return a;",
    ] {
        let facts = observe(body);
        assert!(facts.other && !facts.string, "{body}: {facts:?}");
    }
}

#[test]
fn to_reversed_tracks_fresh_slots_and_shared_child_mutations() {
    for body in [
        "const xs=['x'];const ys=xs.toReversed();ys[0]=a;const observed=xs[0];return a;",
        "const xs=['x'];for(const v of xs.toReversed()){const observed=v;}return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const xs=[{label:'x'}];const ys=xs.toReversed();for(const v of ys)v.label=a;const observed=xs[0].label;return a;",
        "const xs=['x'];xs[0]=a;for(const v of xs.toReversed()){const observed=v;}return a;",
        "for(const v of [,'x'].toReversed()){const observed=v;}return a;",
    ] {
        let f = observe(body);
        assert!(f.other && !f.string, "{body}: {f:?}");
    }
}

#[test]
fn to_reversed_keeps_unrepresented_calls_and_kinds_rejected() {
    for body in [
        "return a.toReversed().length;",
        "return [a].toReversed?.().length;",
        "return [a].toReversed(...[]).length;",
        "return [a].toReversed.call([a]).length;",
        "const xs=[a];xs.toReversed=()=>[];return xs.toReversed().length;",
        "const xs=a?[]:null;return xs.toReversed().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let err = match crate::lower_source(&source, ts) {
                Err(err) => err,
                Ok(_) => panic!("unsupported input"),
            };
            assert!(!err.contains("internal IR"), "{err}");
        }
    }
}

#[test]
fn relative_string_selection_does_not_prove_a_string() {
    for source in [
        "export function run(a,b){const observed='abc'.at(a);return 0;}",
        "function last(s){return s.at(-1);}export function run(a,b){const observed=last('');return 0;}",
    ] {
        assert!(!observe_source(source).string);
    }
    assert!(
        observe_source("export function run(a,b){const observed='abc'.charAt(a);return 0;}").string
    );
}

#[test]
fn string_at_defaults_replace_missing_units_without_laundering_other_origins() {
    for body in [
        "const {c='?'}={c:'abc'.at(a)};const observed=c;return a;",
        "const obj={at:x=>'x'};const {c='?'}={c:obj.at(a)};const observed=c;return a;",
        "const [c='?']=['abc'.at(a)];const observed=c;return a;",
        "const box={c:'abc'.at(a)};const alias=box;const {c='?'}=alias;const observed=c;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    let selected = observe("const observed='abc'.at(a);return a;");
    assert!(!selected.string && selected.undefined && !selected.other);
    assert_eq!(
        observe_source(
            "function last(s){return s.at(-1);}export function run(a,b){const {c='?'}={c:last('')};const observed=c;return a;}"
        ),
        STRING
    );
    for body in [
        "const box={c:'abc'.at(a)};box.c=b;const {c='?'}=box;const observed=c;return a;",
        "const box={c:'abc'.at(a)};const alias=box;alias.c=null;const {c='?'}=box;const observed=c;return a;",
        "const {c=b}={c:'abc'.at(a)};const observed=c;return a;",
        "const {c='?'}={c:'abc'.codePointAt(a)};const observed=c;return a;",
        "const obj={at:x=>x};const {c='?'}={c:obj.at(a)};const observed=c;return a;",
        "let s='abc';const read=()=>{const {c='?'}={c:s.at(0)};const observed=c;return 0;};s=a;return read();",
    ] {
        assert!(!observe(body).string, "{body}");
    }
}

#[test]
fn array_at_defaults_preserve_elements_and_a_missing_alternative() {
    for body in [
        "const xs=['abc'];const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const [c='?']=[['abc'].at(a)];const observed=c;return a;",
        "const xs=['abc'];const alias=xs;alias[0]='replacement';const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const xs=['abc'];const {c='?'}={c:xs.slice().at(a)};const observed=c;return a;",
        "const xs=['abc'];const [...copy]=xs;const {c='?'}={c:copy.at(a)};const observed=c;return a;",
        "const {c='?'}={c:[].at(a)};const observed=c;return a;",
        "const {c='?'}={c:Object.keys({x:a}).at(a)};const observed=c;return a;",
    ] {
        assert_eq!(observe(body), STRING, "{body}");
    }
    for body in [
        "const observed=['abc'].at(a);return a;",
        "const observed=['abc'].at(0);return a;",
        "const observed=[{label:'abc'}].at(-1);return a;",
    ] {
        assert_eq!(observe(body), MAYBE_UNDEFINED, "{body}");
    }
    for (body, ty) in [
        (
            "const {c={label:'?'}}={c:[{label:'abc'}].at(a)};const observed=c;return a;",
            Type::ManagedObject,
        ),
        (
            "const {c=[]}={c:[[a],[b]].at(a)};const observed=c;return a;",
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
                undefined: false,
            },
            "{body}"
        );
    }
    assert!(observe("const {c=()=>b}={c:[()=>a].at(a)};const observed=c;return a;").callable);
    assert_eq!(
        observe_source(
            "function pick(xs,i){return xs.at(i);}function forward(xs,i){return pick(xs,i);}export function run(a,b){const rows=[{label:'abc'}];const read=()=>forward(rows,a);const {c={label:'?'}}={c:read()};const observed=c.label;return a;}"
        ),
        STRING
    );
}

#[test]
fn array_at_defaults_retain_mixed_alias_and_sparse_uncertainty() {
    for body in [
        "const xs=['abc',a];const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const xs=['abc'];const alias=xs;alias[0]=a;const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const xs=['abc'];const replace=(target)=>{target[0]=null;};replace(xs);const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const xs=['abc'];const {c='?'}={c:xs.at(a)};xs[0]=b;const observed=c;return a;",
        "const xs=['abc'];xs.length=2;const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const xs=[,'abc'];const {c='?'}={c:xs.at(a)};const observed=c;return a;",
        "const xs=[{label:'abc'}];const copy=xs.slice();copy[0].label=a;const {c={label:'?'}}={c:xs.at(a)};const observed=c.label;return a;",
        "const xs=a?['abc']:null;const {c='?'}={c:xs.at(a)};const observed=c;return a;",
    ] {
        let found = observe(body);
        assert!(found.other && !found.string, "{body}: {found:?}");
    }
}
