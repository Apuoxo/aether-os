//! Native Win32 PE32 loader/runtime boundary for the Windows Personality.
//!
//! This module deliberately does not emulate a CPU or boot a second OS. 32-bit
//! Windows binaries execute in the x86 compatibility mode provided by the host
//! x86_64 CPU; the Aether kernel supplies the Windows ABI boundary.
//!
//! The first integrated loader stage handles the real PE image format, the
//! extracted Winamp dependency tree, section mapping and import enumeration.
//! Unsupported ABI calls are rejected before control is transferred to foreign
//! code so the native Aether desktop remains intact.

use crate::mm;
use crate::mm::paging;

const PAGE: usize = 4096;
const MAX_IMAGE_PAGES: usize = 4096;
const MAX_IMPORTS: usize = 2048;

#[derive(Clone, Copy)]
struct PeImage {
    base: usize,
    entry: usize,
    image_size: usize,
    headers_size: usize,
    pages: [usize; MAX_IMAGE_PAGES],
    page_count: usize,
    imports: usize,
    unresolved: usize,
}

impl PeImage {
    const fn empty() -> Self {
        Self { base: 0, entry: 0, image_size: 0, headers_size: 0,
            pages: [0; MAX_IMAGE_PAGES], page_count: 0, imports: 0, unresolved: 0 }
    }
}

fn u16(b: &[u8], p: usize) -> Option<u16> {
    if p.checked_add(2)? > b.len() { None } else { Some(u16::from_le_bytes([b[p],b[p+1]])) }
}
fn u32(b: &[u8], p: usize) -> Option<u32> {
    if p.checked_add(4)? > b.len() { None } else {
        Some(u32::from_le_bytes([b[p],b[p+1],b[p+2],b[p+3]]))
    }
}
fn bytes_eq(b: &[u8], p: usize, x: &[u8]) -> bool {
    p.checked_add(x.len()).map_or(false, |e| e <= b.len()) && &b[p..p+x.len()] == x
}
fn cstr(b: &[u8], p: usize) -> Option<&[u8]> {
    if p >= b.len() { return None; }
    let mut e=p;
    while e < b.len() && b[e] != 0 { e += 1; }
    if e == b.len() { None } else { Some(&b[p..e]) }
}

fn read_file(path: &str) -> Option<([u8; 8192], usize)> {
    let mut b=[0u8;8192];
    let n=crate::fs::read_large(path,&mut b)?;
    Some((b,n))
}

pub fn install_bundle(blob: &[u8]) -> bool {
    if blob.len() < 12 || &blob[0..8] != b"AETHW32\x01" { 
        crate::serial::write_str("[WIN32] dependency bundle header invalid\n");
        return false;
    }
    let count = u32::from_le_bytes([blob[8],blob[9],blob[10],blob[11]]) as usize;
    let mut p=12usize;
    let mut i=0usize;
    while i<count {
        if p.checked_add(8).map_or(true,|e|e>blob.len()) { return false; }
        let nl=u32::from_le_bytes([blob[p],blob[p+1],blob[p+2],blob[p+3]]) as usize;
        let dl=u32::from_le_bytes([blob[p+4],blob[p+5],blob[p+6],blob[p+7]]) as usize;
        p+=8;
        if p.checked_add(nl).and_then(|x|x.checked_add(dl)).map_or(true,|e|e>blob.len()) { return false; }
        let name=&blob[p..p+nl]; p+=nl;
        let data=&blob[p..p+dl]; p+=dl;
        let mut path=[0u8;160]; let mut n=0usize;
        let prefix=b"/Winamp/";
        while n<prefix.len() { path[n]=prefix[n]; n+=1; }
        if n+name.len()>=path.len() { return false; }
        let mut j=0usize; while j<name.len() { path[n+j]=name[j]; j+=1; }
        n+=name.len();
        let s=match core::str::from_utf8(&path[..n]) { Ok(v)=>v, Err(_)=>return false };
        if !crate::fs::write_large(s,data) { return false; }
        i+=1;
    }
    crate::serial::write_str("[WIN32] Winamp dependency tree installed count=");
    crate::serial::write_usize(count);
    crate::serial::write_str("\n");
    true
}

