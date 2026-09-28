//! Bundled native Aether cursor variants.
//! Source artwork: Phinger Cursors by Philipp Schaffrath, CC BY-SA 4.0.
//! CI downloads the SVG sources and rasterizes them to 16x16 RGBA.

use crate::graphics;
pub const COUNT: usize = 24;
pub static C0: &[u8] = include_bytes!("../build/cursor_png/default_red.png");
pub static C1: &[u8] = include_bytes!("../build/cursor_png/default_blue.png");
pub static C2: &[u8] = include_bytes!("../build/cursor_png/default_green.png");
pub static C3: &[u8] = include_bytes!("../build/cursor_png/default_gold.png");
pub static C4: &[u8] = include_bytes!("../build/cursor_png/default_purple.png");
pub static C5: &[u8] = include_bytes!("../build/cursor_png/default_cyan.png");
pub static C6: &[u8] = include_bytes!("../build/cursor_png/pointer_red.png");
pub static C7: &[u8] = include_bytes!("../build/cursor_png/pointer_blue.png");
pub static C8: &[u8] = include_bytes!("../build/cursor_png/pointer_green.png");
pub static C9: &[u8] = include_bytes!("../build/cursor_png/pointer_gold.png");
pub static C10: &[u8] = include_bytes!("../build/cursor_png/pointer_purple.png");
pub static C11: &[u8] = include_bytes!("../build/cursor_png/pointer_cyan.png");
pub static C12: &[u8] = include_bytes!("../build/cursor_png/crosshair_red.png");
pub static C13: &[u8] = include_bytes!("../build/cursor_png/crosshair_blue.png");
pub static C14: &[u8] = include_bytes!("../build/cursor_png/crosshair_green.png");
pub static C15: &[u8] = include_bytes!("../build/cursor_png/crosshair_gold.png");
pub static C16: &[u8] = include_bytes!("../build/cursor_png/crosshair_purple.png");
pub static C17: &[u8] = include_bytes!("../build/cursor_png/crosshair_cyan.png");
pub static C18: &[u8] = include_bytes!("../build/cursor_png/text_red.png");
pub static C19: &[u8] = include_bytes!("../build/cursor_png/text_blue.png");
pub static C20: &[u8] = include_bytes!("../build/cursor_png/text_green.png");
pub static C21: &[u8] = include_bytes!("../build/cursor_png/text_gold.png");
pub static C22: &[u8] = include_bytes!("../build/cursor_png/text_purple.png");
pub static C23: &[u8] = include_bytes!("../build/cursor_png/text_cyan.png");
pub fn name(id: usize) -> &'static str {
    match id {
        0=>"Arrow Red",1=>"Arrow Blue",2=>"Arrow Green",3=>"Arrow Gold",4=>"Arrow Purple",5=>"Arrow Cyan",
        6=>"Pointer Red",7=>"Pointer Blue",8=>"Pointer Green",9=>"Pointer Gold",10=>"Pointer Purple",11=>"Pointer Cyan",
        12=>"Crosshair Red",13=>"Crosshair Blue",14=>"Crosshair Green",15=>"Crosshair Gold",16=>"Crosshair Purple",17=>"Crosshair Cyan",
        18=>"Text Red",19=>"Text Blue",20=>"Text Green",21=>"Text Gold",22=>"Text Purple",_=>"Text Cyan"
    }
}
fn data(id: usize) -> &'static [u8] {
    match id {0=>C0,1=>C1,2=>C2,3=>C3,4=>C4,5=>C5,6=>C6,7=>C7,8=>C8,9=>C9,10=>C10,11=>C11,12=>C12,13=>C13,14=>C14,15=>C15,16=>C16,17=>C17,18=>C18,19=>C19,20=>C20,21=>C21,22=>C22,_=>C23}
}
pub fn draw(id: usize, x: usize, y: usize) {
    let d=data(id.min(COUNT-1));
    if d.len()<1024{return;}
    let mut py=0usize;
    while py<16 { let mut px=0usize; while px<16 {
        let o=(py*16+px)*4; if d[o+3]>8 { graphics::put_pixel(x+px,y+py,((d[o] as u32)<<16)|((d[o+1] as u32)<<8)|(d[o+2] as u32)); }
        px+=1;
    } py+=1; }
}
