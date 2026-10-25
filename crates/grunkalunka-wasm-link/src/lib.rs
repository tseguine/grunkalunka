//! Bounded composition for the runtime/generated core-Wasm profile, not a general linker.
#![forbid(unsafe_code)]

mod profile;
use std::convert::Infallible;
use wasm_encoder::{
    reencode::{Reencode, RoundtripReencoder},
    *,
};
use wasmparser::{ExternalKind, Parser, Payload};

#[derive(Clone, Copy, Debug)]
pub struct LinkLimits {
    /// Inclusive byte length of each input, checked before binary validation.
    pub max_input_bytes: usize,
    /// Inclusive final byte length; not an allocator capacity guarantee.
    pub max_output_bytes: usize,
    /// Inclusive metadata vector sizes and combined type/function/export/data counts.
    pub max_items: usize,
    /// Inclusive sum of generated data payload bytes; runtime data stays unchanged.
    pub max_generated_data_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkLimit {
    Input,
    Output,
    Items,
    Data,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LinkError {
    InvalidRuntime(String),
    InvalidGenerated(String),
    UnsupportedRuntime(&'static str),
    UnsupportedGenerated(&'static str),
    MissingImport { module: String, name: String },
    SignatureMismatch(String),
    DuplicateExport(String),
    Limit(LinkLimit),
    Encoding(String),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LinkError {}

// Prevalidation proves source index bounds. The profile excludes generated index
// spaces we do not map, and checked combined u32 counts bound each appended index.
struct AppMap {
    imports: Vec<u32>,
    function_base: u32,
    type_base: u32,
    data_base: u32,
}
impl Reencode for AppMap {
    type Error = Infallible;
    fn function_index(&mut self, index: u32) -> Result<u32, reencode::Error<Infallible>> {
        Ok(if let Some(&f) = self.imports.get(index as usize) {
            f
        } else {
            self.function_base + (index - self.imports.len() as u32)
        })
    }
    fn type_index(&mut self, index: u32) -> Result<u32, reencode::Error<Infallible>> {
        Ok(self.type_base + index)
    }
    fn data_index(&mut self, index: u32) -> Result<u32, reencode::Error<Infallible>> {
        Ok(self.data_base + index)
    }
}

fn encoding(e: impl std::fmt::Display) -> LinkError {
    LinkError::Encoding(e.to_string())
}

/// Compose the declared runtime/generated profile into one import-free module.
/// Quotas may reject before validation; this shares memory, not a sandbox boundary.
pub fn compose(runtime: &[u8], generated: &[u8], limits: LinkLimits) -> Result<Vec<u8>, LinkError> {
    if runtime.len() > limits.max_input_bytes || generated.len() > limits.max_input_bytes {
        return Err(LinkError::Limit(LinkLimit::Input));
    }
    wasmparser::Validator::new()
        .validate_all(runtime)
        .map_err(|e| LinkError::InvalidRuntime(e.to_string()))?;
    wasmparser::Validator::new()
        .validate_all(generated)
        .map_err(|e| LinkError::InvalidGenerated(e.to_string()))?;
    let r = profile::read(runtime, false, limits)?;
    let a = profile::read(generated, true, limits)?;
    if a.memory.unwrap().initial > r.memory.unwrap().initial {
        return Err(LinkError::UnsupportedGenerated(
            "memory import minimum exceeds runtime",
        ));
    }
    profile::items(r.types.len() + a.types.len(), limits)?;
    profile::items(r.functions.len() + a.functions.len(), limits)?;
    profile::items(r.exports.len() + a.exports.len(), limits)?;
    profile::items(r.data.len() + a.data.len(), limits)?;
    let mut map = AppMap {
        imports: Vec::new(),
        function_base: r.functions.len() as u32,
        type_base: r.types.len() as u32,
        data_base: r.data.len() as u32,
    };
    for &(module, name, ty) in &a.imports {
        let Some(&(ExternalKind::Func, index)) = r.exports.get(name) else {
            return Err(LinkError::MissingImport {
                module: module.into(),
                name: name.into(),
            });
        };
        if r.types[r.functions[index as usize] as usize] != a.types[ty as usize] {
            return Err(LinkError::SignatureMismatch(name.into()));
        }
        map.imports.push(index);
    }
    for name in a.exports.keys() {
        if r.exports.contains_key(name) {
            return Err(LinkError::DuplicateExport((*name).into()));
        }
    }
    if a.functions.is_empty() && a.exports.is_empty() && a.data.is_empty() {
        if runtime.len() > limits.max_output_bytes {
            return Err(LinkError::Limit(LinkLimit::Output));
        }
        return Ok(runtime.to_vec());
    }
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    let mut round = RoundtripReencoder;
    for p in Parser::new(0).parse_all(runtime) {
        match p.map_err(encoding)? {
            Payload::TypeSection(s) => round.parse_type_section(&mut types, s).map_err(encoding)?,
            Payload::FunctionSection(s) => {
                for t in s {
                    functions.function(t.map_err(encoding)?);
                }
            }
            Payload::ExportSection(s) => round
                .parse_export_section(&mut exports, s)
                .map_err(encoding)?,
            Payload::CodeSectionEntry(b) => {
                code.raw(b.as_bytes());
            }
            _ => {}
        }
    }
    for p in Parser::new(0).parse_all(generated) {
        match p.map_err(encoding)? {
            Payload::TypeSection(s) => map.parse_type_section(&mut types, s).map_err(encoding)?,
            Payload::FunctionSection(s) => {
                for t in s {
                    functions.function(map.type_index(t.map_err(encoding)?).map_err(encoding)?);
                }
            }
            Payload::ExportSection(s) => {
                for e in s {
                    let e = e.map_err(encoding)?;
                    exports.export(
                        e.name,
                        ExportKind::Func,
                        map.function_index(e.index).map_err(encoding)?,
                    );
                }
            }
            Payload::CodeSectionEntry(b) => {
                map.parse_function_body(&mut code, b).map_err(encoding)?
            }
            _ => {}
        }
    }
    let mut module = Module::new();
    if !types.is_empty() {
        module.section(&types);
    }
    if !functions.is_empty() {
        module.section(&functions);
    }
    for id in [4, 5, 6] {
        if let Some(data) = r.sections.get(&id) {
            module.section(&RawSection { id, data });
        }
    }
    module.section(&exports);
    if let Some(data) = r.sections.get(&9) {
        module.section(&RawSection { id: 9, data });
    }
    let mut data = DataSection::new();
    for segment in r.data.iter().chain(&a.data) {
        data.raw(segment);
    }
    module.section(&DataCountSection { count: data.len() });
    if !code.is_empty() {
        module.section(&code);
    }
    if !data.is_empty() {
        module.section(&data);
    }
    for data in r.customs {
        module.section(&RawSection { id: 0, data });
    }
    let bytes = module.finish();
    if bytes.len() > limits.max_output_bytes {
        return Err(LinkError::Limit(LinkLimit::Output));
    }
    wasmparser::Validator::new()
        .validate_all(&bytes)
        .map_err(encoding)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn function_offset_does_not_overflow_before_subtracting_imports() {
        let mut map = AppMap {
            imports: vec![0, 1],
            function_base: u32::MAX - 1,
            type_base: 0,
            data_base: 0,
        };
        assert_eq!(map.function_index(2).unwrap(), u32::MAX - 1);
    }
}
