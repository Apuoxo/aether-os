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
fn aud_probe_one(codec: u8, node: u8, param: u16) -> crate::drivers::audio::HdaVerbDiag {
    // GET_PARAMETER is a 12-bit verb ID (F00h) plus an 8-bit payload.
    // The complete 20-bit verb field is therefore F00 | parameter.
    let verb20 = 0x000F_00u32 | (param as u32 & 0xFF);
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
fn run_line(line: &[u8], len: usize) {
    let mut s = 0usize;
    while s < len && line[s] == b' ' { s += 1; }
    let mut e = len;
    while e > s && (line[e - 1] == b' ' || line[e - 1] == b'\r') { e -= 1; }
    let clen = e.saturating_sub(s);
    if eq(line, s, clen, b"WF") || eq(line, s, clen, b"wf") {
        cmd_wf();
    } else if eq(line, s, clen, b"AUD") || eq(line, s, clen, b"aud") {
        cmd_aud();
    } else if eq(line, s, clen, b"AUDPLAY") || eq(line, s, clen, b"audplay") {
        write_str("======== AUD PCM PLAYBACK ========\\n");
        let ok = crate::media_player::state() != crate::media_player::State::Empty
            && crate::media_player::state() != crate::media_player::State::Error;
        if ok {
            crate::media_player::play();
            write_str("PLAY_REQUEST=YES\\n");
            write_str("STREAM_RUNNING=");
            write_str(if crate::drivers::audio::hda_stream_running() { "YES" } else { "NO" });
            write_str("\\nLPIB="); write_hex(crate::drivers::audio::hda_lpib() as usize);
            write_str(" CTL="); write_hex(crate::drivers::audio::hda_stream_control() as usize);
            write_str(" STAT="); write_hex(crate::drivers::audio::hda_stream_status() as usize); write_str("\\n");
        } else {
            write_str("PLAY_REQUEST=NO_MEDIA_SELECTED\\n");
        }
        write_str("======== AUD PCM END ========\\n");
    } else {
        write_str("unknown — commands: AUD, WF\n");
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
