//! Aether shell — VGA text console + serial + PS/2 (real HW usable)

use crate::serial;
use crate::mm;
use crate::fs;
use crate::drivers::ps2;

const VGA: usize = 0xB8000;
const COLS: usize = 80;
const ROWS: usize = 25;

static mut CUR_ROW: usize = 4;
static mut CUR_COL: usize = 0;

// True only while the GUI terminal invokes the single shared command executor.
static mut GUI_OUTPUT: bool = false;

fn vga_cell(row: usize, col: usize, ch: u8, attr: u8) {
    if row >= ROWS || col >= COLS {
        return;
    }
    unsafe {
        let p = (VGA + (row * COLS + col) * 2) as *mut u16;
        *p = ((attr as u16) << 8) | (ch as u16);
    }
}

fn vga_clear_row(row: usize) {
    let mut c = 0usize;
    while c < COLS {
        vga_cell(row, c, b' ', 0x07);
        c += 1;
    }
}

fn vga_scroll() {
    unsafe {
        // copy rows 4..24 up by one (keep markers 0-3)
        let mut r = 4usize;
        while r < ROWS - 1 {
            let mut c = 0usize;
            while c < COLS {
                let src = (VGA + ((r + 1) * COLS + c) * 2) as *const u16;
                let dst = (VGA + (r * COLS + c) * 2) as *mut u16;
                *dst = *src;
                c += 1;
            }
            r += 1;
        }
        vga_clear_row(ROWS - 1);
        CUR_ROW = ROWS - 1;
        CUR_COL = 0;
    }
}

fn vga_putc(ch: u8) {
    unsafe {
        if ch == b'\n' {
            CUR_COL = 0;
            CUR_ROW += 1;
            if CUR_ROW >= ROWS {
                vga_scroll();
            }
            return;
        }
        if ch == 0x08 {
            if CUR_COL > 0 {
                CUR_COL -= 1;
                vga_cell(CUR_ROW, CUR_COL, b' ', 0x07);
            }
            return;
        }
        if ch >= 32 && ch < 127 {
            vga_cell(CUR_ROW, CUR_COL, ch, 0x0F);
            CUR_COL += 1;
            if CUR_COL >= COLS {
                CUR_COL = 0;
                CUR_ROW += 1;
                if CUR_ROW >= ROWS {
                    vga_scroll();
                }
            }
        }
    }
}

fn vga_write(s: &[u8]) {
    let mut i = 0usize;
    while i < s.len() {
        vga_putc(s[i]);
        i += 1;
    }
}

fn vga_write_str(s: &str) {
    vga_write(s.as_bytes());
}

fn putc(c: u8) {
    unsafe {
        if GUI_OUTPUT {
            crate::desktop::terminal_write_char(c);
            return;
        }
    }
    vga_putc(c);
    if c == 10 {
        serial::write_str("\n");
        return;
    }
    if c >= 32 && c < 127 {
        unsafe {
            let mut t = 0u32;
            while t < 20_000 {
                let s: u8;
                core::arch::asm!("in al, dx", in("dx") 0x3FDu16, out("al") s, options(nostack, preserves_flags));
                if s & 0x20 != 0 {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nostack, preserves_flags));
                    break;
                }
                t += 1;
            }
        }
    }
}

fn write_str(s: &str) {
    unsafe {
        if GUI_OUTPUT {
            crate::desktop::terminal_write(s);
            return;
        }
    }
    vga_write_str(s);
    serial::write_str(s);
}

fn write_usize(v: usize) {
    unsafe {
        if !GUI_OUTPUT {
            serial::write_usize(v);
            return;
        }
    }
    if v == 0 { putc(b'0'); return; }
    let mut n = v;
    let mut d = [0u8; 20];
    let mut k = 0usize;
    while n > 0 { d[k] = b'0' + (n % 10) as u8; n /= 10; k += 1; }
    while k > 0 { k -= 1; putc(d[k]); }
}

