//! Multiboot2 framebuffer bootstrap and compatibility facade.
//!
//! `graphics::Framebuffer` is the single source of truth after initialization.
//! This module only discovers/maps the Multiboot framebuffer and forwards all
//! runtime state and drawing through `graphics`.

use crate::graphics;
use crate::serial;
use crate::mm::paging;

const MB2_TAG_FB: u32 = 8;
const MB2_TAG_END: u32 = 0;

pub fn is_ready() -> bool { graphics::ready() }
pub fn width() -> usize { graphics::width() }
pub fn height() -> usize { graphics::height() }
pub fn address() -> usize { graphics::fb_addr() }
pub fn pitch() -> usize { graphics::pitch() }
pub fn bpp() -> u8 { graphics::bpp() }

/// Compatibility drawing facade. `graphics` remains the only state owner.
pub fn put_pixel(x: usize, y: usize, color: u32) { graphics::put_pixel(x, y, color); }
pub fn fill(color: u32) { graphics::fill(color); }
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    graphics::fill_rect(x, y, w, h, color);
}

fn vga_fb_mark(col: usize, a: u8, b: u8) {
    unsafe {
        let p = (0xB8000 + 80 * 2 + col * 2) as *mut u16;
        *p = 0x0E00u16 | a as u16;
        let p2 = (0xB8000 + 80 * 2 + (col + 1) * 2) as *mut u16;
        *p2 = 0x0E00u16 | b as u16;
    }
}

fn read_u32(p: usize) -> u32 { unsafe { core::ptr::read_unaligned(p as *const u32) } }
fn read_u64(p: usize) -> u64 { unsafe { core::ptr::read_unaligned(p as *const u64) } }
fn read_u8(p: usize) -> u8 { unsafe { *(p as *const u8) } }

/// Identity-map the physical framebuffer when it lies outside the bootstrap map.
fn map_fb_range(phys: usize, size: usize) -> bool {
    if phys == 0 || size == 0 { return false; }
    let cr3 = unsafe { paging::read_cr3() };
    let start = phys & !0xFFF;
    let end = match phys.checked_add(size.saturating_add(0xFFF)) {
        Some(v) => v & !0xFFF,
        None => return false,
    };
    let mut va = start;
    while va < end {
        if va >= 0x4000_0000 {
            let _ = unsafe { paging::map_page(cr3, va, va, paging::PAGE_PRESENT | paging::PAGE_WRITE) };
        }
        va = match va.checked_add(0x1000) { Some(v) => v, None => return false };
    }
    unsafe { paging::load_cr3(cr3) };
    true
}

/// Parse Multiboot2 framebuffer tag and initialize the canonical graphics state.
pub fn init_from_mbi(mbi: usize) -> bool {
    serial::write_str("\n[FB] Multiboot2 scan...\n");
    if mbi == 0 || mbi < 0x1000 {
        serial::write_str("[FBX] no MBI pointer\n");
        vga_fb_mark(0, b'F', b'X');
        return false;
    }

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
        if tag_size < 8 || off + tag_size > total { break; }
        if tag_type == MB2_TAG_END { break; }

        if tag_type == MB2_TAG_FB && tag_size >= 32 {
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

            let bytes_per_pixel = match bpp { 32 => 4usize, 24 => 3usize, _ => 0 };
            let min_pitch = match width.checked_mul(bytes_per_pixel) { Some(v) => v, None => 0 };
            if addr == 0 || width < 320 || width > 4096 || height < 200 || height > 2160
                || bytes_per_pixel == 0 || pitch < min_pitch
            {
                serial::write_str("[FBX] parameters invalid\n");
                vga_fb_mark(0, b'F', b'X');
                return false;
            }
            vga_fb_mark(0, b'F', b'1');
            serial::write_str("[FB1] parameters valid\n");

            let fb_size = match pitch.checked_mul(height) {
                Some(v) if v != 0 && v <= 32 * 1024 * 1024 => v,
                _ => {
                    serial::write_str("[FBX] size invalid\n");
                    vga_fb_mark(0, b'F', b'X');
                    return false;
                }
            };

            if !map_fb_range(addr, fb_size) {
                serial::write_str("[FBX] map failed\n");
                vga_fb_mark(0, b'F', b'X');
                return false;
            }
            vga_fb_mark(0, b'F', b'2');

            graphics::init(addr, width, height, pitch, bpp, false);
            serial::write_str("[FB] SoT -> graphics::FB ");
            serial::write_usize(width);
            serial::write_str("x");
            serial::write_usize(height);
            serial::write_str(" pitch=");
            serial::write_usize(pitch);
            serial::write_str(" bpp=");
            serial::write_usize(bpp as usize);
            serial::write_str("\n");
            return true;
        }
        off = (off + tag_size + 7) & !7;
    }

    serial::write_str("[FBX] no framebuffer tag — text mode\n");
    vga_fb_mark(0, b'F', b'X');
    false
}

/// Test pattern uses the canonical graphics path only.
pub fn draw_test_pattern() -> bool {
    if !is_ready() { return false; }
    serial::write_str("[FB] drawing test pattern...\n");
    let w = graphics::width();
    let h = graphics::height();
    graphics::fill(0x0010_2840);
    graphics::fill_rect(40, 40, w.saturating_sub(80), 20, 0x0020_C060);
    graphics::fill_rect(80, 100, 120, 120, 0x00C0_3040);
    graphics::fill_rect(240, 100, 200, 80, 0x0020_A0C0);
    graphics::fill_rect(0, 0, 16, 16, 0x00FF_FFFF);
    graphics::fill_rect(w.saturating_sub(16), 0, 16, 16, 0x00FF_FFFF);
    graphics::fill_rect(0, h.saturating_sub(16), 16, 16, 0x00FF_FFFF);
    graphics::fill_rect(w.saturating_sub(16), h.saturating_sub(16), 16, 16, 0x00FF_FFFF);
    serial::write_str("[FB] test pattern rendered\n");
    vga_fb_mark(0, b'F', b'3');
    true
}

pub fn try_init(mbi: usize) {
    if init_from_mbi(mbi) {
        let _ = draw_test_pattern();
    } else {
        serial::write_str("[FB] safe fallback — keep text shell\n");
    }
}
