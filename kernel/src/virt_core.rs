//! Virt Core — persistent Aether-side identity and observation boundary.
//!
//! The local model is deliberately not the identity. Virt Core owns the
//! identity, session, location and safety mode that are presented to any
//! reasoning backend. The backend can later be local Qwen or an external LLM.

use crate::serial;

static mut READY: bool = false;
static mut SESSION_ID: u32 = 0;

fn mix(mut x: u32, v: u32) -> u32 {
    x ^= v.wrapping_add(0x9E37_79B9).rotate_left(13);
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^ (x >> 16)
}

fn push(buf: &mut [u8; 96], pos: &mut usize, bytes: &[u8]) {
    let mut i = 0usize;
    while i < bytes.len() && *pos < 90 {
        buf[*pos] = bytes[i];
        *pos += 1;
        i += 1;
    }
}

fn push_hex(buf: &mut [u8; 96], pos: &mut usize, mut v: u32) {
    push(buf, pos, b"0x");
    let mut started = false;
    let mut i = 8usize;
    while i > 0 && *pos < 90 {
        i -= 1;
        let d = ((v >> (i * 4)) & 0xF) as u8;
        if d != 0 || started || i == 0 {
            started = true;
            buf[*pos] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
            *pos += 1;
        }
    }
}

/// Establish the identity of this booted Virt instance.
pub fn init(mbi: usize, kernel_end: usize) {
    let (year, month, day, hour, minute, second) = crate::time::rtc_read();
    let mut sid = 0xA37E_0001u32;
    sid = mix(sid, mbi as u32);
    sid = mix(sid, (mbi >> 32) as u32);
    sid = mix(sid, kernel_end as u32);
    sid = mix(sid, (kernel_end >> 32) as u32);
    sid = mix(sid, year as u32);
    sid = mix(sid, ((month as u32) << 24) | ((day as u32) << 16) | ((hour as u32) << 8) | minute as u32);
    sid = mix(sid, second as u32);
    unsafe {
        SESSION_ID = sid;
        READY = true;
    }
    serial::write_str("[VIRT CORE] ID=VIRT MODE=RO LOC=KERNEL BACKEND=EXTERNAL SESSION=");
    serial::write_hex(sid as usize);
    serial::write_str("\n");
}

pub fn ready() -> bool {
    unsafe { READY }
}

pub fn session_id() -> u32 {
    unsafe { SESSION_ID }
}

/// Append the authoritative Virt identity to a request sent to a reasoning backend.
/// This is deliberately compact because the transport currently limits AI_REQ to 90 bytes.
pub fn append_context(buf: &mut [u8; 96], pos: &mut usize) {
    push(buf, pos, b"VIRT=CORE MODE=RO LOC=TERM SID=");
    push_hex(buf, pos, session_id());
    push(buf, pos, b" PID=");
}
