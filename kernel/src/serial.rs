//! Early serial console (COM1) — non-blocking on real hardware without UART

mod ports {
    use core::arch::asm;

    pub unsafe fn outb(port: u16, val: u8) {
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
    }
    pub unsafe fn inb(port: u16) -> u8 {
        let val: u8;
        asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
        val
    }
}

static mut ENABLED: bool = false;

// Bounded runtime trace. The newest bytes are retained so diagnostics can be
// inspected from the terminal without requiring a network or persistent FS.
const TRACE_CAP: usize = 8192;
static mut TRACE: [u8; TRACE_CAP] = [0; TRACE_CAP];
static mut TRACE_HEAD: usize = 0;
static mut TRACE_LEN: usize = 0;

// Minimal polled AI bridge framing for the QEMU COM1 transport. The bridge sends
// AI_IN before the runtime has necessarily emitted a state-change AI_REQ, so the
// serial endpoint must consume that input and turn it into a bounded request.
const AI_RX_CAP: usize = 128;
static mut AI_RX: [u8; AI_RX_CAP] = [0; AI_RX_CAP];
static mut AI_RX_LEN: usize = 0;
static mut AI_REQ_ID: usize = 1;
static mut AI_POLLING: bool = false;

fn trace_push(b: u8) {
    unsafe {
        TRACE[TRACE_HEAD] = b;
        TRACE_HEAD = (TRACE_HEAD + 1) % TRACE_CAP;
        if TRACE_LEN < TRACE_CAP {
            TRACE_LEN += 1;
        }
    }
}

/// Copy the newest runtime trace bytes in chronological order.
pub fn trace_snapshot(out: &mut [u8]) -> usize {
    unsafe {
        let n = TRACE_LEN.min(out.len());
        if n == 0 { return 0; }
        let start = (TRACE_HEAD + TRACE_CAP - TRACE_LEN) % TRACE_CAP;
        let skip = TRACE_LEN - n;
        let mut i = 0usize;
        while i < n {
            out[i] = TRACE[(start + skip + i) % TRACE_CAP];
            i += 1;
        }
        n
    }
}

fn tx_byte_raw(b: u8) {
    unsafe {
        if !ENABLED { return; }
        use ports::*;
        let mut t = 0u32;
        while (inb(0x3F8 + 5) & 0x20) == 0 {
            t += 1;
            if t > 100_000 {
                ENABLED = false;
                return;
            }
        }
        outb(0x3F8, b);
    }
}

fn tx_str_raw(s: &str) {
    for &b in s.as_bytes() {
        if b == b'\n' { tx_byte_raw(b'\r'); }
        tx_byte_raw(b);
    }
}

fn ai_emit_request(payload: &[u8]) {
    unsafe {
        let id = AI_REQ_ID;
        AI_REQ_ID = AI_REQ_ID.wrapping_add(1);
        tx_str_raw("AI_REQ:REQ=R");
        let mut div = 1usize;
        let mut n = id;
        while n >= 10 { n /= 10; div *= 10; }
        let mut x = id;
        while div != 0 {
            tx_byte_raw(b'0' + (x / div) as u8);
            x %= div;
            div /= 10;
        }
        tx_str_raw(" TEXT=");
        for &b in payload {
            if b == b'\r' || b == b'\n' { tx_byte_raw(b' '); }
            else if b.is_ascii() { tx_byte_raw(b); }
        }
        tx_byte_raw(b'\r');
        tx_byte_raw(b'\n');
    }
}

fn ai_emit_ack(line: &[u8]) {
    // Preserve the request tag when present: AI_RES:REQ=R1:...
    tx_str_raw("AI_ACK:");
    let mut i = 7usize;
    while i < line.len() && line[i] != b':' && i < 40 {
        if line[i].is_ascii() { tx_byte_raw(line[i]); }
        i += 1;
    }
    tx_byte_raw(b'\r');
    tx_byte_raw(b'\n');
}

fn ai_process_line() {
    unsafe {
        if AI_RX_LEN >= 7 && &AI_RX[..7] == b"AI_IN:" {
            ai_emit_request(&AI_RX[7..AI_RX_LEN]);
        } else if AI_RX_LEN >= 7 && &AI_RX[..7] == b"AI_RES:" {
            ai_emit_ack(&AI_RX[..AI_RX_LEN]);
        }
        AI_RX_LEN = 0;
    }
}

