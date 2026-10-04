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
    if clen != b.len() { return false; }
    let mut i = 0usize;
    while i < clen {
        let mut a = line[s + i];
        let mut e = b[i];
        if a >= b'A' && a <= b'Z' { a += b'a' - b'A'; }
        if e >= b'A' && e <= b'Z' { e += b'a' - b'A'; }
        if a != e { return false; }
        i += 1;
    }
    true
}

fn cmd_log() {
    let mut buf = [0u8; 8192];
    let n = serial::trace_snapshot(&mut buf);
    write_str("======== AETHER RUNTIME LOG ========\n");
    write_str("SOURCE=SERIAL TRACE RING (latest 8192 bytes)\n");
    if n == 0 {
        write_str("EMPTY\n");
    } else {
        let mut i = 0usize;
        while i < n {
            let b = buf[i];
            if b == b'\r' {
                // Serial CR is paired with LF; do not duplicate it in the GUI.
            } else if b >= 32 && b < 127 {
                putc(b);
            } else if b == b'\n' {
                putc(b'\n');
            } else {
                putc(b'.');
            }
            i += 1;
        }
        if n > 0 && buf[n - 1] != b'\n' {
            putc(b'\n');
        }
    }
    write_str("======== LOG END ========\n");
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
    write_str("WF-DIAG-REV=COMPACT-1\n");

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
fn aud_probe_one(codec: u8, node: u8, param: u16) -> crate::drivers::audio::HdaVerbDiag {
    // GET_PARAMETER is a 12-bit verb ID (F00h) plus an 8-bit payload.
    // The complete 20-bit verb field is therefore F00 | parameter.
    let verb20 = (0x000F_00u32 << 8) | (param as u32 & 0xFF);
    let d = crate::drivers::audio::hda_raw_verb(codec, node, verb20);
    write_str("C="); write_hex(codec as usize);
    write_str(" N="); write_hex(node as usize);
    write_str(" P="); write_hex(param as usize);
    write_str(" V="); write_hex(verb20 as usize);
    write_str(" OK="); write_str(if d.ok { "Y" } else { "N" });
    write_str(" RESP="); write_hex(d.response as usize);
    write_str("\n");
    d
}

fn aud_readback(codec: u8, node: u8, label: &str, verb20: u32) {
    let d = crate::drivers::audio::hda_raw_verb(codec, node, verb20);
    write_str(label);
    write_str("=");
    write_str(if d.ok { "OK:" } else { "ERR:" });
    write_hex(d.response as usize);
    write_str("\n");
}

fn cmd_aud_readback() {
    let codec = crate::drivers::audio::hda_codec();
    let pin = crate::drivers::audio::hda_analog_pin();
    let conv = crate::drivers::audio::hda_output_conv();

    write_str("======== AUD CODEC READBACK ========\n");
    write_str("CODEC="); write_hex(codec as usize);
    write_str(" PIN="); write_hex(pin as usize);
    write_str(" CONV="); write_hex(conv as usize);
    write_str("\n");

    if codec == 0 && pin == 0 && conv == 0 {
        write_str("READBACK=NO_ACTIVE_CODEC_PATH\n");
        write_str("======== AUD READBACK END ========\n");
        return;
    }

    if conv != 0 {
        aud_readback(codec, conv, "CONV_FMT", 0xA0000);
        aud_readback(codec, conv, "CONV_STREAM", 0xF06u32 << 8);
        aud_readback(codec, conv, "CONV_POWER", 0xF05u32 << 8);
        aud_readback(codec, conv, "CONV_AMP_L", 0xB0000 | 0xA000);
        aud_readback(codec, conv, "CONV_AMP_R", 0xB0000 | 0x8000);
    }

    if pin != 0 {
        aud_readback(codec, pin, "PIN_CTL", 0xF07u32 << 8);
        aud_readback(codec, pin, "PIN_POWER", 0xF05u32 << 8);
        aud_readback(codec, pin, "PIN_EAPD", 0xF0Cu32 << 8);
        aud_readback(codec, pin, "PIN_AMP_L", 0xB0000 | 0xA000);
        aud_readback(codec, pin, "PIN_AMP_R", 0xB0000 | 0x8000);

        // HDA topology evidence: connection-list length, entries, and
        // currently selected connection. F02 returns packed entries.
        let lp = crate::drivers::audio::hda_raw_verb(codec, pin, (0xF00u32 << 8) | 0x0E);
        if lp.ok {
            let raw = lp.response;
            let count = (raw & 0x7F).min(16) as usize;
            let long_form = (raw & 0x80) != 0;
            write_str("PIN_CONN_COUNT="); write_usize(count);
            write_str(" FORM="); write_str(if long_form { "LONG" } else { "SHORT" }); write_str("\n");

            let mut ci = 0usize;
            while ci < count {
                let base = if long_form { ci & !1 } else { ci & !3 };
                let d = crate::drivers::audio::hda_raw_verb(
                    codec, pin, (0xF02u32 << 8) | (base as u32)
                );
                if d.ok {
                    let nid = if long_form {
                        if ci & 1 == 0 { d.response & 0x7FFF } else { (d.response >> 16) & 0x7FFF }
                    } else {
                        (d.response >> ((ci - base) * 8)) & 0x7F
                    };
                    write_str("PIN_CONN["); write_usize(ci); write_str("]=");
                    write_hex(nid as usize); write_str("\n");
                }
                ci += 1;
            }
            aud_readback(codec, pin, "PIN_CONN_SEL", 0xF01u32 << 8);

            // Read-only topology evidence for the two upstream widgets.
            let mut ci = 0usize;
            while ci < 2 {
                let nid = if ci == 0 { 0x0C } else { 0x0D };
                write_str("UPSTREAM_NID="); write_hex(nid as usize); write_str("\n");
                let caps = crate::drivers::audio::hda_raw_verb(codec, nid, 0x000F0009);
                if caps.ok {
                    write_str("UPSTREAM_TYPE="); write_hex(((caps.response >> 20) & 0xF) as usize);
                    write_str(" CAPS="); write_hex(caps.response as usize); write_str("\n");
                }
                let lp = crate::drivers::audio::hda_raw_verb(codec, nid, (0xF00u32 << 8) | 0x0E);
                if lp.ok {
                    let raw = lp.response;
                    let count = (raw & 0x7F).min(16) as usize;
                    let long_form = (raw & 0x80) != 0;
                    write_str("UPSTREAM_CONN_COUNT="); write_usize(count);
                    write_str(" FORM="); write_str(if long_form { "LONG" } else { "SHORT" }); write_str("\n");
                    let mut j = 0usize;
                    while j < count {
                        let base = if long_form { j & !1 } else { j & !3 };
                        let d = crate::drivers::audio::hda_raw_verb(codec, nid, (0xF02u32 << 8) | base as u32);
                        if d.ok {
                            let v = if long_form {
                                if j & 1 == 0 { d.response & 0x7FFF } else { (d.response >> 16) & 0x7FFF }
                            } else {
                                (d.response >> ((j - base) * 8)) & 0x7F
                            };
                            write_str("UPSTREAM_CONN["); write_usize(j); write_str("]=");
                            write_hex(v as usize); write_str("\n");
                        }
                        j += 1;
                    }
                    aud_readback(codec, nid, "UPSTREAM_CONN_SEL", 0xF01u32 << 8);

                    // Mixer input/output amplifier state; read-only.
                    let mut ai = 0usize;
                    while ai < count {
                        let left = crate::drivers::audio::hda_raw_verb(
                            codec, nid, (0xBu32 << 16) | 0x2000 | ai as u32
                        );
                        let right = crate::drivers::audio::hda_raw_verb(
                            codec, nid, (0xBu32 << 16) | ai as u32
                        );
                        if left.ok {
                            write_str("UPSTREAM_AMP_IN_L["); write_usize(ai); write_str("]=");
                            write_hex(left.response as usize); write_str("\n");
                        }
                        if right.ok {
                            write_str("UPSTREAM_AMP_IN_R["); write_usize(ai); write_str("]=");
                            write_hex(right.response as usize); write_str("\n");
                        }
                        ai += 1;
                    }
                    let out_l = crate::drivers::audio::hda_raw_verb(
                        codec, nid, (0xBu32 << 16) | 0x8000 | 0x2000
                    );
                    let out_r = crate::drivers::audio::hda_raw_verb(
                        codec, nid, (0xBu32 << 16) | 0x8000
                    );
                    if out_l.ok {
                        write_str("UPSTREAM_AMP_OUT_L="); write_hex(out_l.response as usize); write_str("\n");
                    }
                    if out_r.ok {
                        write_str("UPSTREAM_AMP_OUT_R="); write_hex(out_r.response as usize); write_str("\n");
                    }
                }
                ci += 1;
            }
        } else {
            write_str("PIN_CONN=READ_ERROR\n");
        }
    }

    write_str("======== AUD READBACK END ========\n");
}

fn cmd_aud3() {
    let codec = crate::drivers::audio::hda_codec();
    let afg = crate::drivers::audio::hda_afg();

    write_str("======== AUD3 HDA OUTPUT PIN SCAN ========\n");
    write_str("CODEC="); write_hex(codec as usize);
    write_str(" AFG="); write_hex(afg as usize); write_str("\n");

    if !crate::drivers::audio::hda_mmio_ready() || codec > 3 || afg == 0 {
        write_str("SCAN=NOT_READY\n");
        write_str("======== AUD3 END ========\n");
        return;
    }

    let ar = aud_probe_one(codec, afg, 0x04);
    let start = ((ar.response >> 16) & 0xFF) as u8;
    let count = (ar.response & 0xFF) as usize;
    write_str("WIDGETS="); write_hex(start as usize);
    write_str("+"); write_usize(count); write_str("\n");

    let mut n = 0usize;
    let end = count.min(32);
    while n < end {
        let node = start.wrapping_add(n as u8);
        let caps = crate::drivers::audio::hda_raw_verb(
            codec, node, 0x000F0009
        );
        if caps.ok {
            let wtype = ((caps.response >> 20) & 0xF) as u8;
            if wtype == 4 {
                let pin_caps = aud_probe_one(codec, node, 0x000C);
                let is_output = pin_caps.ok && (pin_caps.response & (1 << 4)) != 0;
                if is_output {
                    let cfg = aud_probe_one(codec, node, 0x001C);
                    let device = if cfg.ok { ((cfg.response >> 20) & 0xF) as u8 } else { 0xFF };
                    write_str("PIN NID="); write_hex(node as usize);
                    write_str(" DEV="); write_hex(device as usize);
                    write_str(" CAPS="); write_hex(pin_caps.response as usize); write_str("\n");

                    aud_readback(codec, node, "  CTL", 0xF07u32 << 8);
                    aud_readback(codec, node, "  PWR", 0xF05u32 << 8);
                    aud_readback(codec, node, "  EAPD", 0xF0Cu32 << 8);
                    aud_readback(codec, node, "  AMP_L", 0xB0000 | 0xA000);
                    aud_readback(codec, node, "  AMP_R", 0xB0000 | 0x8000);

                    let lp = aud_probe_one(codec, node, 0x000E);
                    if lp.ok {
                        let conn_count = (lp.response & 0xFF).min(16) as usize;
                        write_str("  CONN_COUNT="); write_usize(conn_count); write_str("\n");
                        let mut ci = 0usize;
                        while ci < conn_count {
                            aud_readback(codec, node, "  CONN", (0xF02u32 << 8) | (ci as u32));
                            ci += 1;
                        }
                    }
                    aud_readback(codec, node, "  CONN_SEL", 0xF01u32 << 8);
                }
            }
        }
        n += 1;
    }

    write_str("======== AUD3 END ========\n");
}

fn cmd_aud_extended() {
    write_str("======== AUD HDA TOPOLOGY ========\n");
    if !crate::drivers::audio::hda_mmio_ready() {
        write_str("EXT_CAUSE=MMIO_NOT_READY\n======== AUD EXT END ========\n");
        return;
    }

    // First pass only: validate correctly encoded root parameters and locate
    // the Audio Function Group. Do not scan widgets until this succeeds.
    let codecs = [0u8, 3u8];
    let mut ci = 0usize;
    while ci < codecs.len() {
        let codec = codecs[ci];
        if (crate::drivers::audio::hda_statests() & (1u16 << codec)) == 0 {
            ci += 1;
            continue;
        }

        write_str("CODEC "); write_hex(codec as usize); write_str(": ");
        let vid = aud_probe_one(codec, 0, 0x00);
        let sub = aud_probe_one(codec, 0, 0x04);
        let fg = aud_probe_one(codec, 0, 0x05);

        let start = ((sub.response >> 16) & 0xFF) as u8;
        let count = (sub.response & 0xFF) as usize;
        write_str("ROOT="); write_hex(start as usize);
        write_str("+"); write_usize(count);

        let mut afg = 0u8;
        if fg.ok && (fg.response & 0xFF) == 1 {
            afg = 0;
        }

        // Parameter 05h is Function Group Type, not the root AFG selector.
        // Enumerate only the function-group range returned by root 04h.
        let mut n = 0usize;
        while n < count.min(16) {
            let node = start.wrapping_add(n as u8);
            let d = aud_probe_one(codec, node, 0x05);
            if d.ok && (d.response & 0xFF) == 1 {
                afg = node;
                break;
            }
            n += 1;
        }

        write_str(" AFG=");
        if afg != 0 {
            write_hex(afg as usize);
            let ar = aud_probe_one(codec, afg, 0x04);
            let ws = ((ar.response >> 16) & 0xFF) as usize;
            let wc = (ar.response & 0xFF) as usize;
            write_str(" WIDGETS="); write_hex(ws);
            write_str("+"); write_usize(wc);
        } else {
            write_str("NONE");
        }
        write_str("\n");
        let _ = vid;
        ci += 1;
    }

    write_str("RINGS CORB_RP="); write_hex(crate::drivers::audio::hda_corb_rp() as usize);
    write_str(" CORB_WP="); write_hex(crate::drivers::audio::hda_corb_wp() as usize);
    write_str(" RIRB_WP="); write_hex(crate::drivers::audio::hda_rirb_wp() as usize);
    write_str(" RSTS="); write_hex(crate::drivers::audio::hda_rirb_sts() as usize);
    write_str("\n======== AUD EXT END ========\n");
}

fn cmd_aud() {
    write_str("======== AUD HDA/PCM DIAGNOSTIC ========\n");
    write_str("HDA_FOUND=");
    write_str(if crate::drivers::audio::hda_found() { "YES" } else { "NO" });
    write_str(" MMIO=");
    write_str(if crate::drivers::audio::hda_mmio_ready() { "READY" } else { "NO" });
    write_str(" BAR0="); write_hex(crate::drivers::audio::hda_bar0() as usize); write_str("\n");

    write_str("GCAP="); write_hex(crate::drivers::audio::hda_gcap() as usize);
    write_str(" OSS="); write_usize(crate::drivers::audio::hda_oss() as usize);
    write_str(" ISS="); write_usize(crate::drivers::audio::hda_iss() as usize);
    write_str(" BSS="); write_usize(crate::drivers::audio::hda_bss() as usize); write_str("\n");

    write_str("STATESTS="); write_hex(crate::drivers::audio::hda_statests() as usize);
    write_str(" CODEC_SELECTED="); write_usize(crate::drivers::audio::hda_codec() as usize); write_str("\n");

    write_str("CTRL GCTL="); write_hex(crate::drivers::audio::hda_gctl() as usize);
    write_str(" INTSTS="); write_hex(crate::drivers::audio::hda_intsts() as usize);
    write_str(" WALCLK="); write_hex(crate::drivers::audio::hda_walclk() as usize); write_str("\n");

    write_str("RINGS CORBSIZE="); write_hex(crate::drivers::audio::hda_corb_size() as usize);
    write_str(" RIRBSIZE="); write_hex(crate::drivers::audio::hda_rirb_size() as usize);
    write_str(" CORBRP="); write_hex(crate::drivers::audio::hda_corb_rp() as usize);
    write_str(" CORBWP="); write_hex(crate::drivers::audio::hda_corb_wp() as usize); write_str("\n");
    write_str("CORBCTL="); write_hex(crate::drivers::audio::hda_corb_ctl() as usize);
    write_str(" RIRBWP="); write_hex(crate::drivers::audio::hda_rirb_wp() as usize);
    write_str(" RIRBCTL="); write_hex(crate::drivers::audio::hda_rirb_ctl() as usize);
    write_str(" RIRBSTS="); write_hex(crate::drivers::audio::hda_rirb_sts() as usize);
    write_str(" RINTCNT="); write_hex(crate::drivers::audio::hda_rintcnt() as usize); write_str("\n");

    write_str("VERB_MODE=CORB_RIRB OK=");
    write_str(if crate::drivers::audio::hda_verb_ok() { "YES" } else { "NO" });
    write_str(" LAST="); write_hex(crate::drivers::audio::hda_verb_last() as usize);
    write_str(" RESP="); write_hex(crate::drivers::audio::hda_verb_resp() as usize);
    write_str(" META="); write_hex(crate::drivers::audio::hda_verb_meta() as usize);
    write_str(" ICIS="); write_hex(crate::drivers::audio::hda_verb_icis() as usize); write_str("\n");

    write_str("CODEC0=");
    write_str(if crate::drivers::audio::hda_codec0_vid_did() != 0 { "FOUND" } else { "NONE" });
    write_str(" VID_DID="); write_hex(crate::drivers::audio::hda_codec0_vid_did() as usize);
    write_str(" AFG="); write_hex(crate::drivers::audio::hda_codec0_afg() as usize); write_str("\n");

    write_str("CODEC3=");
    write_str(if crate::drivers::audio::hda_codec3_vid_did() != 0 { "FOUND" } else { "NONE" });
    write_str(" VID_DID="); write_hex(crate::drivers::audio::hda_codec3_vid_did() as usize);
    write_str(" AFG="); write_hex(crate::drivers::audio::hda_codec3_afg() as usize); write_str("\n");

    write_str("AFG="); write_hex(crate::drivers::audio::hda_afg() as usize);
    write_str(" ANALOG_PIN="); write_hex(crate::drivers::audio::hda_analog_pin() as usize);
    write_str(" OUTPUT_CONV="); write_hex(crate::drivers::audio::hda_output_conv() as usize); write_str("\n");

    write_str("STREAM_READY=");
    write_str(if crate::drivers::audio::hda_stream_ready() { "YES" } else { "NO" });
    write_str(" RUNNING=");
    write_str(if crate::drivers::audio::hda_stream_running() { "YES" } else { "NO" });
    write_str(" BASE="); write_hex(crate::drivers::audio::hda_stream_base()); write_str("\n");

    write_str("FMT="); write_hex(crate::drivers::audio::hda_stream_format() as usize);
    write_str(" CTL="); write_hex(crate::drivers::audio::hda_stream_control() as usize);
    write_str(" STAT="); write_hex(crate::drivers::audio::hda_stream_status() as usize); write_str("\n");
    write_str("LPIB="); write_hex(crate::drivers::audio::hda_lpib() as usize);
    write_str(" CBL="); write_hex(crate::drivers::audio::hda_stream_cbl() as usize);
    write_str(" LVI="); write_hex(crate::drivers::audio::hda_stream_lvi() as usize); write_str("\n");
    write_str("DMA="); write_hex(crate::drivers::audio::hda_dma_phys());
    write_str(" BDL="); write_hex(crate::drivers::audio::hda_bdl_phys());
    write_str(" TOTAL="); write_usize(crate::drivers::audio::hda_dma_total());
    write_str(" NEXT="); write_usize(crate::drivers::audio::hda_dma_next()); write_str("\n");

    write_str("CAUSE=");
    if !crate::drivers::audio::hda_found() {
        write_str("HDA_NOT_FOUND");
    } else if !crate::drivers::audio::hda_mmio_ready() {
        write_str("MMIO_NOT_READY");
    } else if crate::drivers::audio::hda_afg() == 0 {
        write_str("AFG_NOT_FOUND");
    } else if crate::drivers::audio::hda_analog_pin() == 0 {
        write_str("NO_ANALOG_PIN");
    } else if crate::drivers::audio::hda_output_conv() == 0 {
        write_str("NO_OUTPUT_CONVERTER");
    } else if crate::drivers::audio::hda_oss() == 0 {
        write_str("NO_OUTPUT_STREAM");
    } else if !crate::drivers::audio::hda_stream_ready() {
        write_str("STREAM_NOT_SELECTED");
    } else {
        write_str("READY_FOR_PCM");
    }
    write_str("\n");
    write_str("======== AUD END ========\n");
    cmd_aud_extended();
}
fn cmd_usbtop() {
    crate::drivers::xhci::scan_usb_controllers();
    write_str("======== USB HOST CONTROLLER TOPOLOGY ========\n");
    let n = crate::drivers::xhci::diag_usbctl_count();
    write_str("COUNT="); write_usize(n); write_str("\n");
    let mut i = 0usize;
    while i < n {
        let bdf = crate::drivers::xhci::diag_usbctl_bdf(i);
        let id = crate::drivers::xhci::diag_usbctl_id(i);
        let cl = crate::drivers::xhci::diag_usbctl_class(i);
        let bar = crate::drivers::xhci::diag_usbctl_bar0(i);
        let bus = (bdf >> 8) & 0xFF;
        let devfn = bdf & 0xFF;
        let dev = (devfn >> 3) & 0x1F;
        let func = devfn & 7;
        let prog = (cl >> 8) & 0xFF;
        write_str("USB["); write_usize(i); write_str("] BDF=");
        write_usize(bus as usize); write_str(":"); write_usize(dev as usize); write_str(".");
        write_usize(func as usize); write_str(" VID:DID="); write_hex(id as usize);
        write_str(" IF="); write_hex(prog as usize);
        write_str(" BAR0="); write_hex(bar as usize);
        write_str(" TYPE=");
        write_str(if prog == 0x30 { "xHCI" } else { "EHCI" });
        write_str("\n");
        i += 1;
    }
    write_str("======== USB TOPOLOGY END ========\n");
}

fn cmd_mous() {
    // The desktop used to enumerate xHCI only during boot. On AH532 the
    // mouse is commonly plugged after that probe, so MOUS must first retry
    // enumeration when no USB device was found. This keeps MOUS a diagnostic
    // command while making the documented "plug mouse, then rerun" path real.
    if !crate::drivers::xhci::diag_dev_found() {
        write_str("MOUS: USB device not enumerated; re-probing xHCI...\n");
        crate::drivers::xhci::probe();
    }

    write_str("======== MOUS USB MOUSE DIAGNOSTIC ========\n");
    write_str("XHCI=");
    write_str(if crate::drivers::xhci::diag_xhci_ok() { "OK" } else { "NO" });
    write_str(" PORTS="); write_usize(crate::drivers::xhci::diag_ports() as usize);
    write_str(" CCS_MASK="); write_hex(crate::drivers::xhci::diag_ccs_mask() as usize);
    write_str(" PORTSC_LAST="); write_hex(crate::drivers::xhci::diag_portsc_last() as usize);
    write_str("\n");
    write_str("PRE_CCS_MASK="); write_hex(crate::drivers::xhci::diag_pre_ccs_mask() as usize);
    write_str(" PRE_PORTSC_LAST="); write_hex(crate::drivers::xhci::diag_pre_portsc_last() as usize);
    write_str(" DEV=");
    write_str(if crate::drivers::xhci::diag_dev_found() { "FOUND" } else { "NO" });
    write_str(" HID=");
    write_str(if crate::drivers::xhci::diag_hid_ok() { "OK" } else { "NO" });
    write_str(" MOUSE_IF=");
    write_str(if crate::drivers::xhci::diag_mouse_if() { "YES" } else { "NO" });
    write_str(" EP_IN=");
    write_str(if crate::drivers::xhci::diag_ep_in() { "YES" } else { "NO" });
    write_str("\n");

    write_str("LIVE=");
    write_str(if crate::drivers::xhci::diag_mouse_live() { "YES" } else { "NO" });
    write_str(" SLOT="); write_usize(crate::drivers::xhci::diag_mouse_slot() as usize);
    write_str(" DCI="); write_usize(crate::drivers::xhci::diag_mouse_dci() as usize);
    write_str(" DB="); write_hex(crate::drivers::xhci::diag_mouse_db());
    write_str("\n");

    write_str("EVENT_RING=");
    write_hex(crate::drivers::xhci::diag_mouse_er());
    write_str(" DEQ="); write_usize(crate::drivers::xhci::diag_mouse_er_deq());
    write_str(" CYCLE="); write_usize(crate::drivers::xhci::diag_mouse_er_cycle() as usize);
    write_str(" SIZE="); write_usize(crate::drivers::xhci::diag_mouse_er_size());
    write_str("\n");

    write_str("TRANSFER_RING=");
    write_hex(crate::drivers::xhci::diag_mouse_ep_ring());
    write_str(" ENQ="); write_usize(crate::drivers::xhci::diag_mouse_enq());
    write_str(" CYCLE="); write_usize(crate::drivers::xhci::diag_mouse_cycle() as usize);
    write_str("\n");

    write_str("REPORT_BUF=");
    write_hex(crate::drivers::xhci::diag_mouse_report());
    write_str(" LEN="); write_usize(crate::drivers::xhci::diag_mouse_report_len());
    write_str("\n");

    write_str("TRB_QUEUED="); write_usize(crate::drivers::xhci::diag_trb_queued() as usize);
    write_str(" COMPLETIONS="); write_usize(crate::drivers::xhci::diag_completions() as usize);
    write_str(" REPORTS="); write_usize(crate::drivers::xhci::diag_reports() as usize);
    write_str(" EVENTS="); write_usize(crate::drivers::xhci::diag_mouse_events() as usize);
    write_str("\n");

    if !crate::drivers::xhci::diag_xhci_ok() {
        write_str("CAUSE=XHCI_NOT_FOUND\n");
    } else if !crate::drivers::xhci::diag_dev_found() {
        write_str("CAUSE=USB_DEVICE_NOT_ENUMERATED\n");
    } else if !crate::drivers::xhci::diag_mouse_if() {
        write_str("CAUSE=NO_HID_BOOT_MOUSE_INTERFACE\n");
    } else if !crate::drivers::xhci::diag_ep_in() {
        write_str("CAUSE=NO_INTERRUPT_IN_ENDPOINT\n");
    } else if !crate::drivers::xhci::diag_mouse_live() {
        write_str("CAUSE=MOUSE_ENDPOINT_NOT_LIVE\n");
    } else if crate::drivers::xhci::diag_completions() == 0 {
        write_str("CAUSE=NO_TRANSFER_COMPLETIONS\n");
    } else if crate::drivers::xhci::diag_reports() == 0 {
        write_str("CAUSE=TRANSFER_COMPLETES_BUT_NO_REPORT_DELIVERY\n");
    } else {
        write_str("CAUSE=REPORTS_DELIVERED_CHECK_INPUT_PATH\n");
    }
    write_str("======== MOUS END ========\n");
}

fn cmd_kms5() {
    write_str("======== KMS STAGE5 ========\n");
    write_str("TARGET=Intel Gen6 primary plane\n");
    write_str("GEN="); write_usize(crate::drivers::intel_igpu::gen() as usize);
    write_str(" DID="); write_hex(crate::drivers::intel_igpu::did() as usize);
    write_str(" MMIO_READY="); write_str(if crate::drivers::intel_kms::ready() { "YES" } else { "NO" });
    write_str(" FORCEWAKE="); write_str(if crate::drivers::intel_kms::forcewake_ready() { "YES" } else { "NO" });
    write_str("\n");
    write_str("MMIO="); write_hex(crate::drivers::intel_kms::mmio_base());
    write_str(" GMADR="); write_hex(crate::drivers::intel_igpu::aperture_bar() as usize);
    write_str("\n");
    write_str("LFB="); write_hex(crate::fb::address());
    write_str(" W="); write_usize(crate::fb::width());
    write_str(" H="); write_usize(crate::fb::height());
    write_str(" PITCH="); write_usize(crate::fb::pitch());
    write_str(" BPP="); write_usize(crate::fb::bpp() as usize);
    write_str("\n");
    if crate::fb::address() != crate::drivers::intel_igpu::aperture_bar() as usize {
        write_str("RESULT=REFUSE LFB!=GMADR\n");
        write_str("SAFE=YES (no plane write)\n");
        write_str("======== KMS5 END ========\n");
        return;
    }
    let w = crate::fb::width() as u16;
    let h = crate::fb::height() as u16;
    write_str("ACTION=MODESET_CURRENT\n");
    let ok = crate::drivers::intel_kms::modeset_to(w, h);
    write_str("RESULT="); write_str(if ok { "PASS" } else { "FAIL" }); write_str("\n");
    write_str("SCANOUT="); write_str(if ok { "PRIMARY-PLANE" } else { "LFB-UNCHANGED" }); write_str("\n");
    write_str("======== KMS5 END ========\n");
}

fn cmd_video_info() {
    write_str("======== VIDEO GEN6 ========\n");
    write_str("GEN="); write_usize(crate::drivers::video::gen() as usize);
    write_str(" MMIO="); write_hex(crate::drivers::video::mmio_base());
    write_str(" APER="); write_hex(crate::drivers::video::aperture());
    write_str(" FB="); write_hex(crate::fb::address());
    write_str(" W="); write_usize(crate::graphics::width());
    write_str(" H="); write_usize(crate::graphics::height());
    write_str(" SCANOUT="); write_str(if crate::drivers::video::scanout_ready() { "YES" } else { "NO" });
    write_str("\n");
}
fn cmd_video_mode(w: u16, h: u16) {
    write_str("VIDEO MODESET ");
    write_usize(w as usize); write_str("x"); write_usize(h as usize); write_str("\n");
    let ok = crate::drivers::video::modeset_to(w, h);
    write_str("RESULT="); write_str(if ok { "PASS" } else { "FAIL" }); write_str("\n");
    write_str("FB="); write_hex(crate::fb::address());
    write_str(" W="); write_usize(crate::graphics::width());
    write_str(" H="); write_usize(crate::graphics::height()); write_str("\n");
}

fn pci_cfg_read32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let addr = 0x8000_0000u32 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((off as u32) & 0xFC);
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") addr, options(nostack, preserves_flags));
        let value: u32;
        core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") value, options(nostack, preserves_flags));
        value
    }
}

