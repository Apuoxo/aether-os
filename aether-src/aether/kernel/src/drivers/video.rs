//! Aether video driver — software framebuffer path (no GPU accel yet).
//! Wraps Multiboot2 framebuffer already initialized by graphics/fb.

use crate::graphics;
use crate::serial;

static mut READY: bool = false;
static mut WIDTH: u32 = 0;
static mut HEIGHT: u32 = 0;
static mut PITCH: u32 = 0;
static mut BPP: u8 = 0;
static mut ADDR: u64 = 0;

pub fn init() {
    if !graphics::ready() {
        serial::write_str("[VIDEO] FAIL — no framebuffer\n");
        unsafe { READY = false; }
        return;
    }
    unsafe {
        WIDTH = graphics::width() as u32;
        HEIGHT = graphics::height() as u32;
        // pitch/bpp from graphics module if available
        READY = WIDTH > 0 && HEIGHT > 0;
        ADDR = 0; // physical logged by fb init
    }
    if unsafe { READY } {
        serial::write_str("[VIDEO] OK software FB ");
        serial::write_usize(unsafe { WIDTH as usize });
        serial::write_str("x");
        serial::write_usize(unsafe { HEIGHT as usize });
        serial::write_str("\n");
    }
}

pub fn ready() -> bool {
    unsafe { READY }
}
pub fn width() -> u32 {
    unsafe { WIDTH }
}
pub fn height() -> u32 {
    unsafe { HEIGHT }
}
