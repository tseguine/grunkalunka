//! Replace the fixture's first, empty core-module slot with unchanged guest bytes.
//! The remaining adapter sections and their module indices are copied verbatim.
pub fn embed_guest(template: &str, guest: &[u8]) -> wasmtime::Result<Vec<u8>> {
    if !guest.starts_with(b"\0asm\x01\0\0\0") {
        wasmtime::bail!("expected a core module");
    }
    let mut length = u32::try_from(guest.len())?;
    // Parse only the small adapter text. A syntax error must not quote the guest.
    let adapter = wat::parse_str(template)?;
    let prefix = wat::parse_str("(component (core module))")?;
    let Some(suffix) = adapter.strip_prefix(prefix.as_slice()) else {
        wasmtime::bail!("document adapter must start with an unnamed empty core module");
    };
    let mut component = wat::parse_str("(component)")?;
    // Component core-module section: id 1, unsigned LEB128 byte length, raw module.
    // This deliberately handles one fixed fixture slot, not arbitrary rewriting.
    component.push(1);
    loop {
        let mut byte = (length & 0x7f) as u8;
        length >>= 7;
        if length != 0 {
            byte |= 0x80;
        }
        component.push(byte);
        if length == 0 {
            break;
        }
    }
    component.extend_from_slice(guest);
    component.extend_from_slice(suffix);
    Ok(component)
}