fn rva_to_file(rva: usize, sections: &[(usize,usize,usize,usize);96], nsec:usize) -> Option<usize> {
    let mut i=0usize;
    while i<nsec {
        let (va,vs,raw,rs)=sections[i];
        let span=if vs>rs {vs}else{rs};
        if rva>=va && rva-va<span { return raw.checked_add(rva-va); }
        i+=1;
    }
    None
}

fn map_pe(path: &str) -> Option<PeImage> {
    let (head,n)=read_file(path)?;
    if n<0x100 || head[0]!=b'M' || head[1]!=b'Z' { return None; }
    let pe=u32(&head,0x3c)? as usize;
    if !bytes_eq(&head,pe,b"PE\0\0") { return None; }
    let machine=u16(&head,pe+4)?;
    let nsec=u16(&head,pe+6)? as usize;
    let opt=u16(&head,pe+20)? as usize;
    if machine!=0x014c || opt<0xE0 || nsec==0 || nsec>96 { return None; }
    let oh=pe+24;
    if u16(&head,oh)? != 0x10B { return None; }
    let image_base=u32(&head,oh+28)? as usize;
    let image_size=u32(&head,oh+56)? as usize;
    let headers=u32(&head,oh+60)? as usize;
    let entry_rva=u32(&head,oh+16)? as usize;
    if image_size<0x1000 || image_size>0x10000000 || headers<0x200 || entry_rva>=image_size { return None; }

    let mut sections=[(0usize,0usize,0usize,0usize);96];
    let sh=oh+opt;
    let mut i=0usize;
    while i<nsec {
        let p=sh+i*40;
        let va=u32(&head,p+12)? as usize;
        let vs=u32(&head,p+8)? as usize;
        let raw=u32(&head,p+20)? as usize;
        let rs=u32(&head,p+16)? as usize;
        sections[i]=(va,vs,raw,rs);
        i+=1;
    }

    let cr3=unsafe{paging::create_user_pml4()}?;
    let mut img=PeImage::empty();
    img.base=image_base;
    img.image_size=(image_size+PAGE-1)&!(PAGE-1);
    img.headers_size=headers;

    let mut va=0usize;
    while va<img.image_size {
        if img.page_count>=MAX_IMAGE_PAGES { return None; }
        let phys=mm::alloc_page()?;
        mm::zero_pages(phys,1);
        if !unsafe{paging::map_page(cr3, image_base+va, phys,
            paging::PAGE_PRESENT|paging::PAGE_WRITE|paging::PAGE_USER)} { return None; }
        img.pages[img.page_count]=phys; img.page_count+=1; va+=PAGE;
    }

    // Copy PE headers.
    let copyh=if headers<n {headers}else{n};
    unsafe {
        let dst=(image_base as *mut u8);
        let mut j=0usize; while j<copyh { *dst.add(j)=head[j]; j+=1; }
    }

    // Copy section bodies. The whole installer/executable is available from AetherFS.
    // Stage the real executable once into a bounded loader buffer.
    if n > 32 * 1024 * 1024 { return None; }
    let mut filebuf=[0u8;32*1024*1024];
    let file_n=match crate::fs::read_large(path,&mut filebuf) { Some(x)=>x, None=>return None };
    i=0;
    while i<nsec {
        let (sva,_svs,sraw,srs)=sections[i];
        if sraw.checked_add(srs).map_or(true,|e|e>file_n) { return None; }
        unsafe {
            let d=(image_base+sva) as *mut u8;
            let mut j=0usize;
            while j<srs { *d.add(j)=filebuf[sraw+j]; j+=1; }
        }
        i+=1;
    }

    // Import directory (index 1): RVA + size.
    let imp_rva=u32(&head,dirs+8)? as usize;
    let imp_size=u32(&head,dirs+12)? as usize;
    if imp_rva!=0 && imp_size!=0 {
        let mut d=imp_rva;
        let end=imp_rva.saturating_add(imp_size);
        while d+20<=end && d+20<=image_size {
            let oft=unsafe{*((image_base+d) as *const u32)} as usize;
            let name=unsafe{*((image_base+d+12) as *const u32)} as usize;
            let ft=unsafe{*((image_base+d+16) as *const u32)} as usize;
            if oft==0 && name==0 && ft==0 { break; }
            img.imports+=1;
            let _dll=unsafe{cstr(core::slice::from_raw_parts((image_base+name) as *const u8,256),0)};
            let mut t=if oft!=0{oft}else{ft};
            while t!=0 {
                let val=unsafe{*((image_base+t) as *const u32)};
                if val==0 { break; }
                // No fabricated API address: every unresolved import is a hard gate.
                // This preserves desktop stability until the corresponding Win32 ABI
                // implementation exists.
                img.unresolved+=1;
                t+=4;
            }
            d+=20;
        }
    }

    // Base relocations (index 5) are parsed and applied for a non-preferred base.
    let rel_rva=u32(&head,dirs+40)? as usize;
    let rel_size=u32(&head,dirs+44)? as usize;
    if rel_rva!=0 && rel_size!=0 && image_base!=0x400000 {
        let delta=(image_base as isize - 0x400000isize) as isize;
        let mut p=rel_rva; let end=p.saturating_add(rel_size);
        while p+8<=end {
            let page=unsafe{*((image_base+p) as *const u32)} as usize;
            let size=unsafe{*((image_base+p+4) as *const u32)} as usize;
            if size<8 { break; }
            let count=(size-8)/2;
            let mut j=0usize;
            while j<count {
                let e=unsafe{*((image_base+p+8+j*2) as *const u16)};
                if e>>12==3 {
                    let at=page+(e&0x0fff) as usize;
                    unsafe {
                        let q=(image_base+at) as *mut u32;
                        *q=((*q as isize)+delta) as u32;
                    }
                }
                j+=1;
            }
            p+=size;
        }
    }

    img.entry=image_base+entry_rva;
    img.cr3=cr3;
    crate::serial::write_str("[WIN32] PE32 image mapped base=0x");
    crate::serial::write_hex(img.base);
    crate::serial::write_str(" entry=0x");
    crate::serial::write_hex(img.entry);
    crate::serial::write_str(" pages=");
    crate::serial::write_usize(img.page_count);
    crate::serial::write_str(" imports=");
    crate::serial::write_usize(img.imports);
    crate::serial::write_str(" unresolved=");
    crate::serial::write_usize(img.unresolved);
    crate::serial::write_str("\n");
    Some(img)
}

pub fn launch_winamp(path:&str)->bool {
    crate::serial::write_str("\n======== WINAMP NATIVE LOADER ========\n");
    let img=match map_pe(path) {
        Some(x)=>x,
        None=>{ crate::serial::write_str("[WIN32] PE32 load rejected safely\n"); return false; }
    };
    if img.unresolved!=0 {
        crate::serial::write_str("[WIN32] execution gate: unresolved Win32 imports; desktop preserved\n");
        crate::serial::write_str("[WIN32] real CPU handoff NOT attempted\n");
        return false;
    }
    crate::serial::write_str("[WIN32] all imports resolved; execution handoff is permitted by loader\n");
    crate::serial::write_hex(img.entry);
    crate::serial::write_str("\n");
    // The actual handoff is intentionally not reached until the Win32 ABI resolver
    // has supplied every import. This is the safety boundary protecting the native
    // desktop from an incompatible foreign call.
    false
}
