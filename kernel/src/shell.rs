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
    write_str("Commands: help ls cat mem uname vdiag vedid echo halt\n");
}

fn cmd_dsk() {
    write_str("======== DSK: real storage probe (READ-ONLY) ========\n");

    let snap = crate::storage_hw_diag::snapshot();
    write_str("PCI mass-storage controllers: ");
    serial::write_usize(snap.mass_n);
    write_str("  AHCI=");
    serial::write_usize(snap.ahci_n);
    write_str(" IDE=");
    serial::write_usize(snap.ide_n);
    write_str(" NVMe=");
    serial::write_usize(snap.nvme_n);
    write_str("\n");

    if snap.mass_n > 0 {
        let mut i = 0usize;
        while i < snap.mass_n {
            let d = snap.mass[i];
            write_str("  PCI ");
            serial::write_usize(d.bus as usize);
            write_str(":");
            serial::write_usize(d.dev as usize);
            write_str(".");
            serial::write_usize(d.func as usize);
            write_str(" ");
            serial::write_hex(d.vid as usize);
            write_str(":");
            serial::write_hex(d.did as usize);
            write_str(" AHCI=");
            write_str(if d.is_ahci { "Y" } else { "N" });
            write_str(" ABAR=");
            serial::write_hex(d.abar as usize);
            write_str(" PI=");
            serial::write_hex(d.pi as usize);
            write_str(" present=");
            serial::write_hex(d.ports_present as usize);
            write_str("\n");
            i += 1;
        }
    }

    let nd = crate::drivers::ahci::disk_count();
    write_str("AHCI runtime disks: ");
    serial::write_usize(nd);
    write_str("\n");

    let mut di = 0usize;
    while di < nd {
        let mut model = [0u8; 40];
        let ml = crate::drivers::ahci::disk_model(di, &mut model);
        write_str("  disk");
        serial::write_usize(di);
        write_str(" model=");
        let mut j = 0usize;
        while j < ml {
            putc(model[j]);
            j += 1;
        }
        write_str(" sectors=");
        serial::write_usize(crate::drivers::ahci::disk_sectors(di) as usize);
        write_str("\n");

        let mut sec0 = [0u8; 512];
        let ok = crate::drivers::ahci::read_sectors(di, 0, 1, &mut sec0);
        write_str("    LBA0 READ=");
        write_str(if ok { "OK" } else { "FAIL" });
        if ok {
            write_str(" sig=");
            if sec0[510] == 0x55 && sec0[511] == 0xAA {
                write_str("MBR");
            } else {
                write_str("no-55AA");
            }
        }
        write_str("\n");
        di += 1;
    }

    let nb = crate::block::count();
    write_str("Block devices: ");
    serial::write_usize(nb);
    write_str("\n");
    let mut bi = 0usize;
    while bi < nb {
        if let Some(d) = crate::block::get(bi) {
            write_str("  block");
            serial::write_usize(bi);
            write_str(" hw=");
            write_str(if d.is_hw { "Y" } else { "N" });
            write_str(" ahci=");
            write_str(if d.is_ahci { "Y" } else { "N" });
            write_str(" sectors32=");
            serial::write_usize(d.sectors as usize);
            write_str("\n");
        }
        bi += 1;
    }

    let np = crate::part::count();
    write_str("Partitions parsed from READ data: ");
    serial::write_usize(np);
    write_str("\n");
    let mut pi = 0usize;
    while pi < np {
        if let Some(p) = crate::part::get(pi) {
            write_str("  #");
            serial::write_usize(pi);
            write_str(" disk=");
            serial::write_usize(p.disk as usize);
            write_str(" idx=");
            serial::write_usize(p.index as usize);
            write_str(" type=");
            write_str(crate::part::type_name(p.ptype));
            write_str(" LBA=");
            serial::write_usize(p.lba_start as usize);
            write_str(" sectors=");
            serial::write_usize(p.sectors as usize);

            let mut sec = [0u8; 512];
            let ok = crate::block::read(p.disk, p.lba_start, 1, &mut sec);
            write_str(" read=");
            write_str(if ok { "OK" } else { "FAIL" });
            if ok {
                write_str(" fs=");
                if sec[3] == b'N' && sec[4] == b'T' && sec[5] == b'F' && sec[6] == b'S' {
                    write_str("NTFS");
                } else if sec[54] == b'F' && sec[55] == b'A' && sec[56] == b'T' {
                    write_str("FAT");
                } else {
                    write_str("unknown");
                }
            }
            write_str("\n");
        }
        pi += 1;
    }

    write_str("NTFS mounted: ");
    write_str(if crate::fs_ntfs::is_mounted() { "YES" } else { "NO" });
    write_str("\n");
    if crate::fs_ntfs::is_mounted() {
        write_str("NTFS root entries: ");
        serial::write_usize(crate::fs_ntfs::entry_count());
        write_str("\n");
    }
    write_str("======== END DSK ========\n");
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
