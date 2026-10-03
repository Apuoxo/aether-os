//! Framebuffer drawing primitives + 8x8 font

#[derive(Copy, Clone)]
pub struct PixelFormat {
    pub framebuffer_type: u8,
    pub red_position: u8,
    pub red_mask_size: u8,
    pub green_position: u8,
    pub green_mask_size: u8,
    pub blue_position: u8,
    pub blue_mask_size: u8,
}

impl PixelFormat {
    pub const fn unknown() -> Self {
        Self {
            framebuffer_type: 0xFF,
            red_position: 0,
            red_mask_size: 0,
            green_position: 0,
            green_mask_size: 0,
            blue_position: 0,
            blue_mask_size: 0,
        }
    }

    pub const fn direct_rgb(
        red_position: u8, red_mask_size: u8,
        green_position: u8, green_mask_size: u8,
        blue_position: u8, blue_mask_size: u8,
    ) -> Self {
        Self {
            framebuffer_type: 1,
            red_position,
            red_mask_size,
            green_position,
            green_mask_size,
            blue_position,
            blue_mask_size,
        }
    }

    pub fn compatible_with_renderer(&self, bpp: u8) -> bool {
        self.framebuffer_type == 1
            && self.red_position == 16 && self.red_mask_size == 8
            && self.green_position == 8 && self.green_mask_size == 8
            && self.blue_position == 0 && self.blue_mask_size == 8
            && (bpp == 24 || bpp == 32)
    }
}

pub struct Framebuffer {
    pub addr: usize,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bpp: u8,
    pub software: bool,
    pub pixel_format: PixelFormat,
}

const BACKBUFFER_BYTES: usize = 16 * 1024 * 1024;

#[link_section = ".bss"]
static mut BACKBUFFER: [u8; BACKBUFFER_BYTES] = [0; BACKBUFFER_BYTES];
static mut BACKBUFFER_ACTIVE: bool = false;
static mut DIRTY_MIN_X: usize = usize::MAX;
static mut DIRTY_MIN_Y: usize = usize::MAX;
static mut DIRTY_MAX_X: usize = 0;
static mut DIRTY_MAX_Y: usize = 0;

static mut FB: Framebuffer = Framebuffer {
    addr: 0,
    width: 0,
    height: 0,
    pitch: 0,
    bpp: 0,
    software: false,
    pixel_format: PixelFormat::unknown(),
};

pub fn init(addr: usize, width: usize, height: usize, pitch: usize, bpp: u8, software: bool) {
    let min_pitch = width.saturating_mul(bpp as usize) / 8;
    if pitch < min_pitch {
        crate::serial::write_str("[FB] bad pitch ");
        crate::serial::write_usize(pitch); crate::serial::write_str(" < "); crate::serial::write_usize(min_pitch); crate::serial::write_str("\n");
        return;
    }
    unsafe {
        FB.addr = addr;
        FB.width = width;
        FB.height = height;
        FB.pitch = pitch;
        FB.bpp = bpp;
        FB.software = software;
        FB.pixel_format = PixelFormat::unknown();
        crate::serial::write_str("[FB] set "); crate::serial::write_usize(width); crate::serial::write_str("x"); crate::serial::write_usize(height); crate::serial::write_str(" pitch="); crate::serial::write_usize(pitch); crate::serial::write_str(" bpp="); crate::serial::write_usize(bpp as usize); crate::serial::write_str(" addr="); crate::serial::write_hex(addr); crate::serial::write_str("\n");
        BACKBUFFER_ACTIVE = false;
        DIRTY_MIN_X = usize::MAX;
        DIRTY_MIN_Y = usize::MAX;
        DIRTY_MAX_X = 0;
        DIRTY_MAX_Y = 0;
    }
}

