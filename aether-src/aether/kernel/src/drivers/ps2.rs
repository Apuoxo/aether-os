//! PS/2 i8042 — keyboard + mouse (timeout-safe) + diagnostics

use crate::serial;

const DATA: u16 = 0x60;
const STATUS: u16 = 0x64;
const CMD: u16 = 0x64;
const STAT_OBF: u8 = 1;
const STAT_IBF: u8 = 2;
const STAT_MOUSE: u8 = 1 << 5;

static mut KBD_READY: bool = false;
static mut MOUSE_READY: bool = false;
static mut LAST_KEY: u8 = 0;
static mut SHIFT_DOWN: bool = false;
static mut NUMLOCK_ON: bool = true; // default ON for laptop usability
static mut EXTENDED: bool = false; // E0 prefix
static mut MX: i32 = 400;
static mut MY: i32 = 300;
static mut MB: u8 = 0;
static mut MOUSE_CYCLE: u8 = 0;
static mut MOUSE_BYTE: [u8; 3] = [0; 3];

// Diagnostics
static mut DIAG_CTRL: bool = false;
static mut DIAG_ACK: bool = false;
static mut DIAG_STREAM: bool = false;
static mut DIAG_PACKETS: u32 = 0;
static mut DIAG_BYTES: u32 = 0;
static mut DIAG_LAST_ACK: u8 = 0;
static mut DIAG_STATUS0: u8 = 0;

pub fn diag_controller_ok() -> bool { unsafe { DIAG_CTRL } }
pub fn diag_ack_ok() -> bool { unsafe { DIAG_ACK } }
pub fn diag_stream_ok() -> bool { unsafe { DIAG_STREAM } }
pub fn diag_packets() -> u32 { unsafe { DIAG_PACKETS } }
pub fn diag_bytes() -> u32 { unsafe { DIAG_BYTES } }
pub fn diag_last_ack() -> u8 { unsafe { DIAG_LAST_ACK } }

unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nostack, preserves_flags));
    v
}
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}

fn wait_ibf_clear() -> bool {
    let mut i = 0u32;
    while i < 50_000 {
        if unsafe { inb(STATUS) } & STAT_IBF == 0 {
            return true;
        }
        i += 1;
    }
    false
}
fn wait_obf() -> bool {
    let mut i = 0u32;
    while i < 50_000 {
        if unsafe { inb(STATUS) } & STAT_OBF != 0 {
            return true;
        }
        i += 1;
    }
    false
}

fn write_cmd(cmd: u8) -> bool {
    if !wait_ibf_clear() {
        return false;
    }
    unsafe { outb(CMD, cmd); }
    true
}
fn write_data(data: u8) -> bool {
    if !wait_ibf_clear() {
        return false;
    }
    unsafe { outb(DATA, data); }
    true
}
fn read_data() -> Option<u8> {
    if wait_obf() {
        Some(unsafe { inb(DATA) })
    } else {
        None
    }
}

fn flush_output() {
    let mut n = 0u32;
    while n < 64 {
        let st = unsafe { inb(STATUS) };
        if st & STAT_OBF == 0 {
            break;
        }
        let _ = unsafe { inb(DATA) };
        n += 1;
    }
}

fn mouse_write(val: u8) -> bool {
    if !write_cmd(0xD4) {
        return false;
    }
    write_data(val)
}

