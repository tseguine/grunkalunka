use quote::{ToTokens, quote};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

fn tokens<T: ToTokens>(v: &T) -> String {
    v.to_token_stream().to_string()
}
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.parse_args::<syn::Path>()
                .is_ok_and(|p| p.is_ident("test"))
    })
}
fn position<T: Spanned>(v: &T) -> Value {
    let p = v.span().start();
    json!({"line":p.line,"column":p.column+1})
}
fn end_position<T: Spanned>(v: &T) -> Value {
    let p = v.span().end();
    json!({"line":p.line,"column":p.column+1})
}
fn path(p: &syn::Path) -> Vec<String> {
    p.segments.iter().map(|s| s.ident.to_string()).collect()
}
fn ty(t: &syn::Type) -> Value {
    match t {
        syn::Type::Reference(t) => ty(&t.elem),
        syn::Type::Paren(t) => ty(&t.elem),
        syn::Type::Group(t) => ty(&t.elem),
        syn::Type::Slice(t) => json!({"path":["Slice"],"args":[ty(&t.elem)]}),
        syn::Type::Array(t) => json!({"path":["Array"],"args":[ty(&t.elem)]}),
        syn::Type::Path(t) => {
            json!({"path":path(&t.path),"args":t.path.segments.last().map(|s|match &s.arguments {
            syn::PathArguments::AngleBracketed(a)=>a.args.iter().filter_map(|a|if let syn::GenericArgument::Type(t)=a {Some(ty(t))}else{None}).collect::<Vec<_>>(),
            _=>vec![]}).unwrap_or_default()})
        }
        _ => json!({"unknown":tokens(t)}),
    }
}
fn owner(t: &syn::Type) -> String {
    match t {
        syn::Type::Path(t) => t.path.segments.last().unwrap().ident.to_string(),
        _ => tokens(t),
    }
}
fn attrs(items: &[syn::Attribute]) -> Vec<String> {
    items.iter().map(tokens).collect()
}

struct RawFunction {
    id: String,
    module: String,
    file: String,
    name: String,
    owner: Option<String>,
    signature: syn::Signature,
    body: syn::Block,
    attributes: Vec<syn::Attribute>,
}
#[derive(Default)]
struct Inventory {
    files: Vec<Value>,
    functions: Vec<Value>,
    types: Vec<Value>,
    imports: Vec<Value>,
    macros: Vec<Value>,
    unresolved_modules: Vec<Value>,
    raw: Vec<RawFunction>,
    visited: BTreeSet<(PathBuf, String)>,
}