fn pci_cfg_write32(bus: u8, dev: u8, func: u8, off: u8, value: u32) {
    let addr = 0x8000_0000u32 | ((bus as u32) << 16) | ((dev as u32) << 11) |
        ((func as u32) << 8) | ((off as u32) & 0xFC);
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") addr, options(nostack, preserves_flags));
        core::arch::asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") value, options(nostack, preserves_flags));
    }
}

fn ethdiag_capabilities(bus: u8, dev: u8, func: u8, status: u16) {
    if (status & 0x10) == 0 {
        write_str("CAP_LIST=NO\n");
        return;
    }
    let mut ptr = ((pci_cfg_read32(bus, dev, func, 0x34) & 0xFF) as u8) & 0xFC;
    let mut steps = 0usize;
    let mut pcie = false;
    let mut msi = false;
    let mut msix = false;
    while ptr >= 0x40 && ptr < 0xFC && steps < 48 {
        let cap = pci_cfg_read32(bus, dev, func, ptr);
        let id = (cap & 0xFF) as u8;
        if id == 0 { break; }
        if id == 0x10 { pcie = true; }
        if id == 0x05 { msi = true; }
        if id == 0x11 { msix = true; }
        let next = ((cap >> 8) & 0xFF) as u8;
        if next == ptr { break; }
        ptr = next & 0xFC;
        steps += 1;
    }
    write_str("CAP_LIST=YES PCIE="); write_str(if pcie { "YES" } else { "NO" });
    write_str(" MSI="); write_str(if msi { "YES" } else { "NO" });
    write_str(" MSIX="); write_str(if msix { "YES" } else { "NO" });
    write_str(" COUNT="); write_usize(steps); write_str("\n");
    if pcie {
        let mut p = ((pci_cfg_read32(bus, dev, func, 0x34) & 0xFF) as u8) & 0xFC;
        let mut i = 0usize;
        while p >= 0x40 && p < 0xFC && i < 48 {
            let cap = pci_cfg_read32(bus, dev, func, p);
            if (cap & 0xFF) as u8 == 0x10 {
                let pcie_cap = pci_cfg_read32(bus, dev, func, p);
                let link = pci_cfg_read32(bus, dev, func, p.wrapping_add(0x0C));
                write_str("PCIE_CAP="); write_hex(p as usize);
                write_str(" TYPE="); write_hex(((pcie_cap >> 20) & 0xF) as usize);
                write_str(" LINKCAP="); write_hex(link as usize);
                write_str("\n");
                break;
            }
            let next = ((cap >> 8) & 0xFF) as u8;
            if next == p { break; }
            p = next & 0xFC;
            i += 1;
        }
    }
}