pub fn init_with_format(
    addr: usize, width: usize, height: usize, pitch: usize, bpp: u8,
    software: bool, pixel_format: PixelFormat,
) -> bool {
    let min_pitch = width.saturating_mul(bpp as usize) / 8;
    if pitch < min_pitch || !pixel_format.compatible_with_renderer(bpp) {
        crate::serial::write_str("[FB] unsupported framebuffer format or pitch\n");
        return false;
    }
    init(addr, width, height, pitch, bpp, software);
    unsafe { FB.pixel_format = pixel_format; }
    true
}

pub fn ready() -> bool {
    unsafe { FB.addr != 0 && (FB.bpp == 32 || FB.bpp == 24) }
}
pub fn width() -> usize {
    unsafe { FB.width }
}
pub fn height() -> usize {
    unsafe { FB.height }
}

pub fn pitch() -> usize { unsafe { FB.pitch } }

/// Update the software framebuffer geometry after a successful scanout change.
/// The existing framebuffer address and pitch remain authoritative; reject any
/// geometry that would exceed the fixed RAM backbuffer.
pub fn resize_geometry(width: usize, height: usize) -> bool {
    unsafe {
        if FB.addr == 0 || FB.bpp != 32 || width == 0 || height == 0 {
            return false;
        }
        let min_pitch = width.saturating_mul(4);
        if FB.pitch < min_pitch {
            return false;
        }
        let bytes = match FB.pitch.checked_mul(height) {
            Some(v) if v != 0 && v <= BACKBUFFER_BYTES => v,
            _ => return false,
        };
        FB.width = width;
        FB.height = height;
        if BACKBUFFER_ACTIVE {
            BACKBUFFER_ACTIVE = false;
            if !enable_backbuffer() {
                return false;
            }
        }
        DIRTY_MIN_X = usize::MAX;
        DIRTY_MIN_Y = usize::MAX;
        DIRTY_MAX_X = 0;
        DIRTY_MAX_Y = 0;
        crate::serial::write_str("[FB] resize ");
        crate::serial::write_usize(width);
        crate::serial::write_str("x");
        crate::serial::write_usize(height);
        crate::serial::write_str(" bytes=");
        crate::serial::write_usize(bytes);
        crate::serial::write_str("\n");
        true
    }
}
pub fn fb_addr() -> usize { unsafe { FB.addr } }
pub fn bpp() -> u8 { unsafe { FB.bpp } }
pub fn software() -> bool { unsafe { FB.software } }
pub fn pixel_format() -> PixelFormat { unsafe { FB.pixel_format } }

/// Enable the RAM rendering surface after the framebuffer geometry is known.
/// The desktop compositor paints the complete scene before the first present,
/// so no framebuffer readback is required.
pub fn enable_backbuffer() -> bool {
    unsafe {
        if FB.addr == 0 || FB.bpp != 32 {
            return false;
        }
        let bytes = match FB.pitch.checked_mul(FB.height) {
            Some(v) => v,
            None => return false,
        };
        if bytes == 0 || bytes > BACKBUFFER_BYTES {
            crate::serial::write_str("[FB] backbuffer unavailable bytes=");
            crate::serial::write_usize(bytes);
            crate::serial::write_str("\n");
            return false;
        }
        BACKBUFFER_ACTIVE = true;
        DIRTY_MIN_X = usize::MAX;
        DIRTY_MIN_Y = usize::MAX;
        DIRTY_MAX_X = 0;
        DIRTY_MAX_Y = 0;
        crate::serial::write_str("[FB] backbuffer=ON bytes=");
        crate::serial::write_usize(bytes);
        crate::serial::write_str("\n");
        true
    }
}

pub fn backbuffer_active() -> bool {
    unsafe { BACKBUFFER_ACTIVE }
}

#[inline(always)]
unsafe fn fill_row_u32(dst: *mut u32, count: usize, color: u32) {
    core::arch::asm!(
        "rep stosd",
        inout("rdi") dst => _,
        inout("rcx") count => _,
        inout("eax") color => _,
        options(nostack, preserves_flags)
    );
}