impl Inventory {
    fn file(&mut self, file: &Path, module: &str, module_dir: &Path) -> Result<(), String> {
        if !self
            .visited
            .insert((file.to_path_buf(), module.to_string()))
        {
            return Ok(());
        }
        let source = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let ast = syn::parse_file(&source).map_err(|e| format!("{}: {e}", file.display()))?;
        self.files
            .push(json!({"file":file.display().to_string(),"module":module,"bytes":source.len()}));
        self.items(&ast.items, file, module, module_dir)
    }
    fn items(
        &mut self,
        items: &[syn::Item],
        file: &Path,
        module: &str,
        module_dir: &Path,
    ) -> Result<(), String> {
        for item in items {
            match item {
                syn::Item::Mod(m) if !test_only(&m.attrs)=>{
                    let sub=format!("{module}::{}",m.ident);
                    if let Some((_,items))=&m.content {self.items(items,file,&sub,&module_dir.join(m.ident.to_string()))?;}
                    else {
                        let explicit=m.attrs.iter().find_map(|a|if a.path().is_ident("path") {if let syn::Meta::NameValue(v)=&a.meta {if let syn::Expr::Lit(l)=&v.value {if let syn::Lit::Str(s)=&l.lit{return Some(s.value())}}}None}else{None});
                        let candidates=if let Some(p)=explicit {vec![file.parent().unwrap().join(p)]}else{vec![module_dir.join(format!("{}.rs",m.ident)),module_dir.join(m.ident.to_string()).join("mod.rs")]};
                        if let Some(p)=candidates.iter().find(|p|p.is_file()) {
                            let next=if p.file_name().is_some_and(|s|s=="mod.rs"){p.parent().unwrap().to_path_buf()}else{p.with_extension("")};
                            self.file(p,&sub,&next)?;
                        }else{self.unresolved_modules.push(json!({"file":file.display().to_string(),"module":sub,"candidates":candidates,"position":position(m)}));}
                    }
                }
                syn::Item::Use(u) if !test_only(&u.attrs)=>self.imports.push(json!({"module":module,"use":tokens(&u.tree)})),
                syn::Item::Fn(f) if !test_only(&f.attrs)=>self.raw.push(RawFunction{id:format!("{module}::{}",f.sig.ident),module:module.into(),file:file.display().to_string(),name:f.sig.ident.to_string(),owner:None,signature:f.sig.clone(),body:(*f.block).clone(),attributes:f.attrs.clone()}),
                syn::Item::Impl(i) if !test_only(&i.attrs)=>{
                    let name=owner(&i.self_ty);
                    for member in &i.items {if let syn::ImplItem::Fn(f)=member {if !test_only(&f.attrs){
                        let trait_name=i.trait_.as_ref().map(|(_,p,_)|tokens(p));
                        let id=format!("{module}::{name}{}::{}",trait_name.as_ref().map(|t|format!("<as {t}>")).unwrap_or_default(),f.sig.ident);
                        self.raw.push(RawFunction{id,module:module.into(),file:file.display().to_string(),name:f.sig.ident.to_string(),owner:Some(name.clone()),signature:f.sig.clone(),body:f.block.clone(),attributes:f.attrs.clone()});
                    }}}
                }
                syn::Item::Struct(s) if !test_only(&s.attrs)=>self.types.push(json!({"module":module,"name":s.ident.to_string(),"fields":s.fields.iter().enumerate().map(|(i,f)|json!({"name":f.ident.as_ref().map(|x|x.to_string()).unwrap_or(i.to_string()),"type":ty(&f.ty)})).collect::<Vec<_>>()})),
                syn::Item::Enum(e) if !test_only(&e.attrs)=>{
                    for v in &e.variants {self.types.push(json!({"module":module,"name":format!("{}::{}",e.ident,v.ident),"fields":v.fields.iter().enumerate().map(|(i,f)|json!({"name":f.ident.as_ref().map(|x|x.to_string()).unwrap_or(i.to_string()),"type":ty(&f.ty)})).collect::<Vec<_>>()}));}
                }
                syn::Item::Macro(m) if !test_only(&m.attrs)=>self.macros.push(json!({"kind":"item","file":file.display().to_string(),"module":module,"position":position(m),"macro":tokens(&m.mac.path),"tokens":tokens(&m.mac.tokens)})),
                _=>{}
            }
        }
        Ok(())
    }
}