fn ethchip_read32(mmio: usize, off: usize) -> u32 {
    unsafe { core::ptr::read_volatile((mmio + off) as *const u32) }
}

fn ethchip_mdio_read(mmio: usize, phy_reg: u8) -> Option<u16> {
    unsafe {
        core::ptr::write_volatile(
            (mmio + 0x60) as *mut u32,
            ((phy_reg as u32) & 0x1F) << 16,
        );
    }
    for _ in 0..1_000_000usize {
        let v = ethchip_read32(mmio, 0x60);
        if (v & 0x8000_0000) != 0 {
            return Some((v & 0xFFFF) as u16);
        }
    }
    None
}

fn ethchip_mdio_write(mmio: usize, phy_reg: u8, value: u16) -> bool {
    unsafe {
        core::ptr::write_volatile(
            (mmio + 0x60) as *mut u32,
            0x8000_0000
                | (((phy_reg as u32) & 0x1F) << 16)
                | value as u32,
        );
    }
    for _ in 0..1_000_000usize {
        let v = ethchip_read32(mmio, 0x60);
        if (v & 0x8000_0000) == 0 {
            // RTL8168 hardware requires a short quiet interval after write
            // completion before issuing the next PHYAR command.
            for _ in 0..2_000usize { core::hint::spin_loop(); }
            return true;
        }
    }
    false
}

