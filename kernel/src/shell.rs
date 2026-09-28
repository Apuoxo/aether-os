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

fn cmd_help() {
    write_str("Aether Terminal - native command interface\n");
    write_str("Core: HELP  CLS  VER  TANSI  SEARCH <text>\n");
    write_str("Hardware: KMS5  VINFO  V800  V1366  AUD  AUD2  AUD3  MOUS  USB  WF\n");
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
    if eq(line, s, clen, b"HELP") || eq(line, s, clen, b"help") {
        cmd_help();
    } else if eq(line, s, clen, b"CLS") || eq(line, s, clen, b"cls") {
        crate::desktop::terminal_clear();
    } else if eq(line, s, clen, b"VER") || eq(line, s, clen, b"ver") {
        cmd_ver();
    } else if eq(line, s, clen, b"TANSI") || eq(line, s, clen, b"tansi") {
        cmd_tansi();
    } else if clen >= 7 && (line[s]==b'S'||line[s]==b's')&&(line[s+1]==b'E'||line[s+1]==b'e')&&(line[s+2]==b'A'||line[s+2]==b'a')&&(line[s+3]==b'R'||line[s+3]==b'r')&&(line[s+4]==b'C'||line[s+4]==b'c')&&(line[s+5]==b'H'||line[s+5]==b'h')&&line[s+6]==b' ' {
        let mut q=s+7;while q<e&&line[q]==b' '{q+=1;}crate::desktop::terminal_search(&line[q..e]);
    } else if eq(line, s, clen, b"KMS5") || eq(line, s, clen, b"kms5") {
        cmd_kms5();
    } else if eq(line, s, clen, b"VINFO") || eq(line, s, clen, b"vinfo") {
        cmd_video_info();
    } else if eq(line, s, clen, b"V1366") || eq(line, s, clen, b"v1366") {
        cmd_video_mode(1366, 768);
    } else if eq(line, s, clen, b"V800") || eq(line, s, clen, b"v800") {
        cmd_video_mode(800, 600);
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

pub fn terminal_search(query:&[u8]) { term_search_set(query,query.len()); }

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