fn mark_dirty(x: usize, y: usize) {
    mark_dirty_rect(x, y, x, y);
}

fn mark_dirty_rect(min_x: usize, min_y: usize, max_x: usize, max_y: usize) {
    unsafe {
        if !BACKBUFFER_ACTIVE { return; }
        if min_x < DIRTY_MIN_X { DIRTY_MIN_X = min_x; }
        if min_y < DIRTY_MIN_Y { DIRTY_MIN_Y = min_y; }
        if max_x > DIRTY_MAX_X { DIRTY_MAX_X = max_x; }
        if max_y > DIRTY_MAX_Y { DIRTY_MAX_Y = max_y; }
    }
}

pub fn put_pixel(x: usize, y: usize, color: u32) {
    unsafe {
        if FB.addr == 0 || x >= FB.width || y >= FB.height {
            return;
        }
        if BACKBUFFER_ACTIVE && FB.bpp == 32 {
            let ptr = BACKBUFFER.as_mut_ptr().add(y * FB.pitch + x * 4) as *mut u32;
            core::ptr::write_unaligned(ptr, color);
            mark_dirty(x, y);
        } else if FB.bpp == 32 {
            let ptr = (FB.addr + y * FB.pitch + x * 4) as *mut u32;
            core::ptr::write_volatile(ptr, color);
        } else if FB.bpp == 24 {
            let p = (FB.addr + y * FB.pitch + x * 3) as *mut u8;
            *p = (color & 0xFF) as u8;
            *p.add(1) = ((color >> 8) & 0xFF) as u8;
            *p.add(2) = ((color >> 16) & 0xFF) as u8;
        }
    }
}

pub fn present() {
    unsafe {
        if !BACKBUFFER_ACTIVE || FB.addr == 0 || FB.bpp != 32 ||
           DIRTY_MIN_X == usize::MAX || DIRTY_MIN_Y == usize::MAX {
            return;
        }
        let min_x = DIRTY_MIN_X;
        let min_y = DIRTY_MIN_Y;
        let max_x = DIRTY_MAX_X.min(FB.width.saturating_sub(1));
        let max_y = DIRTY_MAX_Y.min(FB.height.saturating_sub(1));
        if min_x > max_x || min_y > max_y {
            DIRTY_MIN_X = usize::MAX;
            DIRTY_MIN_Y = usize::MAX;
            return;
        }
        // Stage 5: synchronize the scanout update to vertical blank when
        // the Intel KMS path is actually available. Every wait is bounded;
        // failure to obtain KMS/vblank must never stop the LFB presentation.
        if crate::drivers::intel_kms::ready()
            && crate::drivers::intel_kms::forcewake_get()
        {
            let _ = crate::drivers::intel_kms::wait_vblank();
            crate::drivers::intel_kms::forcewake_put();
        }

        blit_to_framebuffer(min_x, min_y, min_x, min_y,
            max_x - min_x + 1, max_y - min_y + 1);
        DIRTY_MIN_X = usize::MAX;
        DIRTY_MIN_Y = usize::MAX;
        DIRTY_MAX_X = 0;
        DIRTY_MAX_Y = 0;
    }
}

pub fn get_pixel(x: usize, y: usize) -> u32 {
    unsafe {
        if FB.addr == 0 || x >= FB.width || y >= FB.height || FB.bpp != 32 {
            return 0;
        }
        if BACKBUFFER_ACTIVE {
            let ptr = BACKBUFFER.as_ptr().add(y * FB.pitch + x * 4) as *const u32;
            return core::ptr::read_unaligned(ptr);
        }
        let ptr = (FB.addr + y * FB.pitch + x * 4) as *const u32;
        core::ptr::read_volatile(ptr)
    }
}

pub fn fill(color: u32) {
    let w = width();
    let h = height();
    fill_rect(0, 0, w, h, color);
}