pub fn init() {
    serial::write_str("[PS/2] init\n");
    let st0 = unsafe { inb(STATUS) };
    unsafe { DIAG_STATUS0 = st0; }
    if st0 == 0xFF {
        serial::write_str("[PS2] controller FAIL (status=FF)\n");
        unsafe {
            DIAG_CTRL = false;
            DIAG_ACK = false;
            DIAG_STREAM = false;
        }
        return;
    }
    unsafe { DIAG_CTRL = true; }
    serial::write_str("[PS2] controller OK\n");

    let _ = write_cmd(0xAD);
    let _ = write_cmd(0xA7);
    flush_output();

    if !write_cmd(0x20) {
        serial::write_str("[PS2] cfg read FAIL\n");
        return;
    }
    let mut cfg = read_data().unwrap_or(0);
    // IRQ off, translation on, mouse clock enable
    cfg &= !3;
    cfg |= 1 << 6;
    cfg |= 1 << 1; // mouse clock
    cfg |= 1 << 0; // kbd clock
    // Also enable mouse interface in config (bit 5 = disable mouse? clear disable)
    cfg &= !(1 << 5); // enable mouse (0 = enabled)
    let _ = write_cmd(0x60);
    let _ = write_data(cfg);

    let _ = write_cmd(0xAE);
    let _ = write_data(0xFF);
    if let Some(a) = read_data() {
        serial::write_str("[PS/2] kbd ACK=0x");
        serial::write_usize(a as usize);
        serial::write_str("\n");
        unsafe { KBD_READY = true; }
    } else {
        serial::write_str("[PS/2] kbd timeout — continue\n");
        unsafe { KBD_READY = true; }
    }

    // Enable AUX
    let _ = write_cmd(0xA8);
    flush_output();

    // Mouse reset
    if mouse_write(0xFF) {
        let ack = read_data();
        let bat = read_data();
        let id = read_data();
        serial::write_str("[PS2] mouse reset ack=");
        if let Some(a) = ack {
            serial::write_hex(a as usize);
            unsafe { DIAG_LAST_ACK = a; }
        } else {
            serial::write_str("TIMEOUT");
        }
        serial::write_str(" bat=");
        if let Some(b) = bat {
            serial::write_hex(b as usize);
        } else {
            serial::write_str("?");
        }
        serial::write_str(" id=");
        if let Some(i) = id {
            serial::write_hex(i as usize);
        } else {
            serial::write_str("?");
        }
        serial::write_str("\n");

        let _ = mouse_write(0xF6); // defaults
        let _ = read_data();

        if mouse_write(0xF4) {
            // enable streaming
            if let Some(a) = read_data() {
                serial::write_str("[PS2] mouse ACK=");
                serial::write_hex(a as usize);
                serial::write_str("\n");
                unsafe {
                    DIAG_LAST_ACK = a;
                    DIAG_ACK = a == 0xFA;
                    DIAG_STREAM = a == 0xFA;
                    MOUSE_READY = true;
                }
                if a == 0xFA {
                    serial::write_str("[PS2] streaming OK\n");
                } else {
                    serial::write_str("[PS2] streaming FAIL (bad ACK)\n");
                }
            } else {
                serial::write_str("[PS2] mouse ACK FAIL (timeout)\n");
                unsafe {
                    DIAG_ACK = false;
                    DIAG_STREAM = false;
                    MOUSE_READY = true; // still poll — touchpads sometimes delay ACK
                }
            }
        } else {
            serial::write_str("[PS2] streaming FAIL (write)\n");
            unsafe {
                DIAG_ACK = false;
                DIAG_STREAM = false;
            }
        }
    } else {
        serial::write_str("[PS2] mouse ACK FAIL (no write)\n");
        unsafe {
            DIAG_ACK = false;
            DIAG_STREAM = false;
        }
    }
    serial::write_str("[PS/2] done\n");
}

pub fn poll() {
    unsafe {
        let mut n = 0u32;
        while n < 32 && (inb(STATUS) & STAT_OBF) != 0 {
            let st = inb(STATUS);
            let data = inb(DATA);
            if st & STAT_MOUSE != 0 {
                DIAG_BYTES += 1;
                // resync: bit3 of first byte should be 1
                if MOUSE_CYCLE == 0 && (data & 0x08) == 0 {
                    // skip garbage
                    n += 1;
                    continue;
                }
                MOUSE_BYTE[MOUSE_CYCLE as usize] = data;
                MOUSE_CYCLE += 1;
                if MOUSE_CYCLE >= 3 {
                    MOUSE_CYCLE = 0;
                    DIAG_PACKETS += 1;
                    MB = MOUSE_BYTE[0] & 7;
                    let dx = MOUSE_BYTE[1] as i8 as i32;
                    let dy = -(MOUSE_BYTE[2] as i8 as i32);
                    MX += dx;
                    MY += dy;
                    if MX < 0 {
                        MX = 0;
                    }
                    if MY < 0 {
                        MY = 0;
                    }
                    if MX > 799 {
                        MX = 799;
                    }
                    if MY > 599 {
                        MY = 599;
                    }
                }
            } else {
                LAST_KEY = data;
            }
            n += 1;
        }
    }
}

pub fn mouse_pos() -> (i32, i32) {
    unsafe { (MX, MY) }
}
pub fn mouse_buttons() -> u8 {
    unsafe { MB }
}
pub fn last_scancode() -> u8 {
    unsafe {
        let k = LAST_KEY;
        LAST_KEY = 0;
        k
    }
}

