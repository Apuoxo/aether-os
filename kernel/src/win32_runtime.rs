//! Win32 PE32 runtime boundary.
//!
//! Winamp is kept as the original PE32/x86 binary. This module owns the
//! compatibility boundary; it never fabricates a success or jumps into an
//! image whose ABI has not been resolved.

fn u16(b: &[u8], p: usize) -> Option<u16> {
    if p.checked_add(2)? > b.len() { None }
    else { Some(u16::from_le_bytes([b[p], b[p + 1]])) }
}
fn u32(b: &[u8], p: usize) -> Option<u32> {
    if p.checked_add(4)? > b.len() { None }
    else { Some(u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])) }
}
fn cstr(b: &[u8], p: usize) -> Option<&[u8]> {
    if p >= b.len() { return None; }
    let mut e = p;
    while e < b.len() && b[e] != 0 { e += 1; }
    if e == b.len() { None } else { Some(&b[p..e]) }
}

fn rva_to_file(
    rva: usize,
    sections: &[(usize, usize, usize, usize); 96],
    nsec: usize,
) -> Option<usize> {
    let mut i = 0;
    while i < nsec {
        let (va, vs, raw, rs) = sections[i];
        let span = if vs > rs { vs } else { rs };
        if rva >= va && rva - va < span {
            return raw.checked_add(rva - va);
        }
        i += 1;
    }
    None
}

/// Validate the real embedded PE and enumerate its imports. This is deliberately
/// a gate: no foreign instruction is executed until a complete Win32 ABI exists.
pub fn launch_winamp(_path: &str) -> bool {
    crate::serial::write_str("\n======== WINAMP NATIVE LOADER ========\n");

    let b = crate::winamp_builtin::WINAMP_EXE;
    if b.len() < 0x100 || b[0] != b'M' || b[1] != b'Z' {
        crate::serial::write_str("[WIN32] PE32 load rejected: invalid DOS header\n");
        return false;
    }

    let pe = match u32(b, 0x3c) {
        Some(v) => v as usize,
        None => return false,
    };
    if pe.checked_add(24).map_or(true, |e| e > b.len())
        || &b[pe..pe + 4] != b"PE\0\0" {
        crate::serial::write_str("[WIN32] PE32 load rejected: invalid NT header\n");
        return false;
    }

    let machine = match u16(b, pe + 4) { Some(v) => v, None => return false };
    let nsec = match u16(b, pe + 6) { Some(v) => v as usize, None => return false };
    let opt = match u16(b, pe + 20) { Some(v) => v as usize, None => return false };
    let oh = pe + 24;

    if machine != 0x014c || opt < 0xE0 || nsec == 0 || nsec > 96
        || u16(b, oh) != Some(0x10B) {
        crate::serial::write_str("[WIN32] PE32 load rejected: not PE32/x86\n");
        return false;
    }

    let image_base = match u32(b, oh + 28) { Some(v) => v as usize, None => return false };
    let image_size = match u32(b, oh + 56) { Some(v) => v as usize, None => return false };
    let entry = match u32(b, oh + 16) { Some(v) => v as usize, None => return false };
    let dirs = oh + 96;

    let mut sections = [(0usize, 0usize, 0usize, 0usize); 96];
    let sh = oh + opt;
    let mut i = 0;
    while i < nsec {
        let p = sh + i * 40;
        sections[i] = (
            match u32(b, p + 12) { Some(v) => v as usize, None => return false },
            match u32(b, p + 8) { Some(v) => v as usize, None => return false },
            match u32(b, p + 20) { Some(v) => v as usize, None => return false },
            match u32(b, p + 16) { Some(v) => v as usize, None => return false },
        );
        i += 1;
    }

    let imp_rva = match u32(b, dirs + 8) { Some(v) => v as usize, None => return false };
    let imp_size = match u32(b, dirs + 12) { Some(v) => v as usize, None => return false };
    let mut dlls = 0usize;
    let mut imports = 0usize;

    if imp_rva != 0 && imp_size != 0 {
        let mut d = match rva_to_file(imp_rva, &sections, nsec) {
            Some(v) => v,
            None => return false,
        };
        let end = d.saturating_add(imp_size).min(b.len());
        while d + 20 <= end {
            let oft = match u32(b, d) { Some(v) => v as usize, None => return false };
            let name_rva = match u32(b, d + 12) { Some(v) => v as usize, None => return false };
            let ft = match u32(b, d + 16) { Some(v) => v as usize, None => return false };
            if oft == 0 && name_rva == 0 && ft == 0 { break; }
            let name_off = match rva_to_file(name_rva, &sections, nsec) {
                Some(v) => v,
                None => return false,
            };
            let _dll = match cstr(b, name_off) { Some(v) => v, None => return false };
            dlls += 1;
            let thunk_rva = if oft != 0 { oft } else { ft };
            let mut t = match rva_to_file(thunk_rva, &sections, nsec) {
                Some(v) => v,
                None => return false,
            };
            while t + 4 <= b.len() {
                let v = match u32(b, t) { Some(x) => x, None => return false };
                if v == 0 { break; }
                imports += 1;
                t += 4;
            }
            d += 20;
        }
    }

    crate::serial::write_str("[WIN32] image_base=0x");
    crate::serial::write_hex(image_base);
    crate::serial::write_str(" image_size=");
    crate::serial::write_usize(image_size);
    crate::serial::write_str(" entry_rva=0x");
    crate::serial::write_hex(entry);
    crate::serial::write_str(" dlls=");
    crate::serial::write_usize(dlls);
    crate::serial::write_str(" imports=");
    crate::serial::write_usize(imports);
    crate::serial::write_str("\n");

    crate::serial::write_str("[WIN32] execution gate: Win32 API resolver not installed; CPU handoff NOT attempted\n");
    false
}