/// Fill a clipped rectangle without routing every pixel through put_pixel().
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    let fb_w = width();
    let fb_h = height();
    if w == 0 || h == 0 || x >= fb_w || y >= fb_h { return; }
    let x1 = x.saturating_add(w).min(fb_w);
    let y1 = y.saturating_add(h).min(fb_h);
    if x >= x1 || y >= y1 { return; }
    let count = x1 - x;
    unsafe {
        if BACKBUFFER_ACTIVE && FB.bpp == 32 {
            let value = color.to_ne_bytes();
            let mut row = y;
            while row < y1 {
                let dst = BACKBUFFER.as_mut_ptr().add(row * FB.pitch + x * 4);
                fill_row_u32(dst as *mut u32, count, color);
                row += 1;
            }
            mark_dirty_rect(x, y, x1 - 1, y1 - 1);
        } else if FB.bpp == 32 {
            let mut row = y;
            while row < y1 {
                let dst = (FB.addr + row * FB.pitch + x * 4) as *mut u32;
                let mut i = 0usize;
                while i < count {
                    core::ptr::write_volatile(dst.add(i), color);
                    i += 1;
                }
                row += 1;
            }
        } else if FB.bpp == 24 {
            let mut row = y;
            while row < y1 {
                let mut i = 0usize;
                while i < count {
                    let p = (FB.addr + row * FB.pitch + (x + i) * 3) as *mut u8;
                    *p = (color & 0xFF) as u8;
                    *p.add(1) = ((color >> 8) & 0xFF) as u8;
                    *p.add(2) = ((color >> 16) & 0xFF) as u8;
                    i += 1;
                }
                row += 1;
            }
        }
    }
}

/// Copy a clipped rectangle from the RAM backbuffer to the framebuffer.
pub fn blit_to_framebuffer(src_x: usize, src_y: usize, dst_x: usize, dst_y: usize, w: usize, h: usize) {
    unsafe {
        if !BACKBUFFER_ACTIVE || FB.addr == 0 || FB.bpp != 32 || w == 0 || h == 0 { return; }
        if src_x >= FB.width || src_y >= FB.height || dst_x >= FB.width || dst_y >= FB.height { return; }
        let cw = w.min(FB.width - src_x).min(FB.width - dst_x);
        let ch = h.min(FB.height - src_y).min(FB.height - dst_y);
        if cw == 0 || ch == 0 { return; }
        let row_bytes = cw * 4;
        let mut row = 0usize;
        while row < ch {
            let src = BACKBUFFER.as_ptr().add((src_y + row) * FB.pitch + src_x * 4);
            let dst = (FB.addr + (dst_y + row) * FB.pitch + dst_x * 4) as *mut u8;
            core::ptr::copy_nonoverlapping(src, dst, row_bytes);
            row += 1;
        }
    }
}

pub fn border_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    if w == 0 || h == 0 {
        return;
    }
    fill_rect(x, y, w, 1, color);
    fill_rect(x, y + h - 1, w, 1, color);
    fill_rect(x, y, 1, h, color);
    fill_rect(x + w - 1, y, 1, h, color);
}

