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
    write_str("======== WF NETWORK SURVEY ========\n");
    write_str("PURPOSE: collect native network hardware facts for the WiFi bring-up plan\n");
    write_str("MODE: native PCI probe; no firmware load, association, TX/RX, or disk write\n");
    write_str("PROBE: fresh read-only Intel WLAN PCI discovery is executed now\n");
    crate::drivers::wifi::survey();

    let wifi = crate::drivers::wifi::found();
    let wifi_ready = crate::drivers::wifi::ready();
    let wifi_fw = crate::drivers::wifi::needs_firmware();
    write_str("WIFI: ");
    write_str(if wifi { "FOUND" } else { "NOT-FOUND" });
    write_str(" PHASE1=");
    write_str(if wifi_ready { "READY" } else { "NOT-READY" });
    write_str(" FIRMWARE=");
    write_str(if wifi_fw { "REQUIRED" } else { "NOT-REQUIRED" });
    write_str("\n");

    if wifi {
        let (bus, dev, func) = crate::drivers::wifi::bus_dev_func();
        write_str("  INTEL WLAN PCI=");
        write_usize(bus as usize);
        write_str(":");
        write_usize(dev as usize);
        write_str(".");
        write_usize(func as usize);
        write_str(" VID:DID=8086:0887 SUB=4062\n");
        write_str("  BAR0=");
        write_hex(crate::drivers::wifi::bar0() as usize);
        write_str(" MMIO=");
        write_str(if crate::drivers::wifi::mmio_ready() { "MAPPED" } else { "NOT-MAPPED" });
        write_str("\n");
        write_str("  FW-CONTRACT=");
        write_str(crate::drivers::wifi::firmware_prefix());
        write_str(" API=5..6\n");
    }
    crate::drivers::wifi::probe_prerequisites();
    crate::drivers::wifi::probe_capabilities();
    write_str("  MSI_DECODE ENABLE=");
    write_str(if (crate::drivers::wifi::msi_ctrl() & 0x0001) != 0 { "YES" } else { "NO" });
    write_str(" MULTI=");
    write_hex(((crate::drivers::wifi::msi_ctrl() >> 1) & 0x7) as usize);
    write_str(" 64BIT=");
    write_str(if (crate::drivers::wifi::msi_ctrl() & 0x0080) != 0 { "YES" } else { "NO" });
    write_str("\n");
    write_str("  MSI_CTRL=");    write_hex(crate::drivers::wifi::msi_ctrl() as usize);
    write_str(" PCIE=");
    write_str(if crate::drivers::wifi::pcie_cap() { "YES" } else { "NO" });
    write_str(" LINK_STATUS=");
    write_hex(crate::drivers::wifi::pcie_link_status() as usize);
    write_str("\n");
    write_str("  PCIE_DEV_STATUS=");
    write_hex(crate::drivers::wifi::pcie_device_status() as usize);
    write_str("\n");
    write_str("  CAPS PTR=");
    write_hex(crate::drivers::wifi::cap_ptr() as usize);
    write_str(" PM=");
    write_str(if crate::drivers::wifi::cap_pm() { "YES" } else { "NO" });
    write_str(" MSI=");
    write_str(if crate::drivers::wifi::cap_msi() { "YES" } else { "NO" });
    write_str(" MSIX=");
    write_str(if crate::drivers::wifi::cap_msix() { "YES" } else { "NO" });
    write_str(" READ=");
    write_str(if crate::drivers::wifi::cap_chain_read() { "YES" } else { "NO" });
    write_str("\n");
    write_str("  PREREQ PCI_CMD=");
    write_hex(crate::drivers::wifi::pci_command() as usize);
    write_str(" STATUS=");
    write_hex(crate::drivers::wifi::pci_status() as usize);
    write_str(" IRQ_LINE=");
    write_usize(crate::drivers::wifi::irq_line() as usize);
    write_str(" IRQ_PIN=");
    write_usize(crate::drivers::wifi::irq_pin() as usize);
    write_str(" READ=");
    write_str(if crate::drivers::wifi::prerequisites_read() { "YES" } else { "NO" });
    write_str("\n");

    let eth = crate::drivers::net::eth_found();
    write_str("ETHERNET: ");
    write_str(if eth { "FOUND" } else { "NOT-FOUND" });
    write_str(" RTL8168=");
    write_str(if crate::drivers::net::eth_is_rtl() { "YES" } else { "NO" });
    write_str(" MAC=");
    write_str(if crate::drivers::net::eth_mac_ok() { "VALID" } else { "NOT-READ" });
    write_str(" LINK=");
    write_str(if crate::drivers::net::link_up() { "UP" } else { "NOT-CONFIRMED" });
    write_str("\n");

    if eth {
        write_str("  VID:DID=");
        write_hex(crate::drivers::net::eth_vid() as usize);
        write_str(":");
        write_hex(crate::drivers::net::eth_did() as usize);
        write_str(" BAR0=");
        write_hex(crate::drivers::net::eth_bar0() as usize);
        write_str("\n");
    }

    write_str("PLAN:\n");
    write_str("  1. Preserve exact PCI identity/BAR/MMIO evidence.\n");
    write_str("  2. Validate Intel 2230 reset/interrupt/firmware-loader prerequisites.\n");
    write_str("  3. Add native iwlwifi-2030 firmware loading from Aether storage.\n");
    write_str("  4. Initialize RX/TX rings and interrupt path; keep read-only diagnostics available.\n");
    write_str("  5. Only after hardware init, implement scan/auth/association and IP networking.\n");
    write_str("  6. Test each stage on AH532; do not claim WiFi until real packets pass.\n");
    crate::ai_agent::record_network_probe(if wifi { 1 } else { 0 }, if wifi_ready { 1 } else { 0 });
    write_str("AI-AGENT: network observation recorded for future native planning/state model\n");
    write_str("======== WF END ========\n");
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