/// Drain a bounded amount of COM1 RX data without ever blocking the kernel.
/// The bridge protocol is line-framed and deliberately capped at AI_RX_CAP.
fn poll_ai_bridge() {
    unsafe {
        if !ENABLED || AI_POLLING { return; }
        AI_POLLING = true;
        let mut budget = 64usize;
        while budget != 0 {
            use ports::*;
            if (inb(0x3F8 + 5) & 0x01) == 0 { break; }
            let b = inb(0x3F8);
            if b == b'\n' {
                ai_process_line();
            } else if b != b'\r' {
                if AI_RX_LEN < AI_RX_CAP {
                    AI_RX[AI_RX_LEN] = b;
                    AI_RX_LEN += 1;
                } else {
                    // Drop an overlong frame instead of retaining a partial command.
                    AI_RX_LEN = 0;
                }
            }
            budget -= 1;
        }
        AI_POLLING = false;
    }
}

pub fn init() {
    unsafe {
        use ports::*;
        // Probe: write SCRATCH and read back
        outb(0x3F8 + 7, 0xAE);
        let probe = inb(0x3F8 + 7);
        if probe != 0xAE {
            ENABLED = false;
            return;
        }
        outb(0x3F8 + 1, 0x00);
        outb(0x3F8 + 3, 0x80);
        outb(0x3F8 + 0, 0x03);
        outb(0x3F8 + 1, 0x00);
        outb(0x3F8 + 3, 0x03);
        outb(0x3F8 + 2, 0xC7);
        outb(0x3F8 + 4, 0x0B);
        ENABLED = true;

        // Verify the guest UART RX path independently of the external chardev.
        outb(0x3F8 + 4, 0x1B); // MCR loopback + normal modem bits
        outb(0x3F8, 0x55);
        let mut t = 0u32;
        let mut rx = 0u8;
        while t < 100_000 {
            if (inb(0x3F8 + 5) & 0x01) != 0 {
                rx = inb(0x3F8);
                break;
            }
            t += 1;
        }
        outb(0x3F8 + 4, 0x0B);
        write_str(if rx == 0x55 {
            "[SERIAL] RX_LOOPBACK=PASS\n"
        } else {
            "[SERIAL] RX_LOOPBACK=FAIL\n"
        });
    }
}

pub fn write_byte(b: u8) {
    poll_ai_bridge();
    unsafe {
        if !ENABLED {
            return;
        }
        trace_push(b);
    }
    tx_byte_raw(b);
}

pub fn read_byte() -> Option<u8> {
    unsafe {
        if !ENABLED { return None; }
        use ports::*;
        if (inb(0x3F8 + 5) & 0x01) == 0 { return None; }
        Some(inb(0x3F8))
    }
}

pub fn write_str(s: &str) {
    poll_ai_bridge();
    for &b in s.as_bytes() {
        if b == b'\n' {
            write_byte(b'\r');
        }
        write_byte(b);
    }
}

pub fn write_usize(n: usize) {
    if n == 0 {
        write_byte(b'0');
        return;
    }
    let mut tmp = n;
    let mut digits = 0;
    while tmp > 0 {
        digits += 1;
        tmp /= 10;
    }
    let mut div = 1usize;
    let mut i = 1;
    while i < digits {
        div *= 10;
        i += 1;
    }
    let mut x = n;
    while div > 0 {
        let d = x / div;
        write_byte(b'0' + d as u8);
        x %= div;
        div /= 10;
    }
}

pub fn write_hex(n: usize) {
    write_str("0x");
    if n == 0 {
        write_byte(b'0');
        return;
    }
    let mut started = false;
    let mut i = 16usize;
    while i > 0 {
        i -= 1;
        let d = ((n >> (i * 4)) & 0xf) as u8;
        if d != 0 || started || i == 0 {
            started = true;
            let c = if d < 10 { b'0' + d } else { b'a' + (d - 10) };
            write_byte(c);
        }
    }
}
