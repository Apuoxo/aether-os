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
    vga_write_str(s);
    serial::write_str(s);
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

fn cmd_help() {
    write_str("Commands: help ls cat dsk 77 uname vdiag vedid echo halt\n");
}

fn cmd_77() {
    write_str("===== 77-BEGIN =====\n");
    write_str("77-TEST-01\n");
    write_str("77-TEST-02\n");
    write_str("77-TEST-03\n");
    write_str("77-TEST-04\n");
    write_str("77-TEST-05\n");
    write_str("77-TEST-06\n");
    write_str("77-TEST-07\n");
    write_str("77-TEST-08\n");
    write_str("77-TEST-09\n");
    write_str("77-TEST-10\n");
    write_str("77-TEST-11\n");
    write_str("77-TEST-12\n");
    write_str("77-TEST-13\n");
    write_str("77-TEST-14\n");
    write_str("77-TEST-15\n");
    write_str("77-TEST-16\n");
    write_str("77-TEST-17\n");
    write_str("77-TEST-18\n");
    write_str("77-TEST-19\n");
    write_str("77-TEST-20\n");
    write_str("===== 77-END =====\n");
}

fn write_u64_decimal(mut n: u64) {
    let mut digs = [0u8; 20];
    let mut len = 0usize;
    if n == 0 { putc(b'0'); return; }
    while n > 0 && len < digs.len() {
        digs[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        putc(digs[len]);
    }
}


fn print_hex16_fresh(buf: &[u8; 512]) {
    let mut i = 0usize;
    while i < 16 {
        serial::write_hex(buf[i] as usize);
        if i != 15 { write_str(" "); }
        i += 1;
    }
}

fn cmd_dsk() {
    write_str("DIAG... DSK RAW-ONLY DISK DIAGNOSTIC\n");
    write_str("SOURCE: AHCI IDENTIFY + RAW LBA (NO part.rs / NO block.rs / NO VFS)\n");
    write_str("READ-ONLY: partition table and filesystem metadata are parsed from fresh sectors\n");

    let nd = crate::drivers::ahci::disk_count();
    write_str("PHYSICAL DISKS=");
    serial::write_usize(nd);
    write_str("\n");

    let mut d = 0usize;
    while d < nd {
        write_str("DISK ");
        serial::write_usize(d);
        write_str(" MODEL=");
        let mut model = [0u8; 40];
        let ml = crate::drivers::ahci::disk_model(d, &mut model);
        let mut i = 0usize;
        while i < ml && i < 40 {
            putc(model[i]);
            i += 1;
        }
        write_str(" SECTORS=");
        serial::write_usize(crate::drivers::ahci::disk_sectors(d) as usize);
        write_str("\n");

        let mut lba0 = [0u8; 512];
        let ok0 = crate::drivers::ahci::read_sectors(d, 0, 1, &mut lba0);
        write_str("  RAW_LBA0=");
        write_str(if ok0 { "OK" } else { "FAIL" });
        if !ok0 {
            write_str("\n");
            d += 1;
            continue;
        }
        write_str(" SIG=");
        write_str(if lba0[510] == 0x55 && lba0[511] == 0xAA { "55AA" } else { "NO-55AA" });
        write_str("\n");

        // Parse the partition table directly from the freshly-read LBA0.
        let mut protective_gpt = false;
        let mut pe = 0usize;
        while pe < 4 {
            let off = 446 + pe * 16;
            if lba0[off + 4] == 0xEE {
                protective_gpt = true;
            }
            pe += 1;
        }

        if protective_gpt {
            write_str("  TABLE=GPT (detected from protective MBR)\n");
            let mut gh = [0u8; 512];
            if !crate::drivers::ahci::read_sectors(d, 1, 1, &mut gh) ||
               gh[0] != b'E' || gh[1] != b'F' || gh[2] != b'I' || gh[3] != b' ' ||
               gh[4] != b'P' || gh[5] != b'A' || gh[6] != b'R' || gh[7] != b'T' {
                write_str("  GPT_HEADER=FAIL\n");
                d += 1;
                continue;
            }

            let entry_lba = u64::from_le_bytes([
                gh[72], gh[73], gh[74], gh[75], gh[76], gh[77], gh[78], gh[79]
            ]);
            let entry_count = u32::from_le_bytes([gh[80], gh[81], gh[82], gh[83]]);
            let entry_size = u32::from_le_bytes([gh[84], gh[85], gh[86], gh[87]]);
            write_str("  GPT_HEADER=OK ENTRY_LBA=");
            serial::write_usize(entry_lba as usize);
            write_str(" COUNT=");
            serial::write_usize(entry_count as usize);
            write_str(" SIZE=");
            serial::write_usize(entry_size as usize);
            write_str("\n");

            let es = if entry_size == 128 { 128usize } else { 0usize };
            let max_entries = if entry_count > 128 { 128usize } else { entry_count as usize };
            if es == 0 {
                write_str("  GPT_ENTRIES=UNSUPPORTED_ENTRY_SIZE\n");
                d += 1;
                continue;
            }

            let mut idx = 0usize;
            let mut found = 0usize;
            while idx < max_entries {
                let byte_off = idx * es;
                let sector = entry_lba + (byte_off / 512) as u64;
                let within = byte_off % 512;
                let mut eb = [0u8; 512];
                if within + 128 > 512 {
                    write_str("  GPT_ENTRY_READ=SKIP_CROSS_SECTOR\n");
                    break;
                }
                if !crate::drivers::ahci::read_sectors(d, sector, 1, &mut eb) {
                    write_str("  GPT_ENTRY_READ=FAIL IDX=");
                    serial::write_usize(idx);
                    write_str("\n");
                    idx += 1;
                    continue;
                }

                let mut nonzero = false;
                let mut j = 0usize;
                while j < 16 {
                    if eb[within + j] != 0 {
                        nonzero = true;
                        break;
                    }
                    j += 1;
                }
                if nonzero {
                    let first = u64::from_le_bytes([
                        eb[within + 32], eb[within + 33], eb[within + 34], eb[within + 35],
                        eb[within + 36], eb[within + 37], eb[within + 38], eb[within + 39]
                    ]);
                    let last = u64::from_le_bytes([
                        eb[within + 40], eb[within + 41], eb[within + 42], eb[within + 43],
                        eb[within + 44], eb[within + 45], eb[within + 46], eb[within + 47]
                    ]);
                    if last >= first {
                        write_str("  PART ");
                        serial::write_usize(found);
                        write_str(" GPT_IDX=");
                        serial::write_usize(idx);
                        write_str(" START_LBA=");
                        serial::write_usize(first as usize);
                        write_str(" SECTORS=");
                        serial::write_usize((last - first + 1) as usize);
                        write_str("\n");
                        shell_dsk_probe_partition(d, first);
                        found += 1;
                    }
                }
                idx += 1;
            }
            write_str("  GPT_PARTITIONS=");
            serial::write_usize(found);
            write_str("\n");
        } else {
            write_str("  TABLE=MBR (parsed directly from LBA0)\n");
            let mut found = 0usize;
            let mut pe2 = 0usize;
            while pe2 < 4 {
                let off = 446 + pe2 * 16;
                let ptype = lba0[off + 4];
                if ptype != 0 {
                    let start_lba = u32::from_le_bytes([
                        lba0[off + 8], lba0[off + 9], lba0[off + 10], lba0[off + 11]
                    ]) as u64;
                    let sectors = u32::from_le_bytes([
                        lba0[off + 12], lba0[off + 13], lba0[off + 14], lba0[off + 15]
                    ]) as u64;
                    write_str("  PART ");
                    serial::write_usize(found);
                    write_str(" MBR_IDX=");
                    serial::write_usize(pe2);
                    write_str("\n");
                    write_str("    TYPE=0x");
                    serial::write_hex(ptype as usize);
                    write_str("\n");
                    write_str("    START_LBA=");
                    serial::write_usize(start_lba as usize);
                    write_str("\n");
                    write_str("    SECTORS=");
                    serial::write_usize(sectors as usize);
                    write_str("\n");
                    shell_dsk_probe_partition(d, start_lba);
                    found += 1;
                }
                pe2 += 1;
            }
            write_str("  MBR_PARTITIONS=");
            serial::write_usize(found);
            write_str("\n");
        }

        d += 1;
    }

    write_str("======== DSK RAW-ONLY END ========\n");
}

fn shell_dsk_probe_partition(disk: usize, start: u64) {
    let mut boot = [0u8; 512];
    let rb = crate::drivers::ahci::read_sectors(disk, start, 1, &mut boot);
    write_str("    RAW_BOOT=");
    write_str(if rb { "OK" } else { "FAIL" });
    if !rb {
        write_str("\n");
        return;
    }

    write_str(" SIG=");
    write_str(if boot[510] == 0x55 && boot[511] == 0xAA { "55AA" } else { "NO-55AA" });

    if boot[3] == b'N' && boot[4] == b'T' && boot[5] == b'F' &&
       boot[6] == b'S' && boot[7] == b' ' {
        let bps = u16::from_le_bytes([boot[11], boot[12]]);
        let spc = boot[13];
        let mft_lcn = u64::from_le_bytes([
            boot[48], boot[49], boot[50], boot[51],
            boot[52], boot[53], boot[54], boot[55]
        ]);
        write_str(" FS=NTFS BPS=");
        serial::write_usize(bps as usize);
        write_str(" SPC=");
        serial::write_usize(spc as usize);
        write_str(" MFT_LCN=");
        serial::write_usize(mft_lcn as usize);
        if bps == 512 && spc != 0 {
            let mft_lba = start.saturating_add(mft_lcn.saturating_mul(spc as u64));
            write_str(" MFT_LBA=");
            serial::write_usize(mft_lba as usize);
            let mut mft = [0u8; 512];
            let rm = crate::drivers::ahci::read_sectors(disk, mft_lba, 1, &mut mft);
            write_str(" READ=");
            write_str(if rm { "OK" } else { "FAIL" });
            if rm {
                write_str(" SIG=");
                write_str(if mft[0] == b'F' && mft[1] == b'I' &&
                          mft[2] == b'L' && mft[3] == b'E' { "FILE" } else { "NOT-FILE" });
            }
        }
    } else if (boot[54] == b'F' && boot[55] == b'A' && boot[56] == b'T') ||
              (boot[82] == b'F' && boot[83] == b'A' && boot[84] == b'T') {
        write_str(" FS=FAT");
    } else {
        write_str(" FS=UNKNOWN");
    }
    write_str("\n");
}

fn cmd_vdiag() {
    write_str("======== Video diagnostic ========\n");
    write_str("software FB: ");
    write_str(if crate::drivers::video::ready() { "ready\n" } else { "not-ready\n" });
    write_str("Intel Gen6 MMIO: ");
    write_str(if crate::drivers::video::hardware_ready() { "ready\n" } else { "not-ready\n" });
    write_str("existing scanout: ");
    write_str(if crate::drivers::video::scanout_ready() { "attached\n" } else { "not-attached\n" });
    if crate::drivers::video::scanout_ready() {
        write_str("pipe: ");
        serial::write_usize(crate::drivers::video::scanout_pipe() as usize);
        write_str("\n");
        write_str("surface: ");
        serial::write_hex(crate::drivers::video::scanout_surface() as usize);
        write_str("\n");
    }
    crate::drivers::video::snapshot();
}

fn cmd_vedid() {
    write_str("======== Sandy Bridge EDID probe ========\\n");
    if !crate::drivers::video::hardware_ready() {
        write_str("EDID unavailable: Intel Gen6 MMIO not ready\\n");
        return;
    }
    let mut block = [0u8; 128];
    if crate::drivers::video::probe_edid(&mut block) == 0 {
        write_str("EDID: no validated block found\\n");
        return;
    }
    if !crate::drivers::video::parse_edid(&block) {
        write_str("EDID: block read but validation/parser rejected it\\n");
        return;
    }
    serial::write_str("[VIDEO/EDID] block0:");
    let mut i = 0usize;
    while i < 128 {
        if i % 16 == 0 { serial::write_str(if i == 0 { " " } else { "\\n[VIDEO/EDID] " }); }
        serial::write_hex(block[i] as usize);
        i += 1;
    }
    serial::write_str("\\n");
    if let Some(m) = crate::drivers::video::preferred_mode() {
        write_str("preferred mode: ");
        serial::write_usize(m.width as usize);
        write_str("x");
        serial::write_usize(m.height as usize);
        write_str(" clock_khz=");
        serial::write_usize(m.pixel_clock_khz as usize);
        write_str(" htotal=");
        serial::write_usize(m.h_total as usize);
        write_str(" vtotal=");
        serial::write_usize(m.v_total as usize);
        write_str("\\n");
    }
}

fn cmd_ls() {
    if !fs::is_mounted() {
        write_str("(no filesystem)\n");
        return;
    }
    let mut names = [[0u8; 24]; 16];
    let mut lens = [0usize; 16];
    let n = fs::list(&mut names, &mut lens);
    write_str("files: ");
    // write number via serial helper + vga
    serial::write_usize(n);
    // also on VGA - simple digit
    if n == 0 {
        vga_write(b"0");
    } else {
        let mut tmp = n;
        let mut digs = [0u8; 8];
        let mut nd = 0usize;
        while tmp > 0 && nd < 8 {
            digs[nd] = b'0' + (tmp % 10) as u8;
            tmp /= 10;
            nd += 1;
        }
        while nd > 0 {
            nd -= 1;
            vga_putc(digs[nd]);
        }
    }
    write_str("\n");
    let mut i = 0usize;
    while i < n {
        write_str("  ");
        let mut j = 0usize;
        while j < lens[i] && j < 24 {
            putc(names[i][j]);
            j += 1;
        }
        write_str("\n");
        i += 1;
    }
}

fn cmd_cat_test() {
    let mut buf = [0u8; 64];
    match fs::read("/test.txt", &mut buf) {
        Some(n) => {
            let mut i = 0usize;
            while i < n {
                putc(buf[i]);
                i += 1;
            }
            write_str("\n");
        }
        None => write_str("not found (or no disk)\n"),
    }
}

fn cmd_mem() {
    if crate::drivers::ata::is_ramdisk() {
        write_str("RAMDISK OK  sectors=");
        serial::write_usize(crate::drivers::ata::total_sectors() as usize);
        write_str("\n");
    }
    write_str("FS mounted: ");
    write_str(if fs::is_mounted() { "yes" } else { "no" });
    write_str("\n");
    write_str("free pages: ");
    serial::write_usize(mm::free_count());
    // rough VGA copy of count
    let n = mm::free_count();
    let mut tmp = n;
    let mut digs = [0u8; 12];
    let mut nd = 0usize;
    if tmp == 0 {
        vga_putc(b'0');
    } else {
        while tmp > 0 && nd < 12 {
            digs[nd] = b'0' + (tmp % 10) as u8;
            tmp /= 10;
            nd += 1;
        }
        while nd > 0 {
            nd -= 1;
            vga_putc(digs[nd]);
        }
    }
    write_str("\n");
}

fn cmd_uname() {
    write_str("Aether OS v3.1 polymorphic kernel\n");
    write_str("arch: x86_64  mode: text shell\n");
    if crate::drivers::ata::is_ramdisk() {
        write_str("storage: RAMDISK 32KiB (volatile)\n");
    } else {
        write_str("storage: ATA\n");
    }
}

fn run_line(line: &[u8], len: usize) {
    let mut s = 0usize;
    let mut e = len;
    while s < e && line[s] == b' ' {
        s += 1;
    }
    while e > s && (line[e - 1] == b' ' || line[e - 1] == b'\r') {
        e -= 1;
    }
    if s >= e {
        return;
    }
    let mut sp = s;
    while sp < e && line[sp] != b' ' {
        sp += 1;
    }
    let clen = sp - s;
    let mut a = sp;
    while a < e && line[a] == b' ' {
        a += 1;
    }
    if eq(line, s, clen, b"help") || eq(line, s, clen, b"?") {
        cmd_help();
    } else if eq(line, s, clen, b"ls") {
        cmd_ls();
    } else if eq(line, s, clen, b"cat") {
        cmd_cat_test();
    } else if eq(line, s, clen, b"mem") {
        cmd_mem();
    } else if eq(line, s, clen, b"dsk") {
        cmd_dsk();
    } else if eq(line, s, clen, b"77") {
        cmd_77();
    } else if eq(line, s, clen, b"vdiag") {
        cmd_vdiag();
    } else if eq(line, s, clen, b"vedid") {
        cmd_vedid();
    } else if eq(line, s, clen, b"uname") {
        cmd_uname();
    } else if eq(line, s, clen, b"echo") {
        let mut i = a;
        while i < e {
            putc(line[i]);
            i += 1;
        }
        write_str("\n");
    } else if eq(line, s, clen, b"halt") || eq(line, s, clen, b"exit") {
        write_str("halt\n");
        loop {
            unsafe {
                core::arch::asm!("hlt");
            }
        }
    } else {
        write_str("unknown — try help\n");
    }
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
    write_str("SHELL BUILD MARKER = 4099CMD77\n");
    cmd_help();
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
