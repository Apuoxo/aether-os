//! Minimal PE64 compatibility entry point for the first Windows application.
//!
//! This deliberately stops before executing foreign machine code. The first safe
//! milestone is: install a real PE64 file, recognize its headers, and launch an
//! Aether window through the Windows-runtime boundary. CPU execution will be
//! added only after the Ring3 process path is independently verified.

static mut HELLO_VALID: bool = false;

const fn put16(a: &mut [u8; 1024], p: usize, v: u16) {
    a[p] = v as u8;
    a[p + 1] = (v >> 8) as u8;
}
const fn put32(a: &mut [u8; 1024], p: usize, v: u32) {
    a[p] = v as u8;
    a[p + 1] = (v >> 8) as u8;
    a[p + 2] = (v >> 16) as u8;
    a[p + 3] = (v >> 24) as u8;
}
const fn put64(a: &mut [u8; 1024], p: usize, v: u64) {
    put32(a, p, v as u32);
    put32(a, p + 4, (v >> 32) as u32);
}

const fn make_hello_exe() -> [u8; 1024] {
    let mut a = [0u8; 1024];
    a[0] = b'M'; a[1] = b'Z';
    put32(&mut a, 0x3c, 0x80);
    a[0x80] = b'P'; a[0x81] = b'E';
    put16(&mut a, 0x84, 0x8664); // AMD64
    put16(&mut a, 0x86, 1);      // one section
    put16(&mut a, 0x94, 0xF0);   // PE32+ optional header
    put16(&mut a, 0x96, 0x22);   // executable + large-address-aware

    let o = 0x98;
    put16(&mut a, o, 0x20B);      // PE32+
    put32(&mut a, o + 4, 0x100);  // code size
    put32(&mut a, o + 16, 0x1000);// entry RVA
    put32(&mut a, o + 20, 0x1000);// code RVA
    put64(&mut a, o + 24, 0x140000000);
    put32(&mut a, o + 32, 0x1000);
    put32(&mut a, o + 36, 0x200);
    put16(&mut a, o + 40, 6);
    put16(&mut a, o + 48, 6);
    put32(&mut a, o + 56, 0x2000);
    put32(&mut a, o + 60, 0x200);
    put16(&mut a, o + 68, 3);     // console
    put16(&mut a, o + 70, 0x8160);
    put64(&mut a, o + 72, 0x1000);
    put64(&mut a, o + 80, 0x1000);
    put64(&mut a, o + 88, 0x1000);
    put64(&mut a, o + 96, 0x1000);
    put32(&mut a, o + 108, 16);

    let s = 0x188;
    a[s] = b'.'; a[s+1] = b't'; a[s+2] = b'e'; a[s+3] = b'x';
    a[s+4] = b't';
    put32(&mut a, s + 8, 0x100);
    put32(&mut a, s + 12, 0x1000);
    put32(&mut a, s + 16, 0x200);
    put32(&mut a, s + 20, 0x200);
    put32(&mut a, s + 36, 0x60000020);

    // Entry bytes: mov eax,42; ret. They are not executed by this milestone.
    a[0x200] = 0xB8; a[0x201] = 42; a[0x206] = 0xC3;
    a
}

pub static HELLO_EXE: [u8; 1024] = make_hello_exe();

pub fn install_hello() -> bool {
    crate::fs::write_large("/hello.exe", &HELLO_EXE)
}

fn read16(b: &[u8], p: usize) -> u16 {
    (b[p] as u16) | ((b[p + 1] as u16) << 8)
}
fn read32(b: &[u8], p: usize) -> u32 {
    (b[p] as u32) | ((b[p + 1] as u32) << 8)
        | ((b[p + 2] as u32) << 16) | ((b[p + 3] as u32) << 24)
}

pub fn validate_file(path: &str) -> bool {
    let mut b = [0u8; 512];
    let n = match crate::fs::read_large(path, &mut b) {
        Some(n) if n >= 0x190 => n,
        _ => return false,
    };
    if b[0] != b'M' || b[1] != b'Z' { return false; }
    let pe = read32(&b, 0x3c) as usize;
    if pe + 24 > n || b[pe] != b'P' || b[pe + 1] != b'E' {
        return false;
    }
    let machine = read16(&b, pe + 4);
    let sections = read16(&b, pe + 6);
    let optional = read16(&b, pe + 20);
    let magic = read16(&b, pe + 24);
    machine == 0x8664 && sections > 0 && optional == 0xF0 && magic == 0x20B
}

pub fn launch_hello() -> bool {
    let ok = validate_file("/hello.exe");
    unsafe { HELLO_VALID = ok; }
    if ok {
        crate::serial::write_str("[WIN32] /hello.exe PE64 recognized; entering safe compatibility window\n");
    } else {
        crate::serial::write_str("[WIN32] /hello.exe PE validation FAILED\n");
    }
    ok
}

pub fn hello_valid() -> bool {
    unsafe { HELLO_VALID }
}
