//! Safe framebuffer via Multiboot2 tag — no hang on failure

use crate::serial;
use crate::mm;
use crate::mm::paging;

const MB2_TAG_FB: u32 = 8;
const MB2_TAG_END: u32 = 0;

static mut FB_ADDR: usize = 0;
static mut FB_W: usize = 0;
static mut FB_H: usize = 0;
static mut FB_PITCH: usize = 0;
static mut FB_BPP: u8 = 0;
static mut FB_OK: bool = false;

pub fn is_ready() -> bool {
    unsafe { FB_OK }
}

pub fn width() -> usize {
    unsafe { FB_W }
}
pub fn height() -> usize {
    unsafe { FB_H }
}

fn vga_fb_mark(col: usize, a: u8, b: u8) {
    // write two chars at row 1
    unsafe {
        let p = (0xB8000 + 80 * 2 + col * 2) as *mut u16;
        *p = 0x0E00u16 | (a as u16);
        let p2 = (0xB8000 + 80 * 2 + (col + 1) * 2) as *mut u16;
        *p2 = 0x0E00u16 | (b as u16);
    }
}

fn read_u32(p: usize) -> u32 {
    unsafe { core::ptr::read_unaligned(p as *const u32) }
}
fn read_u64(p: usize) -> u64 {
    unsafe { core::ptr::read_unaligned(p as *const u64) }
}
fn read_u8(p: usize) -> u8 {
    unsafe { *(p as *const u8) }
}

/// Map physical FB range into current page tables (identity, USER not needed)
fn map_fb_range(phys: usize, size: usize) -> bool {
    if phys == 0 || size == 0 {
        return false;
    }
    let cr3 = unsafe { paging::read_cr3() };
    let start = phys & !0xFFF;
    let end = (phys + size + 0xFFF) & !0xFFF;
    let mut va = start;
    while va < end {
        // already identity-mapped below 1GiB?
        if va < 0x4000_0000 {
            va += 0x1000;
            continue;
        }
        // map 4K page at identity
        if !unsafe {
            paging::map_page(
                cr3,
                va,
                va,
                paging::PAGE_PRESENT | paging::PAGE_WRITE,
            )
        } {
            // may already be mapped — try continue
        }
        va += 0x1000;
    }
    unsafe { paging::load_cr3(cr3) };
    true
}


/// Parse Multiboot2 info; return true if usable 32bpp RGB FB found
pub fn init_from_mbi(mbi: usize) -> bool {
    serial::write_str("\n[FB] Multiboot2 scan...\n");
    if mbi == 0 || mbi < 0x1000 {
        serial::write_str("[FBX] no MBI pointer\n");
        vga_fb_mark(0, b'F', b'X');
        return false;
    }

    // total_size at mbi+0
    let total = read_u32(mbi) as usize;
    if total < 16 || total > 0x100000 {
        serial::write_str("[FBX] bad MBI size\n");
        vga_fb_mark(0, b'F', b'X');
        return false;
    }

    let mut off = 8usize;
    while off + 8 <= total {
        let tag_type = read_u32(mbi + off);
        let tag_size = read_u32(mbi + off + 4) as usize;
        if tag_size < 8 || off + tag_size > total {
            break;
        }
        if tag_type == MB2_TAG_END {
            break;
        }
        if tag_type == MB2_TAG_FB && tag_size >= 28 {
            let addr = read_u64(mbi + off + 8) as usize;
            let pitch = read_u32(mbi + off + 16) as usize;
            let width = read_u32(mbi + off + 20) as usize;
            let height = read_u32(mbi + off + 24) as usize;
            let bpp = read_u8(mbi + off + 28);
            serial::write_str("[FB0] detected addr=");
            serial::write_hex(addr);
            serial::write_str(" ");
            serial::write_usize(width);
            serial::write_str("x");
            serial::write_usize(height);
            serial::write_str(" pitch=");
            serial::write_usize(pitch);
            serial::write_str(" bpp=");
            serial::write_usize(bpp as usize);
            serial::write_str("\n");
            vga_fb_mark(0, b'F', b'0');

            // Validate
            if addr == 0
                || width < 320
                || width > 4096
                || height < 200
                || height > 2160
                || pitch < width
                || pitch > width * 8
                || (bpp != 32 && bpp != 24)
            {
                serial::write_str("[FBX] parameters invalid\n");
                vga_fb_mark(0, b'F', b'X');
                return false;
            }
            vga_fb_mark(0, b'F', b'1');
            serial::write_str("[FB1] parameters valid\n");

            let fb_size = pitch.saturating_mul(height);
            if fb_size == 0 || fb_size > 32 * 1024 * 1024 {
                serial::write_str("[FBX] size too large\n");
                vga_fb_mark(0, b'F', b'X');
                return false;
            }

            if !map_fb_range(addr, fb_size) {
                serial::write_str("[FBX] map failed\n");
                vga_fb_mark(0, b'F', b'X');
                return false;
            }
            vga_fb_mark(0, b'F', b'2');
            serial::write_str("[FB2] mapped\n");

            unsafe {
                FB_ADDR = addr;
                FB_W = width;
                FB_H = height;
                FB_PITCH = pitch;
                FB_BPP = bpp;
                FB_OK = true;
            }
            crate::graphics::init(addr, width, height, pitch, bpp, false);
            serial::write_str("[FB] Multiboot mode ");
            serial::write_usize(width);
            serial::write_str("x");
            serial::write_usize(height);
            serial::write_str("\n");
            // CLEAN_INIT_DONE
            return true;
        }
        // next tag 8-byte aligned
        off = (off + tag_size + 7) & !7;
    }
    serial::write_str("[FBX] no framebuffer tag — text mode\n");
    vga_fb_mark(0, b'F', b'X');
    false
}

