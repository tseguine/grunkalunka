use crate::{LinkError, LinkLimit, LinkLimits};
use std::{collections::BTreeMap, ops::Range};
use wasmparser::{ExternalKind, FuncType, MemoryType, Parser, Payload, TypeRef, ValType};

#[derive(Default)]
pub(crate) struct Info<'a> {
    pub types: Vec<FuncType>,
    pub functions: Vec<u32>,
    pub imports: Vec<(&'a str, &'a str, u32)>,
    pub exports: BTreeMap<&'a str, (ExternalKind, u32)>,
    pub memory: Option<MemoryType>,
    pub data: Vec<&'a [u8]>,
    pub sections: BTreeMap<u8, &'a [u8]>,
    pub customs: Vec<&'a [u8]>,
}

pub(crate) fn items(n: usize, limits: LinkLimits) -> Result<(), LinkError> {
    if n > limits.max_items || u32::try_from(n).is_err() {
        Err(LinkError::Limit(LinkLimit::Items))
    } else {
        Ok(())
    }
}

pub(crate) fn slice(bytes: &[u8], range: Range<u64>) -> Result<&[u8], LinkError> {
    let start = usize::try_from(range.start).map_err(|e| LinkError::Encoding(e.to_string()))?;
    let end = usize::try_from(range.end).map_err(|e| LinkError::Encoding(e.to_string()))?;
    bytes
        .get(start..end)
        .ok_or_else(|| LinkError::Encoding("section range outside input".into()))
}

pub(crate) fn read(bytes: &[u8], app: bool, limits: LinkLimits) -> Result<Info<'_>, LinkError> {
    let unsupported = |why| {
        if app {
            LinkError::UnsupportedGenerated(why)
        } else {
            LinkError::UnsupportedRuntime(why)
        }
    };
    let mut info = Info::default();
    let mut generated_data_bytes = 0usize;
    for p in Parser::new(0).parse_all(bytes) {
        let p = p.map_err(|e| LinkError::Encoding(e.to_string()))?;
        if !app
            && let Some((id, range)) = p.as_section()
            && id != 0
        {
            info.sections.insert(id, slice(bytes, range)?);
        }
        match p {
            Payload::Version {
                encoding: wasmparser::Encoding::Module,
                ..
            }
            | Payload::End(_) => {}
            Payload::TypeSection(s) => {
                items(s.count() as usize, limits)?;
                for group in s {
                    let group = group.map_err(|e| LinkError::Encoding(e.to_string()))?;
                    if group.is_explicit_rec_group() || group.types().len() != 1 {
                        return Err(unsupported("non-plain function types"));
                    }
                    let ty = group
                        .into_types()
                        .next()
                        .ok_or_else(|| unsupported("empty type group"))?;
                    if !ty.is_final
                        || !ty.supertype_idxs.is_empty()
                        || ty.composite_type.shared
                        || ty.composite_type.descriptor_idx.is_some()
                        || ty.composite_type.describes_idx.is_some()
                    {
                        return Err(unsupported("non-plain function type attributes"));
                    }
                    let wasmparser::CompositeInnerType::Func(t) = ty.composite_type.inner else {
                        return Err(unsupported("non-function type"));
                    };
                    if app
                        && t.params().iter().chain(t.results()).any(|t| {
                            !matches!(
                                t,
                                ValType::I32
                                    | ValType::I64
                                    | ValType::F32
                                    | ValType::F64
                                    | ValType::V128
                            )
                        })
                    {
                        return Err(unsupported("generated function types must be numeric"));
                    }
                    info.types.push(t);
                }
            }
            Payload::ImportSection(s) => {
                items(s.count() as usize, limits)?;
                // Section count measures compact groups, not logical imports.
                for (index, i) in s.into_imports().enumerate() {
                    items(index + 1, limits)?;
                    let i = i.map_err(|e| LinkError::Encoding(e.to_string()))?;
                    if !app {
                        return Err(unsupported("runtime imports"));
                    }
                    if i.module != "runtime" {
                        return Err(LinkError::MissingImport {
                            module: i.module.into(),
                            name: i.name.into(),
                        });
                    }
                    match i.ty {
                        TypeRef::Func(ty) => info.imports.push((i.module, i.name, ty)),
                        TypeRef::Memory(m)
                            if i.name == "memory"
                                && info.memory.is_none()
                                && !m.memory64
                                && !m.shared
                                && m.page_size_log2.is_none()
                                && m.maximum.is_none() =>
                        {
                            info.memory = Some(m)
                        }
                        _ => return Err(unsupported("generated import kind/memory profile")),
                    }
                }
            }
            Payload::FunctionSection(s) => {
                items(s.count() as usize + info.imports.len(), limits)?;
                for f in s {
                    info.functions
                        .push(f.map_err(|e| LinkError::Encoding(e.to_string()))?);
                }
            }
            Payload::ExportSection(s) => {
                items(s.count() as usize, limits)?;
                for e in s {
                    let e = e.map_err(|e| LinkError::Encoding(e.to_string()))?;
                    if app && e.kind != ExternalKind::Func {
                        return Err(unsupported("generated export kind"));
                    }
                    info.exports.insert(e.name, (e.kind, e.index));
                }
            }
            Payload::MemorySection(s) if !app => {
                items(s.count() as usize, limits)?;
                for m in s {
                    let m = m.map_err(|e| LinkError::Encoding(e.to_string()))?;
                    if info.memory.is_some() || m.memory64 || m.shared || m.page_size_log2.is_some()
                    {
                        return Err(unsupported("runtime memory profile"));
                    }
                    info.memory = Some(m);
                }
            }
            Payload::DataSection(s) => {
                items(s.count() as usize, limits)?;
                for d in s {
                    let d = d.map_err(|e| LinkError::Encoding(e.to_string()))?;
                    if app {
                        if !matches!(d.kind, wasmparser::DataKind::Passive) {
                            return Err(unsupported("generated data must be passive"));
                        }
                        generated_data_bytes = generated_data_bytes
                            .checked_add(d.data.len())
                            .ok_or(LinkError::Limit(LinkLimit::Data))?;
                        if generated_data_bytes > limits.max_generated_data_bytes {
                            return Err(LinkError::Limit(LinkLimit::Data));
                        }
                    }
                    info.data.push(slice(bytes, d.range)?);
                }
            }
            Payload::DataCountSection { count, .. } => items(count as usize, limits)?,
            Payload::TableSection(s) if !app => items(s.count() as usize, limits)?,
            Payload::GlobalSection(s) if !app => items(s.count() as usize, limits)?,
            Payload::ElementSection(s) if !app => items(s.count() as usize, limits)?,
            Payload::CodeSectionStart { count, .. } => items(count as usize, limits)?,
            Payload::CodeSectionEntry(_) => {}
            Payload::CustomSection(s) => {
                if app {
                    if s.name() != "name" {
                        return Err(unsupported("generated custom metadata"));
                    }
                } else {
                    if !matches!(s.name(), "name" | "producers" | "target_features") {
                        return Err(unsupported("runtime custom metadata"));
                    }
                    items(info.customs.len() + 1, limits)?;
                    info.customs.push(slice(bytes, s.range())?);
                }
            }
            _ => return Err(unsupported("module section")),
        }
    }
    if info.memory.is_none() {
        return Err(unsupported("one memory is required"));
    }
    if !app && info.exports.get("memory") != Some(&(ExternalKind::Memory, 0)) {
        return Err(unsupported("runtime must export memory zero as memory"));
    }
    Ok(info)
}