/// Process one Set-1 scancode: update modifiers, return ASCII for make codes only.
pub fn scancode_to_ascii(sc: u8) -> Option<u8> {
    unsafe {
        // Extended prefix E0
        if sc == 0xE0 {
            EXTENDED = true;
            return None;
        }
        let ext = EXTENDED;
        EXTENDED = false;

        let is_break = (sc & 0x80) != 0;
        let code = sc & 0x7F;

        // Shift make/break
        if code == 0x2A || code == 0x36 {
            SHIFT_DOWN = !is_break;
            return None;
        }
        // CapsLock ignore for now on break; NumLock toggle on make
        if code == 0x45 && !is_break && !ext {
            NUMLOCK_ON = !NUMLOCK_ON;
            return None;
        }
        // Ignore all break codes for character generation
        if is_break {
            return None;
        }

        let shift = SHIFT_DOWN;

        // Main keyboard digit row: 1 2 3 4 5 6 7 8 9 0
        // Set-1: 0x02..0x0B
        if code >= 0x02 && code <= 0x0B {
            let digits = b"1234567890";
            let shifted = b"!@#$%^&*()";
            let i = (code - 0x02) as usize;
            return Some(if shift { shifted[i] } else { digits[i] });
        }

        // Numpad (Set-1) — when NumLock ON emit digits; when OFF arrow-like ignored as chars
        // 7 8 9  0x47 0x48 0x49
        // 4 5 6  0x4B 0x4C 0x4D
        // 1 2 3  0x4F 0x50 0x51
        // 0 .    0x52 0x53
        // + - *  0x4E 0x4A 0x37
        if !ext {
            match code {
                0x47 => return if NUMLOCK_ON { Some(b'7') } else { None },
                0x48 => return if NUMLOCK_ON { Some(b'8') } else { None },
                0x49 => return if NUMLOCK_ON { Some(b'9') } else { None },
                0x4B => return if NUMLOCK_ON { Some(b'4') } else { None },
                0x4C => return if NUMLOCK_ON { Some(b'5') } else { None },
                0x4D => return if NUMLOCK_ON { Some(b'6') } else { None },
                0x4F => return if NUMLOCK_ON { Some(b'1') } else { None },
                0x50 => return if NUMLOCK_ON { Some(b'2') } else { None },
                0x51 => return if NUMLOCK_ON { Some(b'3') } else { None },
                0x52 => return if NUMLOCK_ON { Some(b'0') } else { None },
                0x53 => return if NUMLOCK_ON { Some(b'.') } else { None },
                0x4E => return Some(b'+'),
                0x4A => return Some(b'-'),
                0x37 => return Some(b'*'), // keypad *
                _ => {}
            }
        }
        // Keypad / often E0 0x35
        if ext && code == 0x35 {
            return Some(b'/');
        }
        // Keypad Enter E0 0x1C
        if ext && code == 0x1C {
            return Some(b'\n');
        }

        // Letters + common
        let ch = match code {
            0x1C => b'\n',
            0x0E => 0x08, // backspace
            0x39 => b' ',
            0x0F => b'\t',
            0x1E => b'a',
            0x30 => b'b',
            0x2E => b'c',
            0x20 => b'd',
            0x12 => b'e',
            0x21 => b'f',
            0x22 => b'g',
            0x23 => b'h',
            0x17 => b'i',
            0x24 => b'j',
            0x25 => b'k',
            0x26 => b'l',
            0x32 => b'm',
            0x31 => b'n',
            0x18 => b'o',
            0x19 => b'p',
            0x10 => b'q',
            0x13 => b'r',
            0x1F => b's',
            0x14 => b't',
            0x16 => b'u',
            0x2F => b'v',
            0x11 => b'w',
            0x2D => b'x',
            0x15 => b'y',
            0x2C => b'z',
            // punctuation main row
            0x0C => if shift { b'_' } else { b'-' },
            0x0D => if shift { b'+' } else { b'=' },
            0x1A => if shift { b'{' } else { b'[' },
            0x1B => if shift { b'}' } else { b']' },
            0x2B => if shift { b'|' } else { b'\\' },
            0x27 => if shift { b':' } else { b';' },
            0x28 => if shift { b'"' } else { b'\'' },
            0x29 => if shift { b'~' } else { b'`' },
            0x33 => if shift { b'<' } else { b',' },
            0x34 => if shift { b'>' } else { b'.' },
            0x35 => if shift { b'?' } else { b'/' },
            _ => return None,
        };
        if ch >= b'a' && ch <= b'z' && shift {
            return Some(ch - 32);
        }
        Some(ch)
    }
}
