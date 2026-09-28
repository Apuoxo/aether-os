//! Native Aether desktop wallpaper for the 1366x768 AH532 graphics test.
//!
//! Source image: "Comeragh Mountains Lake.jpg" by Mik Herman (Citarny),
//! Wikimedia Commons, CC BY-SA 3.0.
//! The CI build downloads the source and converts it to 1366x768 RGB565.
//! At other resolutions the desktop keeps its procedural fallback.

use crate::graphics;

const WIDTH: usize = 1366;
const HEIGHT: usize = 768;
const BYTES_PER_PIXEL: usize = 2;

static IMAGE: &[u8] = include_bytes!("../build/wallpaper.rgb565");

pub fn draw(width: usize, height: usize) -> bool {
    if width != WIDTH || height != HEIGHT || IMAGE.len() < WIDTH * HEIGHT * BYTES_PER_PIXEL {
        return false;
    }
    draw_region(0, 0, WIDTH, HEIGHT);
    true
}

pub fn draw_region(x: usize, y: usize, width: usize, height: usize) -> bool {
    if graphics::width() != WIDTH || graphics::height() != HEIGHT
        || IMAGE.len() < WIDTH * HEIGHT * BYTES_PER_PIXEL
    {
        return false;
    }

    let x0 = x.min(WIDTH);
    let y0 = y.min(HEIGHT);
    let x1 = x.saturating_add(width).min(WIDTH);
    let y1 = y.saturating_add(height).min(HEIGHT);

    let mut py = y0;
    while py < y1 {
        let mut px = x0;
        while px < x1 {
            let off = (py * WIDTH + px) * BYTES_PER_PIXEL;
            let p = (IMAGE[off] as u16) | ((IMAGE[off + 1] as u16) << 8);
            let r = (((p >> 11) & 0x1F) * 255 / 31) as u32;
            let g = (((p >> 5) & 0x3F) * 255 / 63) as u32;
            let b = ((p & 0x1F) * 255 / 31) as u32;
            graphics::put_pixel(px, py, (r << 16) | (g << 8) | b);
            px += 1;
        }
        py += 1;
    }
    true
}