pub fn line(x0: usize, y0: usize, x1: usize, y1: usize, color: u32) {
    let mut x0 = x0 as i32;
    let mut y0 = y0 as i32;
    let x1 = x1 as i32;
    let y1 = y1 as i32;
    let dx = if x1 > x0 { x1 - x0 } else { x0 - x1 };
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let dy = if y1 > y0 { y0 - y1 } else { y1 - y0 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 {
            put_pixel(x0 as usize, y0 as usize, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

// Minimal 8x8 font for printable ASCII 0x20-0x7E (partial but usable set)
// Generated compact: index = ch - 0x20
fn glyph(ch: u8) -> [u8; 8] {
    match ch {
        b' ' => [0, 0, 0, 0, 0, 0, 0, 0],
        b'!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
        b'"' => [0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'#' => [0x66, 0xFF, 0x66, 0x66, 0xFF, 0x66, 0x66, 0x00],
        b'$' => [0x18, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x18, 0x00],
        b'%' => [0x62, 0x66, 0x0C, 0x18, 0x30, 0x66, 0x46, 0x00],
        b'&' => [0x3C, 0x66, 0x3C, 0x38, 0x67, 0x66, 0x3F, 0x00],
        b'\'' => [0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'(' => [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00],
        b')' => [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00],
        b'*' => [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00],
        b'+' => [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00],
        b',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30],
        b'-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        b'/' => [0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
        b'0' => [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
        b'1' => [0x18, 0x18, 0x38, 0x18, 0x18, 0x18, 0x7E, 0x00],
        b'2' => [0x3C, 0x66, 0x06, 0x0C, 0x30, 0x60, 0x7E, 0x00],
        b'3' => [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
        b'4' => [0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0x00],
        b'5' => [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
        b'6' => [0x3C, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
        b'7' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00],
        b'8' => [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
        b'9' => [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00],
        b':' => [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00],
        b';' => [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x30],
        b'<' => [0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x00],
        b'=' => [0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00],
        b'>' => [0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x00],
        b'?' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x00],
        b'@' => [0x3C, 0x66, 0x6E, 0x6A, 0x6E, 0x60, 0x3C, 0x00],
        b'A' | b'a' => [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00],
        b'B' | b'b' => [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
        b'C' | b'c' => [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
        b'D' | b'd' => [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
        b'E' | b'e' => [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x7E, 0x00],
        b'F' | b'f' => [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x00],
        b'G' | b'g' => [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3C, 0x00],
        b'H' | b'h' => [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        b'I' | b'i' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
        b'J' | b'j' => [0x06, 0x06, 0x06, 0x06, 0x66, 0x66, 0x3C, 0x00],
        b'K' | b'k' => [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
        b'L' | b'l' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
        b'M' | b'm' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        b'N' | b'n' => [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
        b'O' | b'o' => [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'P' | b'p' => [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
        b'Q' | b'q' => [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00],
        b'R' | b'r' => [0x7C, 0x66, 0x66, 0x7C, 0x78, 0x6C, 0x66, 0x00],
        b'S' | b's' => [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
        b'T' | b't' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        b'U' | b'u' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'V' | b'v' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        b'W' | b'w' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        b'X' | b'x' => [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
        b'Y' | b'y' => [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
        b'Z' | b'z' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
        b'[' => [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00],
        b'\\' => [0x40, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00],
        b']' => [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00],
        b'^' => [0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00],
        b'`' => [0x30, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'{' => [0x0E, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0E, 0x00],
        b'|' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        b'}' => [0x70, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x70, 0x00],
        b'~' => [0x00, 0x00, 0x3A, 0x6C, 0x00, 0x00, 0x00, 0x00],
        _ => [0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00],
    }
}

pub fn draw_char(x: usize, y: usize, c: u8, color: u32) {
    let g = glyph(c);
    let mut row = 0usize;
    while row < 8 {
        let bits = g[row];
        let mut col = 0usize;
        while col < 8 {
            if bits & (0x80 >> col) != 0 {
                put_pixel(x + col, y + row, color);
            }
            col += 1;
        }
        row += 1;
    }
}

pub fn draw_str(x: usize, y: usize, s: &str, color: u32) {
    let mut cx = x;
    for b in s.bytes() {
        if b == b'\n' {
            continue;
        }
        draw_char(cx, y, b, color);
        cx += 8;
    }
}

pub fn draw_bytes(x: usize, y: usize, s: &[u8], color: u32) {
    let mut cx = x;
    let mut i = 0usize;
    while i < s.len() {
        draw_char(cx, y, s[i], color);
        cx += 8;
        i += 1;
    }
}