fn ethchip_map_page(phys: usize) -> bool {
    unsafe {
        let cr3 = crate::mm::paging::kernel_cr3();
        if !crate::mm::paging::map_page(
            cr3,
            phys & !0xFFF,
            phys & !0xFFF,
            crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_PCD,
        ) {
            return false;
        }
        crate::mm::paging::load_cr3(cr3);
        true
    }
}

fn cmd_ethchip() {
    write_str("======== ETHERNET CHIP IDENTIFICATION ========\n");
    write_str("MODE=READ_ONLY MMIO PROBE\n");
    let mut found = false;
    for bus in 0u8..=31 { for dev in 0u8..32 { for func in 0u8..8 {
        let id = pci_cfg_read32(bus, dev, func, 0x00);
        if id == 0 || id == 0xFFFF_FFFF { continue; }
        let classreg = pci_cfg_read32(bus, dev, func, 0x08);
        if ((classreg >> 24) & 0xFF) != 0x02 || ((classreg >> 16) & 0xFF) != 0x00 { continue; }
        if (id & 0xFFFF) != 0x10EC || ((id >> 16) & 0xFFFF) != 0x8168 { continue; }

        found = true;
        let bar_lo = pci_cfg_read32(bus, dev, func, 0x18);
        let bar_hi = pci_cfg_read32(bus, dev, func, 0x1C);
        let mmio = ((bar_hi as u64) << 32) | ((bar_lo & 0xFFFF_FFF0) as u64);

        write_str("BDF="); write_usize(bus as usize); write_str(":");
        write_usize(dev as usize); write_str("."); write_usize(func as usize); write_str("\n");
        write_str("PCI=10EC:8168 REV="); write_hex((classreg & 0xFF) as usize); write_str("\n");

        // DMA prerequisite: report PCI Command without changing it.
        let pci_cmd = pci_cfg_read32(bus, dev, func, 0x04) as u16;
        write_str("PCI_COMMAND="); write_hex(pci_cmd as usize);
        write_str(" MEMORY="); write_str(if (pci_cmd & 0x0002) != 0 { "ON" } else { "OFF" });
        write_str(" BUS_MASTER="); write_str(if (pci_cmd & 0x0004) != 0 { "ON" } else { "OFF" });
        write_str("\n");
        write_str("BAR2_MMIO="); write_hex(mmio as usize); write_str("\n");

        if mmio == 0 || (mmio & 0xFFF) != 0 {
            write_str("RESULT=NO_VALID_MMIO_BAR2\n");
            continue;
        }
        if !ethchip_map_page(mmio as usize) {
            write_str("RESULT=MMIO_MAP_FAIL\n");
            continue;
        }

        let txcfg = ethchip_read32(mmio as usize, 0x40);
        let xid = (txcfg >> 20) & 0xFCF;
        let txcfg_v2 = if xid == 0x7C8 { ethchip_read32(mmio as usize, 0x60B0) } else { 0 };
        let chipcmd = unsafe { core::ptr::read_volatile((mmio as usize + 0x37) as *const u8) };
        let phystat = unsafe { core::ptr::read_volatile((mmio as usize + 0x6C) as *const u8) };

        write_str("TXCONFIG="); write_hex(txcfg as usize); write_str("\n");
        write_str("XID="); write_hex(xid as usize); write_str("\n");
        if xid == 0x7C8 {
            write_str("TX_CONFIG_V2="); write_hex(txcfg_v2 as usize); write_str("\n");
        }
        write_str("CHIPCMD="); write_hex(chipcmd as usize);
        write_str(" PHYSTATUS="); write_hex(phystat as usize); write_str("\n");

        if (xid & 0x7C8) == 0x2C8 {
            write_str("VARIANT=RTL8168EVL/8111EVL\n");
        } else if xid == 0x380 {
            write_str("VARIANT=RTL8168B/8111B\n");
        } else if xid == 0x3C0 || xid == 0x3C2 || xid == 0x3C3 {
            write_str("VARIANT=RTL8168C/8111C\n");
        } else if xid == 0x3C8 || xid == 0x3C9 {
            write_str("VARIANT=RTL8168CP/8111CP\n");
        } else if xid == 0x280 || xid == 0x281 {
            write_str("VARIANT=RTL8168D/8111D\n");
        } else if xid == 0x28A || xid == 0x28B {
            write_str("VARIANT=RTL8168DP/8111DP\n");
        } else if xid == 0x7C8 {
            write_str("VARIANT=EXTENDED_XID\n");
            write_str("EXTENDED_VARIANT=REQUIRES_TX_CONFIG_V2_MAPPING\n");
        } else {
            write_str("VARIANT=UNKNOWN_XID\n");
        }

        let mac0 = unsafe { core::ptr::read_volatile((mmio as usize + 0x00) as *const u8) };
        let mac1 = unsafe { core::ptr::read_volatile((mmio as usize + 0x01) as *const u8) };
        let mac2 = unsafe { core::ptr::read_volatile((mmio as usize + 0x02) as *const u8) };
        let mac3 = unsafe { core::ptr::read_volatile((mmio as usize + 0x03) as *const u8) };
        let mac4 = unsafe { core::ptr::read_volatile((mmio as usize + 0x04) as *const u8) };
        let mac5 = unsafe { core::ptr::read_volatile((mmio as usize + 0x05) as *const u8) };
        write_str("MAC=");
        write_hex(mac0 as usize); write_str(":");
        write_hex(mac1 as usize); write_str(":");
        write_hex(mac2 as usize); write_str(":");
        write_hex(mac3 as usize); write_str(":");
        write_hex(mac4 as usize); write_str(":");
        write_hex(mac5 as usize); write_str("\n");

        // Read-only RTL8168 state registers. No reset, descriptor write, or DMA start.
        let cplus = ethchip_read32(mmio as usize, 0xE0);
        let rdsar_lo = ethchip_read32(mmio as usize, 0xE4);
        let tnpds_lo = ethchip_read32(mmio as usize, 0x20);
        let isr = unsafe { core::ptr::read_volatile((mmio as usize + 0x3E) as *const u16) };
        write_str("CPLUS_CMD="); write_hex(cplus as usize);
        write_str(" ISR="); write_hex(isr as usize); write_str("\n");
        write_str("RDSAR_LO="); write_hex(rdsar_lo as usize);
        write_str(" TNPDS_LO="); write_hex(tnpds_lo as usize); write_str("\n");

        // Read-only PHY/link snapshot. Do not touch PHY command/control registers.
        let phystat2 = unsafe { core::ptr::read_volatile((mmio as usize + 0x6C) as *const u8) };
        let phy1 = ethchip_read32(mmio as usize, 0x60);
        let phy2 = ethchip_read32(mmio as usize, 0x64);
        write_str("PHY_STATUS_2="); write_hex(phystat2 as usize);
        write_str(" PHYAR_BEFORE="); write_hex(phy1 as usize);
        write_str(" PHY_REG64_RAW="); write_hex(phy2 as usize); write_str("\n");

        // MDIO reads issue read transactions through PHYAR; no PHY register is written.
        let phy_bmcr = ethchip_mdio_read(mmio as usize, 0);
        let phy_bmsr_1 = ethchip_mdio_read(mmio as usize, 1);
        let phy_bmsr_2 = ethchip_mdio_read(mmio as usize, 1);
        let phy_id1 = ethchip_mdio_read(mmio as usize, 2);
        let phy_id2 = ethchip_mdio_read(mmio as usize, 3);
        let phy_anar = ethchip_mdio_read(mmio as usize, 4);
        let phy_anlpar = ethchip_mdio_read(mmio as usize, 5);

        write_str("MDIO_PHYID1="); match phy_id1 {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        write_str(" PHYID2="); match phy_id2 {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        write_str("\n");
        write_str("MDIO_BMCR="); match phy_bmcr {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        write_str(" BMSR1="); match phy_bmsr_1 {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        write_str(" BMSR2="); match phy_bmsr_2 {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        if let (Some(v), Some(w)) = (phy_bmsr_1, phy_bmsr_2) {
            write_str(" LINK="); write_str(if (v & 0x0004) != 0 && (w & 0x0004) != 0 { "UP" } else { "DOWN_OR_LATCHED" });
            write_str(" ANEG="); write_str(if (w & 0x0020) != 0 { "COMPLETE" } else { "INCOMPLETE" });
        }
        write_str("\n");
        write_str("MDIO_ANAR="); match phy_anar {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        write_str(" ANLPAR="); match phy_anlpar {
            Some(v) => write_hex(v as usize), None => write_str("TIMEOUT"),
        }
        write_str("\n");
        write_str("MDIO_READ=PASS PHY_WRITE=NO RESET=NO DMA_START=NO\n");

        write_str("ACTION=NONE RESET=NO DMA=NO TX=NO RX=NO\n");
        break;
    } if found { break; }}}
    if !found { write_str("RESULT=RTL8168_NOT_FOUND\n"); }
    write_str("======== ETHCHIP END ========\n");
}

fn cmd_ethdma() {
    write_str("======== RTL8168 DMA BRING-UP ========\n");
    write_str("MODE=REAL_HW DMA POLLING\n");
    let mut found = false;

    'outer: for bus in 0u8..=31 {
        for dev in 0u8..32 {
            for func in 0u8..8 {
                let id = pci_cfg_read32(bus, dev, func, 0x00);
                if id == 0 || id == 0xFFFF_FFFF { continue; }
                let classreg = pci_cfg_read32(bus, dev, func, 0x08);
                if ((classreg >> 24) & 0xFF) != 0x02 || ((classreg >> 16) & 0xFF) != 0x00 { continue; }
                if (id & 0xFFFF) != 0x10EC || ((id >> 16) & 0xFFFF) != 0x8168 { continue; }

                found = true;
                let bar_lo = pci_cfg_read32(bus, dev, func, 0x18);
                let bar_hi = pci_cfg_read32(bus, dev, func, 0x1C);
                let mmio = ((bar_hi as u64) << 32) | ((bar_lo & 0xFFFF_FFF0) as u64);
                write_str("BDF="); write_usize(bus as usize); write_str(":");
                write_usize(dev as usize); write_str("."); write_usize(func as usize); write_str("\n");
                write_str("BAR2_MMIO="); write_hex(mmio as usize); write_str("\n");
                if mmio == 0 || (mmio & 0xFFF) != 0 || !ethchip_map_page(mmio as usize) {
                    write_str("RESULT=MMIO_FAIL\n");
                    break 'outer;
                }

                let mut pci_cmd = pci_cfg_read32(bus, dev, func, 0x04);
                write_str("PCI_COMMAND_OLD="); write_hex((pci_cmd & 0xFFFF) as usize); write_str("\n");
                if (pci_cmd & 0x0002) == 0 {
                    write_str("MEMORY=OFF RESULT=ABORT_NO_MMIO_DECODE\n");
                    break 'outer;
                }
                if (pci_cmd & 0x0004) == 0 {
                    let new_cmd = pci_cmd | 0x0004;
                    pci_cfg_write32(bus, dev, func, 0x04, new_cmd);
                    pci_cmd = pci_cfg_read32(bus, dev, func, 0x04);
                    write_str("BUS_MASTER=ENABLED_RMW\n");
                }
                write_str("PCI_COMMAND_NOW="); write_hex((pci_cmd & 0xFFFF) as usize);
                write_str(" MEMORY="); write_str(if (pci_cmd & 0x0002) != 0 { "ON" } else { "OFF" });
                write_str(" BUS_MASTER="); write_str(if (pci_cmd & 0x0004) != 0 { "ON" } else { "OFF" }); write_str("\n");
                if (pci_cmd & 0x0006) != 0x0006 {
                    write_str("RESULT=PCI_DMA_PERMISSION_FAIL\n");
                    break 'outer;
                }

                // RTL8168EVL/8111EVL family check.
                let txcfg = ethchip_read32(mmio as usize, 0x40);
                let xid = (txcfg >> 20) & 0xFCF;
                write_str("TXCONFIG="); write_hex(txcfg as usize); write_str(" XID="); write_hex(xid as usize); write_str("\n");
                if (xid & 0x7C8) != 0x2C8 {
                    write_str("RESULT=XID_UNSUPPORTED\n");
                    break 'outer;
                }

                // Stop/reset the NIC before giving it new descriptor ownership.
                unsafe { core::ptr::write_volatile((mmio as usize + 0x37) as *mut u8, 0x10); }
                let mut reset_ok = false;
                for _ in 0..200_000usize {
                    let v = unsafe { core::ptr::read_volatile((mmio as usize + 0x37) as *const u8) };
                    if v & 0x10 == 0 { reset_ok = true; break; }
                    core::hint::spin_loop();
                }
                write_str("RESET="); write_str(if reset_ok { "PASS" } else { "TIMEOUT" }); write_str("\n");
                if !reset_ok { break 'outer; }

                // One 4 KiB page per ring and per packet buffer. alloc_pages(1)
                // returns a page-aligned physical address; PMM allocates only
                // free pages from its managed physical range.
                let tx_ring = match crate::mm::alloc_pages(1) { Some(v) => v, None => { write_str("TX_RING_ALLOC=FAIL\n"); break 'outer; } };
                let rx_ring = match crate::mm::alloc_pages(1) { Some(v) => v, None => { write_str("RX_RING_ALLOC=FAIL\n"); break 'outer; } };
                let mut tx_bufs = [0usize; 1];
                let mut rx_bufs = [0usize; 8];

                tx_bufs[0] = match crate::mm::alloc_pages(1) { Some(v) => v, None => { write_str("TX_BUF_ALLOC=FAIL\n"); break 'outer; } };
                let mut ri = 0usize;
                while ri < rx_bufs.len() {
                    rx_bufs[ri] = match crate::mm::alloc_pages(1) {
                        Some(v) => v,
                        None => { write_str("RX_BUF_ALLOC=FAIL\n"); break 'outer; }
                    };
                    ri += 1;
                }

                // Make the PMM physical pages reachable through the kernel's
                // identity map before clearing/programming them.
                unsafe {
                    if !crate::mm::paging::map_page(
                        crate::mm::paging::kernel_cr3(), tx_ring, tx_ring,
                        crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE) ||
                    !crate::mm::paging::map_page(
                        crate::mm::paging::kernel_cr3(), rx_ring, rx_ring,
                        crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE) {
                        write_str("DMA_PAGE_MAP=FAIL\n");
                        break 'outer;
                    }
                    let mut ok = true;
                    if !crate::mm::paging::map_page(
                        crate::mm::paging::kernel_cr3(), tx_bufs[0], tx_bufs[0],
                        crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE) { ok = false; }
                    let mut i = 0usize;
                    while i < rx_bufs.len() {
                        if !crate::mm::paging::map_page(
                            crate::mm::paging::kernel_cr3(), rx_bufs[i], rx_bufs[i],
                            crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE) { ok = false; }
                        i += 1;
                    }
                    if !ok {
                        write_str("DMA_PAGE_MAP=FAIL\n");
                        break 'outer;
                    }
                    crate::mm::paging::load_cr3(crate::mm::paging::kernel_cr3());
                }

                let mut dma_mem_ok = tx_ring < 0x1_0000_0000 && rx_ring < 0x1_0000_0000 &&
                    (tx_ring & 0xFF) == 0 && (rx_ring & 0xFF) == 0;
                if dma_mem_ok {
                    dma_mem_ok = tx_bufs[0] < 0x1_0000_0000 && (tx_bufs[0] & 0xFF) == 0;
                    let mut i = 0usize;
                    while i < rx_bufs.len() {
                        if rx_bufs[i] >= 0x1_0000_0000 || (rx_bufs[i] & 0xFF) != 0 { dma_mem_ok = false; }
                        i += 1;
                    }
                }
                write_str("DMA_MEMORY="); write_str(if dma_mem_ok { "PASS" } else { "FAIL" }); write_str("\n");
                write_str("TX_RING_PHYS="); write_hex(tx_ring); write_str(" RX_RING_PHYS="); write_hex(rx_ring); write_str("\n");
                write_str("TX_BUF_PHYS="); write_hex(tx_bufs[0]); write_str("\n");
                write_str("RX_BUF0_PHYS="); write_hex(rx_bufs[0]); write_str(" RX_BUF7_PHYS="); write_hex(rx_bufs[7]); write_str("\n");
                if !dma_mem_ok { break 'outer; }

                crate::mm::zero_pages(tx_ring, 1);
                crate::mm::zero_pages(rx_ring, 1);
                crate::mm::zero_pages(tx_bufs[0], 1);
                let mut i = 0usize;
                while i < rx_bufs.len() { crate::mm::zero_pages(rx_bufs[i], 1); i += 1; }

                // Build RX descriptors: OWN remains with NIC; last descriptor
                // carries RingEnd. Descriptor is 16 bytes: opts1/opts2/addr_lo/addr_hi.
                let rx_desc = rx_ring as *mut u32;
                i = 0;
                while i < rx_bufs.len() {
                    unsafe {
                        let d = rx_desc.add(i * 4);
                        core::ptr::write_volatile(d.add(0), 0x8000_0000 | 2048 |
                            if i + 1 == rx_bufs.len() { 0x4000_0000 } else { 0 });
                        core::ptr::write_volatile(d.add(1), 0);
                        core::ptr::write_volatile(d.add(2), rx_bufs[i] as u32);
                        core::ptr::write_volatile(d.add(3), (rx_bufs[i] >> 32) as u32);
                    }
                    i += 1;
                }

                // One broadcast ARP request (probe form: sender IP 0.0.0.0,
                // target IP 0.0.0.0) proves the TX DMA path without guessing
                // the physical LAN gateway address.
                let tx = tx_bufs[0] as *mut u8;
                unsafe {
                    let mut j = 0usize;
                    while j < 60 { *tx.add(j) = 0; j += 1; }
                    let mut j = 0usize;
                    while j < 6 { *tx.add(j) = 0xFF; *tx.add(6 + j) = unsafe { core::ptr::read_volatile((mmio as usize + j) as *const u8) }; j += 1; }
                    *tx.add(12) = 0x08; *tx.add(13) = 0x06;
                    *tx.add(14) = 0x00; *tx.add(15) = 0x01;
                    *tx.add(16) = 0x08; *tx.add(17) = 0x00;
                    *tx.add(18) = 0x06; *tx.add(19) = 0x04;
                    *tx.add(20) = 0x00; *tx.add(21) = 0x01;
                    let mut j = 0usize;
                    while j < 6 { *tx.add(22 + j) = unsafe { core::ptr::read_volatile((mmio as usize + j) as *const u8) }; j += 1; }
                    // SPA 0.0.0.0, THA zeros, TPA 0.0.0.0.
                }

                // TX descriptor 0: length 60, first+last fragment, then OWN.
                let tx_desc = tx_ring as *mut u32;
                unsafe {
                    core::ptr::write_volatile(tx_desc.add(0), 60 | 0x3000_0000);
                    core::ptr::write_volatile(tx_desc.add(1), 0);
                    core::ptr::write_volatile(tx_desc.add(2), tx_bufs[0] as u32);
                    core::ptr::write_volatile(tx_desc.add(3), (tx_bufs[0] >> 32) as u32);
                    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
                    core::ptr::write_volatile(tx_desc.add(0), 60 | 0xB000_0000);
                }

                // Program 64-bit ring bases and conservative receive filtering:
                // own MAC + broadcast only, no multicast/promiscuous acceptance.
                unsafe {
                    let tx_cfg = (core::ptr::read_volatile((mmio as usize + 0x40) as *const u32) & !0x0000_0700) | 0x0000_0700;
                    core::ptr::write_volatile((mmio as usize + 0x40) as *mut u32, tx_cfg);
                    core::ptr::write_volatile((mmio as usize + 0xE4) as *mut u32, rx_ring as u32);
                    core::ptr::write_volatile((mmio as usize + 0xE8) as *mut u32, (rx_ring >> 32) as u32);
                    core::ptr::write_volatile((mmio as usize + 0x20) as *mut u32, tx_ring as u32);
                    core::ptr::write_volatile((mmio as usize + 0x24) as *mut u32, (tx_ring >> 32) as u32);
                    core::ptr::write_volatile((mmio as usize + 0xDA) as *mut u16, 2048);
                    let rcr_old = core::ptr::read_volatile((mmio as usize + 0x44) as *const u32);
                    let rcr = (rcr_old & !0x3F) | 0x0000_070A;
                    core::ptr::write_volatile((mmio as usize + 0x44) as *mut u32, rcr);
                    core::ptr::write_volatile((mmio as usize + 0x3C) as *mut u16, 0);
                    core::ptr::write_volatile((mmio as usize + 0x3E) as *mut u16, 0xFFFF);
                    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
                    core::ptr::write_volatile((mmio as usize + 0x37) as *mut u8, 0x0C);
                    core::ptr::write_volatile((mmio as usize + 0x38) as *mut u8, 0x40);
                }

                write_str("DMA=ENABLED RX=ON TX=ON\n");
                write_str("RDSAR="); write_hex(unsafe { core::ptr::read_volatile((mmio as usize + 0xE4) as *const u32) as usize });
                write_str(" TNPDS="); write_hex(unsafe { core::ptr::read_volatile((mmio as usize + 0x20) as *const u32) as usize });
                write_str("\n");

                let mut tx_done = false;
                let mut ticks = 0usize;
                while ticks < 2_000_000usize {
                    let st = unsafe { core::ptr::read_volatile(tx_desc as *const u32) };
                    if st & 0x8000_0000 == 0 { tx_done = true; break; }
                    core::hint::spin_loop();
                    ticks += 1;
                }
                let tx_status = unsafe { core::ptr::read_volatile(tx_desc as *const u32) };
                let isr = unsafe { core::ptr::read_volatile((mmio as usize + 0x3E) as *const u16) };
                write_str("TX_DONE="); write_str(if tx_done { "PASS" } else { "TIMEOUT" });
                write_str(" TX_DESC="); write_hex(tx_status as usize);
                write_str(" ISR="); write_hex(isr as usize); write_str("\n");

                write_str("RX_DESC0="); write_hex(unsafe { core::ptr::read_volatile(rx_desc as *const u32) as usize });
                write_str(" RX_DESC1="); write_hex(unsafe { core::ptr::read_volatile(rx_desc.add(4) as *const u32) as usize });
                write_str(" CHIPCMD="); write_hex(unsafe { core::ptr::read_volatile((mmio as usize + 0x37) as *const u8) as usize });
                write_str(" PHYSTATUS="); write_hex(unsafe { core::ptr::read_volatile((mmio as usize + 0x6C) as *const u8) as usize });
                write_str("\n");
                write_str("ARP_TX=ISSUED POLL=2M RESET=YES DMA=YES IRQ=OFF\n");
                write_str("======== ETHDMA END ========\n");
                break 'outer;
            }
        }
    }
    if !found { write_str("RESULT=RTL8168_NOT_FOUND\n"); write_str("======== ETHDMA END ========\n"); }
}

fn cmd_ethlink() {
    write_str("======== ETHERNET LINK CONTROL ========\n");
    write_str("MODE=RTL8168 MDIO ONLY\n");
    write_str("XID_CHECK=MASK(0x7C8)==0x2C8\n");

    let mut found = false;
    'outer: for bus in 0u8..=31 {
        for dev in 0u8..32 {
            for func in 0u8..8 {
                let id = pci_cfg_read32(bus, dev, func, 0x00);
                if id == 0 || id == 0xFFFF_FFFF { continue; }
                let classreg = pci_cfg_read32(bus, dev, func, 0x08);
                if ((classreg >> 24) & 0xFF) != 0x02 || ((classreg >> 16) & 0xFF) != 0x00 { continue; }
                if (id & 0xFFFF) != 0x10EC || ((id >> 16) & 0xFFFF) != 0x8168 { continue; }

                let bar_lo = pci_cfg_read32(bus, dev, func, 0x18);
                let bar_hi = pci_cfg_read32(bus, dev, func, 0x1C);
                let mmio = ((bar_hi as u64) << 32) | ((bar_lo & 0xFFFF_FFF0) as u64);
                if mmio == 0 || (mmio & 0xFFF) != 0 {
                    write_str("RESULT=NO_VALID_MMIO_BAR2\n");
                    found = true;
                    break;
                }
                if !ethchip_map_page(mmio as usize) {
                    write_str("RESULT=MMIO_MAP_FAIL\n");
                    found = true;
                    break;
                }

                let txcfg = ethchip_read32(mmio as usize, 0x40);
                let xid = (txcfg >> 20) & 0xFCF;
                write_str("BDF="); write_usize(bus as usize); write_str(":");
                write_usize(dev as usize); write_str("."); write_usize(func as usize); write_str("\n");
                write_str("TXCONFIG="); write_hex(txcfg as usize);
                write_str(" XID="); write_hex(xid as usize); write_str("\n");

                if (xid & 0x7C8) != 0x2C8 {
                    write_str("XID_CHECK=FAIL\n");
                    write_str("ACTION=NONE RESET=NO DMA=NO\n");
                    break 'outer;
                }
                write_str("XID_CHECK=PASS VARIANT=RTL8168EVL/8111EVL\n");

                let bmcr = ethchip_mdio_read(mmio as usize, 0);
                let bmsr_before = ethchip_mdio_read(mmio as usize, 1);
                let anar = ethchip_mdio_read(mmio as usize, 4);
                let gctl = ethchip_mdio_read(mmio as usize, 9);
                let gstat = ethchip_mdio_read(mmio as usize, 10);

                write_str("BEFORE BMCR="); match bmcr { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" BMSR="); match bmsr_before { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" ANAR="); match anar { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" REG9="); match gctl { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" REG10="); match gstat { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str("\n");

                let restart = match bmcr {
                    Some(v) => ethchip_mdio_write(mmio as usize, 0, v | 0x0200),
                    None => false,
                };
                write_str("RESTART_AN="); write_str(if restart { "ISSUED" } else { "WRITE_FAIL" }); write_str("\n");

                let mut complete = false;
                let mut link = false;
                let mut ticks = 0usize;
                while ticks < 5_000_000usize {
                    if let Some(v) = ethchip_mdio_read(mmio as usize, 1) {
                        complete = (v & 0x0020) != 0;
                        link = (v & 0x0004) != 0;
                        if complete && link { break; }
                    }
                    ticks += 1;
                }

                let bmsr_final_1 = ethchip_mdio_read(mmio as usize, 1);
                let bmsr_final_2 = ethchip_mdio_read(mmio as usize, 1);
                let gstat_final = ethchip_mdio_read(mmio as usize, 10);

                write_str("AFTER BMSR1="); match bmsr_final_1 { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" BMSR2="); match bmsr_final_2 { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" REG10="); match gstat_final { Some(v) => write_hex(v as usize), None => write_str("TIMEOUT") }
                write_str(" ANEG="); write_str(if complete { "COMPLETE" } else { "INCOMPLETE" });
                write_str(" LINK="); write_str(if link { "UP" } else { "DOWN" }); write_str("\n");
                write_str("WAIT_TICKS="); write_usize(ticks); write_str("\n");
                write_str("DMA_START=NO RESET=NO\n");
                write_str("======== ETHLINK END ========\n");
                found = true;
                break 'outer;
            }
        }
    }
    if !found { write_str("RESULT=RTL8168_NOT_FOUND\n"); write_str("======== ETHLINK END ========\n"); }
}

fn cmd_ethdiag() {
    write_str("======== ETHERNET PCI DIAGNOSTIC ========\n");
    write_str("MODE=READ_ONLY PCI_CONFIG\n");
    let mut found = 0usize;
    for bus in 0u8..=31 { for dev in 0u8..32 { for func in 0u8..8 {
        let id = pci_cfg_read32(bus, dev, func, 0x00);
        if id == 0 || id == 0xFFFF_FFFF { continue; }
        let classreg = pci_cfg_read32(bus, dev, func, 0x08);
        if ((classreg >> 24) & 0xFF) != 0x02 || ((classreg >> 16) & 0xFF) != 0x00 { continue; }
        found += 1;
        let cmdstat = pci_cfg_read32(bus, dev, func, 0x04);
        let hdr = pci_cfg_read32(bus, dev, func, 0x0C);
        let subsys = pci_cfg_read32(bus, dev, func, 0x2C);
        write_str("NIC["); write_usize(found); write_str("] BDF="); write_usize(bus as usize); write_str(":"); write_usize(dev as usize); write_str("."); write_usize(func as usize);
        write_str(" VID:DID="); write_hex((id & 0xFFFF) as usize); write_str(":"); write_hex((id >> 16) as usize); write_str("\n");
        write_str("CLASS="); write_hex(((classreg >> 24)&0xFF) as usize); write_str(" SUBCLASS="); write_hex(((classreg >> 16)&0xFF) as usize);
        write_str(" REV="); write_hex((classreg&0xFF) as usize); write_str(" HEADER="); write_hex(((hdr>>16)&0xFF) as usize); write_str("\n");
        write_str("CMD="); write_hex((cmdstat&0xFFFF) as usize); write_str(" STATUS="); write_hex((cmdstat>>16) as usize); write_str(" IRQ="); write_hex(((hdr>>8)&0xFF) as usize); write_str("\n");
        write_str("SUBSYS=VENDOR "); write_hex((subsys&0xFFFF) as usize); write_str(" DEVICE "); write_hex((subsys>>16) as usize); write_str("\n");
        let mut off=0x10u8; let mut n=0usize;
        while n<6 {
            let bar = pci_cfg_read32(bus,dev,func,off);
            write_str("BAR"); write_usize(n); write_str("="); write_hex(bar as usize);
            if bar & 1 != 0 {
                write_str(" IO");
            } else if bar & 4 != 0 {
                write_str(" MEM64");
            } else {
                write_str(" MEM32");
            }
            write_str(if bar & 8 != 0 { " PREFETCH" } else { " NONPREFETCH" });
            write_str("\n");
            off+=4; n+=1;
        }
        ethdiag_capabilities(bus, dev, func, (cmdstat >> 16) as u16);
    }}}
    write_str("FOUND="); write_usize(found); write_str("\n");
    write_str("ACTION=NONE RESET=NO BAR_WRITE=NO MMIO=NO DMA=NO\n");
    write_str("======== ETHDIAG END ========\n");
}


fn cmd_net() {
    let mut mac = [0u8; 6];
    crate::drivers::net::mac(&mut mac);
    write_str("======== AETHER RTL8168 NETWORK ========\n");
    write_str("DRIVER=RTL8168EVL/8111EVL (r8169 family)\n");
    write_str("FOUND="); write_str(if crate::drivers::net::found() { "YES" } else { "NO" });
    write_str(" READY="); write_str(if crate::drivers::net::ready() { "YES" } else { "NO" });
    write_str(" LINK="); write_str(if crate::drivers::net::link_up() { "UP" } else { "DOWN" });
    write_str("\nMAC=");
    for i in 0..6 { write_hex(mac[i] as usize); if i != 5 { write_str(":"); } }
    write_str("\n");
    write_str("DATAPATH=MMIO+DMA_RING_POLLING\n");
    write_str("PROTOCOL=NOT_ENABLED_YET\n");
    write_str("PING=NEXT_STAGE\n");
    write_str("=========================================\n");
}

fn cmd_help() {
    write_str("Aether Terminal - native command interface\n");
    write_str("Core: HELP  CLS  VER  LOG  TANSI  SEARCH <text>\n");
    write_str("Hardware: KMS5  VINFO  V800  V1366  AUD  AUD2  AUD3  MOUS  USB  WF  ETHDIAG  ETHCHIP  ETHLINK  ETHDMA  NET\n");
    write_str("Tip: Up/Down recalls command history; arrow keys scroll long output.\n");
}

fn cmd_ver() {
    write_str("Aether OS terminal 1.0\n");
    write_str("Native x86_64 shell; GUI terminal backend active\n");
}

fn cmd_tansi() {
    write_str("ANSI SGR test: ");
    write_str("\x1b[31mRED \x1b[32mGREEN \x1b[34mBLUE \x1b[93mBRIGHT-YELLOW \x1b[0mDEFAULT\n");
    write_str("\x1b[35mMAGENTA\x1b[0m / \x1b[36mCYAN\x1b[0m / \x1b[97mWHITE\x1b[0m\n");
    write_str("Parser: SGR 0, 1, 30-37, 90-97; CSI 2J/K\n");
}

fn run_line(line: &[u8], len: usize) {
    let mut s = 0usize;
    while s < len && line[s] == b' ' { s += 1; }
    let mut e = len;
    while e > s && (line[e - 1] == b' ' || line[e - 1] == b'\r') { e -= 1; }
    let clen = e.saturating_sub(s);

    if eq(line, s, clen, b"NET") || eq(line, s, clen, b"net") {
        cmd_net();
    } else if eq(line, s, clen, b"ETHCHIP") || eq(line, s, clen, b"ethchip") {
        cmd_ethchip();
    } else if eq(line, s, clen, b"ETHDIAG") || eq(line, s, clen, b"ethdiag") {
        cmd_ethdiag(); 
    } else if eq(line, s, clen, b"ETHLINK") || eq(line, s, clen, b"ethlink") {
        cmd_ethlink();
    } else if eq(line, s, clen, b"ETHDMA") || eq(line, s, clen, b"ethdma") {
        cmd_ethdma();
    } else if eq(line, s, clen, b"HELP") || eq(line, s, clen, b"help") {
        cmd_help();
    } else if eq(line, s, clen, b"CLS") || eq(line, s, clen, b"cls") {
        crate::desktop::terminal_clear();
    } else if eq(line, s, clen, b"VER") || eq(line, s, clen, b"ver") {
        cmd_ver();
    } else if eq(line, s, clen, b"LOG") || eq(line, s, clen, b"log") {
        cmd_log();
    } else if eq(line, s, clen, b"TANSI") || eq(line, s, clen, b"tansi") {
        cmd_tansi();
    } else if clen >= 7 && (line[s]==b'S'||line[s]==b's')&&(line[s+1]==b'E'||line[s+1]==b'e')&&(line[s+2]==b'A'||line[s+2]==b'a')&&(line[s+3]==b'R'||line[s+3]==b'r')&&(line[s+4]==b'C'||line[s+4]==b'c')&&(line[s+5]==b'H'||line[s+5]==b'h')&&line[s+6]==b' ' {
        let mut q=s+7;while q<e&&line[q]==b' '{q+=1;}terminal_search(&line[q..e]);
    } else if eq(line, s, clen, b"KMS5") || eq(line, s, clen, b"kms5") {
        cmd_kms5();
    } else if eq(line, s, clen, b"VINFO") || eq(line, s, clen, b"vinfo") {
        cmd_video_info();
    } else if eq(line, s, clen, b"V1366") || eq(line, s, clen, b"v1366") {
        cmd_video_mode(1366, 768);
    } else if eq(line, s, clen, b"V800") || eq(line, s, clen, b"v800") {
        cmd_video_mode(800, 600);
    } else if eq(line, s, clen, b"WINAMP") || eq(line, s, clen, b"winamp") {
        write_str("======== WINAMP LAUNCH GATE ========\\n");
        let ok = crate::win32_runtime::launch_winamp("/winamp.exe");
        write_str("RESULT=");
        write_str(if ok { "EXECUTION-STARTED" } else { "EXECUTION-BLOCKED" });
        write_str("\\n======== WINAMP END ========\\n");
    } else if eq(line, s, clen, b"WF") || eq(line, s, clen, b"wf") {
        cmd_wf();
    } else if eq(line, s, clen, b"MOUS") || eq(line, s, clen, b"mous") {
        cmd_mous();
    } else if eq(line, s, clen, b"USB") || eq(line, s, clen, b"usb") {
        cmd_usbtop();
    } else if eq(line, s, clen, b"AUD") || eq(line, s, clen, b"aud") {
        cmd_aud();
    } else if eq(line, s, clen, b"AUD3") || eq(line, s, clen, b"aud3") {
        cmd_aud3();
    } else if eq(line, s, clen, b"AUD2") || eq(line, s, clen, b"aud2") {
        write_str("======== AUD PCM PLAYBACK ========\n");
        let opened = crate::media_player::open_embedded_wav();
        write_str("SOURCE=EMBEDDED_TEST_WAV\n");
        write_str("OPEN=");
        write_str(if opened { "YES" } else { "NO" });
        write_str(" SIZE="); write_usize(crate::media_player::file_size());
        write_str(" RATE="); write_usize(crate::media_player::sample_rate() as usize);
        write_str(" CH="); write_usize(crate::media_player::channels() as usize);
        write_str(" BITS="); write_usize(crate::media_player::bits() as usize);
        write_str(" PCM="); write_usize(crate::media_player::data_bytes());
        write_str("\n");
        if opened {
            crate::media_player::play();
            write_str("PLAY_REQUEST=YES\n");
            write_str("STREAM_RUNNING=");
            write_str(if crate::drivers::audio::hda_stream_running() { "YES" } else { "NO" });
            write_str("\nLPIB0="); write_hex(crate::drivers::audio::hda_lpib() as usize);
            // Sample the hardware position repeatedly. If LPIB advances, the
            // HDA stream engine is consuming the BDL/DMA buffer; only then
            // should codec pin/amp routing be investigated.
            unsafe {
                let mut d = 0u32;
                while d < 500_000 { core::arch::asm!("pause", options(nostack, preserves_flags)); d += 1; }
            }
            crate::drivers::audio::playback_poll();
            write_str(" LPIB1="); write_hex(crate::drivers::audio::hda_lpib() as usize);
            unsafe {
                let mut d = 0u32;
                while d < 500_000 { core::arch::asm!("pause", options(nostack, preserves_flags)); d += 1; }
            }
            crate::drivers::audio::playback_poll();
            write_str(" LPIB2="); write_hex(crate::drivers::audio::hda_lpib() as usize);
            write_str(" CTL="); write_hex(crate::drivers::audio::hda_stream_control() as usize);
            write_str(" STAT="); write_hex(crate::drivers::audio::hda_stream_status() as usize);
            write_str(" CBL="); write_hex(crate::drivers::audio::hda_stream_cbl() as usize);
            write_str(" LVI="); write_hex(crate::drivers::audio::hda_stream_lvi() as usize); write_str("\n");
            cmd_aud_readback();
        } else {
            write_str("PLAY_REQUEST=NO_EMBEDDED_WAV\n");
        }
        write_str("======== AUD PCM END ========\n");
    } else {
        write_str("Unknown command. Type HELP for commands.\n");
    }
}

pub fn terminal_search(query:&[u8]) { crate::desktop::term_search_set(query,query.len()); }

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
    write_str("Aether Terminal 1.0 — native shell\n");
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