fn put_pixel(x: usize, y: usize, color: u32) {
    unsafe {
        if !FB_OK || x >= FB_W || y >= FB_H {
            return;
        }
        let bpp = FB_BPP as usize;
        let off = FB_ADDR + y * FB_PITCH + x * (bpp / 8);
        if bpp == 32 {
            core::ptr::write_volatile(off as *mut u32, color);
        } else if bpp == 24 {
            let p = off as *mut u8;
            *p = (color & 0xFF) as u8;
            *p.add(1) = ((color >> 8) & 0xFF) as u8;
            *p.add(2) = ((color >> 16) & 0xFF) as u8;
        }
    }
}

fn fill(color: u32) {
    unsafe {
        if !FB_OK {
            return;
        }
        let mut y = 0usize;
        while y < FB_H {
            let mut x = 0usize;
            while x < FB_W {
                put_pixel(x, y, color);
                x += 1;
            }
            y += 1;
        }
    }
}

fn fill_rect(x0: usize, y0: usize, w: usize, h: usize, color: u32) {
    unsafe {
        if !FB_OK {
            return;
        }
        let mut y = y0;
        while y < y0 + h && y < FB_H {
            let mut x = x0;
            while x < x0 + w && x < FB_W {
                put_pixel(x, y, color);
                x += 1;
            }
            y += 1;
        }
    }
}

/// Test pattern: dark blue fill, green bar, red square, white border
pub fn draw_test_pattern() -> bool {
    if !is_ready() {
        return false;
    }
    serial::write_str("[FB] drawing test pattern...\n");
    // dark blue background
    fill(0x0010_2840);
    // green horizontal bar
    fill_rect(40, 40, unsafe { FB_W.saturating_sub(80) }, 20, 0x0020_C060);
    // red square
    fill_rect(80, 100, 120, 120, 0x00C0_3040);
    // cyan rectangle
    fill_rect(240, 100, 200, 80, 0x0020_A0C0);
    // white corner markers
    fill_rect(0, 0, 16, 16, 0x00FF_FFFF);
    fill_rect(unsafe { FB_W.saturating_sub(16) }, 0, 16, 16, 0x00FF_FFFF);
    fill_rect(0, unsafe { FB_H.saturating_sub(16) }, 16, 16, 0x00FF_FFFF);
    fill_rect(
        unsafe { FB_W.saturating_sub(16) },
        unsafe { FB_H.saturating_sub(16) },
        16,
        16,
        0x00FF_FFFF,
    );
    serial::write_str("[FB3] test pattern rendered\n");
    vga_fb_mark(0, b'F', b'3');
    true
}

/// Entry from kernel: try MBI, pattern, never hang
pub fn try_init(mbi: usize) {
    if init_from_mbi(mbi) {
        let _ = draw_test_pattern();
    } else {
        serial::write_str("[FB] safe fallback — keep text shell\n");
    }
}