struct Body<'a> {
    inventory: &'a mut Inventory,
    id: String,
    module: String,
    file: String,
    owner: Option<String>,
    environment: BTreeMap<String, Value>,
    local_types: BTreeMap<String, Value>,
    calls: Vec<Value>,
    macros: Vec<Value>,
}
impl Body<'_> {
    fn closure_id(&self, c: &syn::ExprClosure) -> String {
        let p = c.span().start();
        format!("{}::closure@{}:{}", self.id, p.line, p.column + 1)
    }
    fn expr(&self, e: &syn::Expr) -> Value {
        match e {
            syn::Expr::Path(p) => {
                let segments = path(&p.path);
                if segments.len() == 1 {
                    if let Some(v) = self.environment.get(&segments[0]) {
                        return v.clone();
                    }
                }
                json!({"kind":"path","segments":segments})
            }
            syn::Expr::Field(f) => {
                json!({"kind":"field","base":self.expr(&f.base),"field":tokens(&f.member)})
            }
            syn::Expr::Reference(r) => self.expr(&r.expr),
            syn::Expr::Paren(p) => self.expr(&p.expr),
            syn::Expr::Group(g) => self.expr(&g.expr),
            syn::Expr::Unary(u) => self.expr(&u.expr),
            syn::Expr::Try(t) => json!({"kind":"try","inner":self.expr(&t.expr)}),
            syn::Expr::Index(i) => json!({"kind":"element","container":self.expr(&i.expr)}),
            syn::Expr::Tuple(t) => {
                json!({"kind":"tuple","values":t.elems.iter().map(|e|self.expr(e)).collect::<Vec<_>>()})
            }
            syn::Expr::Macro(m) if m.mac.path.is_ident("vec") => {
                json!({"kind":"typed","type":{"path":["Vec"],"args":[]}})
            }
            syn::Expr::Macro(m) if m.mac.path.is_ident("format") => {
                json!({"kind":"typed","type":{"path":["String"],"args":[]}})
            }
            syn::Expr::Lit(l) => {
                json!({"kind":"typed","type":{"path":[match &l.lit {syn::Lit::Str(_)=>"str",syn::Lit::Bool(_)=>"bool",syn::Lit::Int(_)=>"integer",_=>"literal"}],"args":[]}})
            }
            syn::Expr::Struct(s) => json!({"kind":"typed","type":{"path":path(&s.path),"args":[]}}),
            syn::Expr::Cast(c) => json!({"kind":"typed","type":ty(&c.ty)}),
            syn::Expr::Call(c) => {
                json!({"kind":"call","callee":self.expr(&c.func),"args":c.args.iter().map(|a|self.expr(a)).collect::<Vec<_>>()})
            }
            syn::Expr::MethodCall(c) => {
                json!({"kind":"method","receiver":self.expr(&c.receiver),"method":c.method.to_string(),"args":c.args.iter().map(|a|self.expr(a)).collect::<Vec<_>>()})
            }
            syn::Expr::Closure(c) => json!({"kind":"closure","id":self.closure_id(c)}),
            _ => json!({"kind":"unknown","text":tokens(e).chars().take(180).collect::<String>()}),
        }
    }
    fn pattern_type(&self, name: &str, short: &str) -> Option<Value> {
        self.local_types
            .get(name)
            .or_else(|| self.local_types.get(short))
            .cloned()
            .or_else(|| {
                let matches = self
                    .inventory
                    .types
                    .iter()
                    .filter(|t| t["scope"].is_null() && (t["name"] == name || t["name"] == short))
                    .collect::<Vec<_>>();
                matches
                    .iter()
                    .find(|t| t["module"] == self.module)
                    .or_else(|| {
                        if matches.len() == 1 {
                            matches.first()
                        } else {
                            None
                        }
                    })
                    .map(|t| (*t).clone())
            })
    }
    fn bind(&mut self, pat: &syn::Pat, value: Value) {
        match pat {
            syn::Pat::Ident(i) => {
                self.environment.insert(i.ident.to_string(), value);
            }
            syn::Pat::Type(t) => self.bind(&t.pat, json!({"kind":"typed","type":ty(&t.ty)})),
            syn::Pat::Reference(r) => self.bind(&r.pat, value),
            syn::Pat::TupleStruct(t) => {
                let mut parts = path(&t.path);
                if parts.first().is_some_and(|p| p == "Self") {
                    if let Some(owner) = &self.owner {
                        parts[0] = owner.clone();
                    }
                }
                if parts.last().is_some_and(|p| p == "Some" || p == "Ok") && t.elems.len() == 1 {
                    self.bind(&t.elems[0], json!({"kind":"try","inner":value}));
                    return;
                }
                let name = parts
                    .iter()
                    .rev()
                    .take(2)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("::");
                let definition = self.pattern_type(&name, parts.last().unwrap());
                for (i, p) in t.elems.iter().enumerate() {
                    let inferred = definition
                        .as_ref()
                        .and_then(|d| d["fields"].as_array())
                        .and_then(|fs| fs.get(i))
                        .map(|f| json!({"kind":"typed","type":f["type"]}))
                        .unwrap_or_else(
                            || json!({"kind":"unknown","text":format!("pattern {}",tokens(p))}),
                        );
                    self.bind(p, inferred);
                }
            }
            syn::Pat::Struct(s) => {
                let mut parts = path(&s.path);
                if parts.first().is_some_and(|p| p == "Self") {
                    if let Some(owner) = &self.owner {
                        parts[0] = owner.clone();
                    }
                }
                let name = parts
                    .iter()
                    .rev()
                    .take(2)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("::");
                let definition = self.pattern_type(&name, parts.last().unwrap());
                for field in &s.fields {
                    let name = tokens(&field.member);
                    let inferred=definition.as_ref().and_then(|d|d["fields"].as_array()).and_then(|fs|fs.iter().find(|f|f["name"]==name)).map(|f|json!({"kind":"typed","type":f["type"]})).unwrap_or_else(||json!({"kind":"unknown","text":format!("pattern {}",tokens(&field.pat))}));
                    self.bind(&field.pat, inferred);
                }
            }
            syn::Pat::Tuple(t) => {
                for (index, p) in t.elems.iter().enumerate() {
                    let element = if value["kind"] == "tuple" {
                        value["values"].get(index).cloned()
                    } else {
                        None
                    };
                    self.bind(
                        p,
                        element.unwrap_or_else(|| json!({"kind":"unknown","text":tokens(p)})),
                    );
                }
            }
            _ => {}
        }
    }
}
impl<'ast> Visit<'ast> for Body<'_> {
    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        if !test_only(&m.attrs) {
            visit::visit_expr_match(self, m);
        }
    }
    fn visit_expr_loop(&mut self, l: &'ast syn::ExprLoop) {
        if !test_only(&l.attrs) {
            visit::visit_expr_loop(self, l);
        }
    }
    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        if test_only(&w.attrs) {
            return;
        }
        self.visit_expr(&w.cond);
        let old = self.environment.clone();
        if let syn::Expr::Let(l) = &*w.cond {
            self.bind(&l.pat, self.expr(&l.expr));
        }
        self.visit_block(&w.body);
        self.environment = old;
    }
    fn visit_expr_block(&mut self, b: &'ast syn::ExprBlock) {
        if !test_only(&b.attrs) {
            self.visit_block(&b.block);
        }
    }
    fn visit_expr_for_loop(&mut self, f: &'ast syn::ExprForLoop) {
        if test_only(&f.attrs) {
            return;
        }
        self.visit_expr(&f.expr);
        let old = self.environment.clone();
        self.bind(
            &f.pat,
            json!({"kind":"element","container":self.expr(&f.expr)}),
        );
        self.visit_block(&f.body);
        self.environment = old;
    }
    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        if test_only(&i.attrs) {
            return;
        }
        self.visit_expr(&i.cond);
        let old = self.environment.clone();
        if let syn::Expr::Let(l) = &*i.cond {
            self.bind(&l.pat, self.expr(&l.expr));
        }
        self.visit_block(&i.then_branch);
        self.environment = old;
        if let Some((_, other)) = &i.else_branch {
            self.visit_expr(other);
        }
    }
    fn visit_block(&mut self, b: &'ast syn::Block) {
        let old = self.environment.clone();
        let old_types = self.local_types.clone();
        for stmt in &b.stmts {
            let mut definitions = vec![];
            match stmt {
                syn::Stmt::Item(syn::Item::Struct(s)) if !test_only(&s.attrs) => {
                    definitions.push((s.ident.to_string(), &s.fields))
                }
                syn::Stmt::Item(syn::Item::Enum(e)) if !test_only(&e.attrs) => {
                    for v in &e.variants {
                        definitions.push((format!("{}::{}", e.ident, v.ident), &v.fields));
                    }
                }
                _ => {}
            }
            for (name, fields) in definitions {
                let definition = json!({"module":self.module,"scope":self.id,"name":name,"fields":fields.iter().enumerate().map(|(i,f)|json!({"name":f.ident.as_ref().map(|x|x.to_string()).unwrap_or(i.to_string()),"type":ty(&f.ty)})).collect::<Vec<_>>()});
                self.local_types.insert(name, definition.clone());
                self.inventory.types.push(definition);
            }
        }
        visit::visit_block(self, b);
        self.environment = old;
        self.local_types = old_types;
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if test_only(&l.attrs) {
            return;
        }
        if let Some(i) = &l.init {
            self.visit_expr(&i.expr);
            let value = self.expr(&i.expr);
            self.bind(&l.pat, value);
            if let Some((_, e)) = &i.diverge {
                self.visit_expr(e);
            }
        } else {
            self.bind(
                &l.pat,
                json!({"kind":"unknown","text":"uninitialized local"}),
            );
        }
    }
    fn visit_arm(&mut self, a: &'ast syn::Arm) {
        let old = self.environment.clone();
        self.bind(&a.pat, json!({"kind":"unknown","text":"match pattern"}));
        visit::visit_arm(self, a);
        self.environment = old;
    }
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if test_only(&c.attrs) {
            return;
        }
        self.calls.push(json!({"kind":"call","position":position(c),"callee":self.expr(&c.func),"args":c.args.iter().map(|a|self.expr(a)).collect::<Vec<_>>(),"syntax":tokens(c).chars().take(240).collect::<String>()}));
        visit::visit_expr_call(self, c);
    }
    fn visit_expr_method_call(&mut self, c: &'ast syn::ExprMethodCall) {
        if test_only(&c.attrs) {
            return;
        }
        self.calls.push(json!({"kind":"method","position":position(c),"receiver":self.expr(&c.receiver),"method":c.method.to_string(),"args":c.args.iter().map(|a|self.expr(a)).collect::<Vec<_>>(),"syntax":tokens(c).chars().take(240).collect::<String>()}));
        visit::visit_expr_method_call(self, c);
    }
    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        if test_only(&c.attrs) {
            return;
        }
        let id = self.closure_id(c);
        let mut env = self.environment.clone();
        let mut params = vec![];
        for (index, p) in c.inputs.iter().enumerate() {
            let name = match p {
                syn::Pat::Ident(p) => p.ident.to_string(),
                syn::Pat::Type(p) => tokens(&p.pat),
                _ => tokens(p),
            };
            let t = if let syn::Pat::Type(p) = p {
                ty(&p.ty)
            } else {
                json!({"unknown":"closure parameter"})
            };
            params.push(json!({"name":name,"type":t}));
            env.insert(
                name,
                json!({"kind":"parameter","function":id,"index":index,"type":t}),
            );
        }
        let mut nested = Body {
            inventory: self.inventory,
            id: id.clone(),
            module: self.module.clone(),
            file: self.file.clone(),
            owner: self.owner.clone(),
            environment: env,
            local_types: self.local_types.clone(),
            calls: vec![],
            macros: vec![],
        };
        nested.visit_expr(&c.body);
        let output = match &c.output {
            syn::ReturnType::Default => json!(null),
            syn::ReturnType::Type(_, t) => ty(t),
        };
        let node = json!({"id":id,"module":nested.module,"file":nested.file,"name":"<closure>","owner":null,"context_owner":nested.owner,"kind":"closure","position":position(c),"end_position":end_position(c),"body_tokens":tokens(&c.body),"params":params,"return_type":output,"has_receiver":false,"attributes":attrs(&c.attrs),"calls":nested.calls,"macros":nested.macros});
        nested.inventory.functions.push(node);
    }
    fn visit_expr_macro(&mut self, m: &'ast syn::ExprMacro) {
        if test_only(&m.attrs) {
            return;
        }
        self.visit_macro(&m.mac);
    }
    fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
        if test_only(&m.attrs) {
            return;
        }
        self.visit_macro(&m.mac);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if m.path.is_ident("vec") {
            let input = &m.tokens;
            if let Ok(expr) = syn::parse2::<syn::Expr>(quote!([#input])) {
                self.macros.push(json!({"macro":"vec","position":position(m),"arguments_parsed_as_expressions":true,"parser":"array-or-repeat","tokens":tokens(input)}));
                self.visit_expr(&expr);
                return;
            }
        }
        if m.path.is_ident("matches") {
            let parser = |input: syn::parse::ParseStream<'_>| -> syn::Result<(syn::Expr, syn::Pat, Option<syn::Expr>)> {
                let value = input.parse()?;
                input.parse::<syn::Token![,]>()?;
                let pattern = syn::Pat::parse_multi_with_leading_vert(input)?;
                let guard = if input.peek(syn::Token![if]) { input.parse::<syn::Token![if]>()?; Some(input.parse()?) } else { None };
                if input.peek(syn::Token![,]) { input.parse::<syn::Token![,]>()?; }
                Ok((value, pattern, guard))
            };
            if let Ok((value, pattern, guard)) = parser.parse2(m.tokens.clone()) {
                self.macros.push(json!({"macro":"matches","position":position(m),"arguments_parsed_as_expressions":true,"parser":"expression-pattern-guard","tokens":tokens(&m.tokens)}));
                self.visit_expr(&value);
                let old = self.environment.clone();
                self.bind(&pattern, self.expr(&value));
                if let Some(guard) = &guard {
                    self.visit_expr(guard);
                }
                self.environment = old;
                return;
            }
        }
        let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let parsed = parser.parse2(m.tokens.clone());
        self.macros.push(json!({"macro":tokens(&m.path),"position":position(m),"arguments_parsed_as_expressions":parsed.is_ok(),"tokens":tokens(&m.tokens).chars().take(500).collect::<String>()}));
        if let Ok(exprs) = parsed {
            for expr in &exprs {
                self.visit_expr(expr)
            }
        }
    }
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if !test_only(&f.attrs) {
            self.inventory.raw.push(RawFunction {
                id: format!("{}::{}@{}", self.id, f.sig.ident, f.span().start().line),
                module: self.module.clone(),
                file: self.file.clone(),
                name: f.sig.ident.to_string(),
                owner: None,
                signature: f.sig.clone(),
                body: (*f.block).clone(),
                attributes: f.attrs.clone(),
            });
        }
    }
    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        visit::visit_expr_assign(self, a);
        if let syn::Expr::Path(p) = &*a.left {
            if p.path.segments.len() == 1 {
                let name = p.path.segments[0].ident.to_string();
                let new = self.expr(&a.right);
                let old = self
                    .environment
                    .get(&name)
                    .cloned()
                    .unwrap_or(json!({"kind":"unknown","text":"assignment"}));
                self.environment
                    .insert(name, json!({"kind":"union","values":[old,new]}));
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("source directory required")?);
    let mut inventory = Inventory::default();
    inventory.file(&root.join("lib.rs"), "crate", &root)?;
    let mut index = 0;
    while index < inventory.raw.len() {
        let raw = &inventory.raw[index];
        let id = raw.id.clone();
        let module = raw.module.clone();
        let file = raw.file.clone();
        let name = raw.name.clone();
        let owner = raw.owner.clone();
        let signature = raw.signature.clone();
        let body = raw.body.clone();
        let attributes = raw.attributes.clone();
        index += 1;
        let mut environment = BTreeMap::new();
        let mut params = vec![];
        let mut has_receiver = false;
        for input in &signature.inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    has_receiver = true;
                    environment.insert("self".into(),json!({"kind":"typed","type":{"path":[owner.as_deref().unwrap_or("Self")],"args":[]}}));
                }
                syn::FnArg::Typed(p) => {
                    let name = tokens(&p.pat).replace("mut ", "");
                    let t = ty(&p.ty);
                    let i = params.len();
                    params.push(json!({"name":name,"type":t}));
                    environment.insert(
                        name,
                        json!({"kind":"parameter","function":id,"index":i,"type":t}),
                    );
                }
            }
        }
        let mut visitor = Body {
            inventory: &mut inventory,
            id: id.clone(),
            module: module.clone(),
            file: file.clone(),
            owner: owner.clone(),
            environment,
            local_types: BTreeMap::new(),
            calls: vec![],
            macros: vec![],
        };
        visitor.visit_block(&body);
        let output = match &signature.output {
            syn::ReturnType::Default => json!(null),
            syn::ReturnType::Type(_, t) => ty(t),
        };
        let node = json!({"id":id,"module":module,"file":file,"name":name,"owner":owner,"context_owner":owner,"kind":"function","position":position(&signature),"end_position":end_position(&body),"body_tokens":tokens(&body),"params":params,"return_type":output,"has_receiver":has_receiver,"attributes":attrs(&attributes),"calls":visitor.calls,"macros":visitor.macros});
        inventory.functions.push(node);
    }
    println!(
        "{}",
        serde_json::to_string(
            &json!({"files":inventory.files,"functions":inventory.functions,"types":inventory.types,"imports":inventory.imports,"item_macros":inventory.macros,"unresolved_modules":inventory.unresolved_modules})
        )?
    );
    Ok(())
}