fn write_hex(mut v: usize) {
    unsafe {
        if !GUI_OUTPUT {
            serial::write_hex(v);
            return;
        }
    }
    if v == 0 { putc(b'0'); return; }
    let mut d = [0u8; 16];
    let mut k = 0usize;
    while v > 0 {
        let x = (v & 0xF) as u8;
        d[k] = if x < 10 { b'0' + x } else { b'a' + x - 10 };
        v >>= 4;
        k += 1;
    }
    while k > 0 { k -= 1; putc(d[k]); }
}

fn serial_read_byte() -> Option<u8> {
    unsafe {
        let s: u8;
        core::arch::asm!("in al, dx", in("dx") 0x3FDu16, out("al") s, options(nostack, preserves_flags));
        if s & 1 == 0 {
            return None;
        }
        let b: u8;
        core::arch::asm!("in al, dx", in("dx") 0x3F8u16, out("al") b, options(nostack, preserves_flags));
        Some(b)
    }
}

fn eq(line: &[u8], s: usize, clen: usize, b: &[u8]) -> bool {
    if clen != b.len() {
        return false;
    }
    let mut i = 0usize;
    while i < clen {
        if line[s + i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

// COMMANDS INTENTIONALLY REMOVED FOR TERMINAL NULL TEST.
// WiFi implementation command only. All unrelated terminal commands remain removed.

fn cmd_wf() {
    // Single WF command: complete Wi-Fi bring-up + RX/scan diagnosis.
    // All output is routed to the desktop terminal; no Wi-Fi diagnostic
    // text is emitted to Serial while this command is running.
    crate::drivers::wifi::set_wf_gui_output(true);
    write_str("======== WF WIFI RX/SCAN DIAGNOSTIC ========\n");
    write_str("MODE: single desktop command; Serial output disabled\n");
    write_str("TARGET: Intel Centrino Wireless-N 2230 native DVM transport\n");
    write_str("STAGE: PCI -> reset -> firmware -> ALIVE -> CMDQ -> SCAN -> RX ring\n");

    crate::drivers::wifi::survey();
    write_str("PCI=");
    write_str(if crate::drivers::wifi::found() { "FOUND" } else { "NOT-FOUND" });
    write_str(" VID:DID=8086:0887 SUB=4062 BAR0=");
    write_hex(crate::drivers::wifi::bar0() as usize);
    write_str("\n");

    crate::drivers::wifi::probe_prerequisites();
    crate::drivers::wifi::probe_capabilities();
    write_str("MSI_CTRL=");
    write_hex(crate::drivers::wifi::msi_ctrl() as usize);
    write_str(" MSI=");
    write_str(if (crate::drivers::wifi::msi_ctrl() & 1) != 0 { "ON" } else { "OFF" });
    write_str(" FLR=");
    write_str(if crate::drivers::wifi::pcie_flr_supported() { "YES" } else { "NO" });
    write_str("\n");

    let reset_ok = crate::drivers::wifi::software_reset();
    write_str("RESET=");
    write_str(if reset_ok { "OK" } else { "FAIL" });
    write_str(" BEFORE=");
    write_hex(crate::drivers::wifi::reset_before() as usize);
    write_str(" AFTER=");
    write_hex(crate::drivers::wifi::reset_after() as usize);
    write_str("\n");

    let activate_ok = crate::drivers::wifi::activate_nic();
    write_str("ACTIVATE=");
    write_str(if activate_ok { "OK" } else { "FAIL" });
    write_str(" BEFORE=");
    write_hex(crate::drivers::wifi::activate_before() as usize);
    write_str(" AFTER=");
    write_hex(crate::drivers::wifi::activate_after() as usize);
    write_str("\n");

    let fw_ok = crate::drivers::wifi::load_firmware();
    write_str("FIRMWARE=");
    write_str(if fw_ok { "LOADED" } else { "FAIL" });
    write_str(" VER=");
    write_hex(crate::drivers::wifi::firmware_version() as usize);
    write_str(" INST=");
    write_usize(crate::drivers::wifi::firmware_inst_size() as usize);
    write_str(" DATA=");
    write_usize(crate::drivers::wifi::firmware_data_size() as usize);
    write_str("\n");

    let exec_ok = crate::drivers::wifi::start_firmware();
    let alive_ok = exec_ok && crate::drivers::wifi::alive_seen();
    write_str("FIRMWARE_EXEC=");
    write_str(if exec_ok { "STARTED" } else { "FAIL" });
    write_str(" ALIVE=");
    write_str(if crate::drivers::wifi::alive_seen() { "SEEN" } else { "NOT-SEEN" });
    write_str(" VALID=");
    write_hex(crate::drivers::wifi::alive_valid() as usize);
    write_str(" SUBTYPE=");
    write_usize(crate::drivers::wifi::alive_subtype() as usize);
    write_str("\n");

    let cmdq_ok = alive_ok && crate::drivers::wifi::init_command_queue();
    write_str("CMDQ=");
    write_str(if cmdq_ok { "READY" } else { "NOT-READY" });
    write_str("\n");

    if cmdq_ok {
        let scan_ok = crate::drivers::wifi::scan_24ghz();
        write_str("SCAN24=");
        write_str(if scan_ok { "SUBMITTED" } else { "NOT-SUBMITTED" });
        write_str("\n");
    } else {
        write_str("SCAN24=NOT-SUBMITTED\n");
    }

    crate::drivers::wifi::wf_post_scan_diagnostics();
    write_str("======== WF END ========\n");
    crate::drivers::wifi::set_wf_gui_output(false);
}
fn run_line(line: &[u8], len: usize) {
    let mut s = 0usize;
    while s < len && line[s] == b' ' { s += 1; }
    let mut e = len;
    while e > s && (line[e - 1] == b' ' || line[e - 1] == b'\r') { e -= 1; }
    let clen = e.saturating_sub(s);
    if eq(line, s, clen, b"WF") || eq(line, s, clen, b"wf") {
        cmd_wf();
    } else {
        write_str("unknown — WF only (WiFi implementation test)\n");
    }
}

pub fn run_command_from_gui(line: &[u8], len: usize) {
    unsafe { GUI_OUTPUT = true; }
    run_line(line, len);
    unsafe { GUI_OUTPUT = false; }
}

pub fn run() -> ! {
    // Preserve row 0 markers; start console at row 4
    unsafe {
        CUR_ROW = 4;
        CUR_COL = 0;
    }
    let mut r = 4usize;
    while r < ROWS {
        vga_clear_row(r);
        r += 1;
    }

    write_str("======== Aether Shell ========\n");
    write_str("SHELL BUILD MARKER = NO-COMMANDS-NULL-TEST\n");
    write_str("aether> ");

    let mut line = [0u8; 128];
    let mut len = 0usize;

    loop {
        if let Some(b) = serial_read_byte() {
            if b == b'\r' || b == b'\n' {
                write_str("\n");
                run_line(&line, len);
                len = 0;
                write_str("aether> ");
            } else if b == 0x08 || b == 0x7F {
                if len > 0 {
                    len -= 1;
                    putc(0x08);
                }
            } else if b >= 32 && b < 127 && len < 127 {
                line[len] = b;
                len += 1;
                putc(b);
            }
        }

        ps2::poll();
        let sc = ps2::last_scancode();
        if sc != 0 {
            if let Some(ch) = ps2::scancode_to_ascii(sc) {
                if ch == b'\n' {
                    write_str("\n");
                    run_line(&line, len);
                    len = 0;
                    write_str("aether> ");
                } else if ch == 0x08 {
                    if len > 0 {
                        len -= 1;
                        putc(0x08);
                    }
                } else if ch >= 32 && ch < 127 && len < 127 {
                    line[len] = ch;
                    len += 1;
                    putc(ch);
                }
            }
        }

        let mut d = 0u32;
        while d < 30 {
            d += 1;
        }
    }
}

pub fn run_demo_commands() {}
