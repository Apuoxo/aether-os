//! Aether File Manager — real VFS-backed Explorer-style UI

use crate::fs;
use crate::graphics;

const COL_CLIENT: u32 = 0x00ECE9D8;
const COL_TOOL: u32 = 0x00D4D0C8;
const COL_TEXT: u32 = 0x00000000;
const COL_DIM: u32 = 0x00404040;
const COL_SEL: u32 = 0x00316AC5;
const COL_WHITE: u32 = 0x00FFFFFF;
const COL_BORDER: u32 = 0x00404040;
const COL_ADDR: u32 = 0x00FFFFFF;

pub const VIEW_COMPUTER: u8 = 0;
pub const VIEW_ROOT: u8 = 1;
pub const VIEW_PROPS: u8 = 2;
pub const VIEW_TEXT: u8 = 3;

static mut VIEW: u8 = VIEW_COMPUTER;
static mut SEL: i32 = -1;
static mut CTX: bool = false;
static mut CTX_X: i32 = 0;
static mut CTX_Y: i32 = 0;
static mut CTX_ON_ITEM: bool = false;
static mut CONFIRM_DEL: bool = false;
static mut RENAME_MODE: bool = false;
static mut STATUS: [u8; 48] = [0; 48];
static mut STATUS_LEN: usize = 0;
static mut PREVIEW: [u8; 128] = [0; 128];
static mut PREVIEW_LEN: usize = 0;
static mut PREVIEW_NAME: [u8; 24] = [0; 24];
static mut PREVIEW_NL: usize = 0;
static mut HIST: [[u8; 32]; 8] = [[0; 32]; 8];
static mut HIST_LEN: [usize; 8] = [0; 8];
static mut HIST_N: usize = 0;
static mut HIST_I: usize = 0;
static mut CWD: [u8; 32] = [b'/', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
static mut CWD_LEN: usize = 1;


fn path_from_name(name: &[u8], len: usize) -> [u8; 32] {
    let mut path = [0u8; 32];
    path[0] = b'/';
    let mut i = 0usize;
    while i < len && i < 30 {
        path[1 + i] = name[i];
        i += 1;
    }
    path
}
fn path_str(path: &[u8; 32], nlen: usize) -> &str {
    let pl = 1 + nlen;
    if pl > 32 {
        return "/";
    }
    unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(path.as_ptr(), pl)) }
}
fn set_status(s: &[u8]) {
    unsafe {
        let mut i = 0usize;
        while i < 48 {
            STATUS[i] = 0;
            i += 1;
        }
        i = 0;
        while i < s.len() && i < 47 {
            STATUS[i] = s[i];
            i += 1;
        }
        STATUS_LEN = i;
    }
}

fn push_hist() {
    unsafe {
        if HIST_N < 8 {
            let mut i = 0usize;
            while i < 32 {
                HIST[HIST_N][i] = CWD[i];
                i += 1;
            }
            HIST_LEN[HIST_N] = CWD_LEN;
            HIST_N += 1;
            HIST_I = HIST_N;
        }
    }
}

pub fn reset() {
    unsafe {
        VIEW = VIEW_COMPUTER;
        SEL = -1;
        CTX = false;
        CONFIRM_DEL = false;
        RENAME_MODE = false;
        CWD[0] = b'/';
        CWD_LEN = 1;
        HIST_N = 0;
        HIST_I = 0;
        PREVIEW_LEN = 0;
        set_status(b"Ready");
    }
}

pub fn draw(wx: usize, wy: usize, ww: usize, wh: usize, title_h: usize) {
    let body_y = wy + title_h;
    let body_h = wh.saturating_sub(title_h + 3);
    graphics::fill_rect(wx + 3, body_y, ww - 6, body_h, COL_CLIENT);

    // Toolbar: Back Forward Up Refresh | Computer
    let ty = body_y + 4;
    draw_btn(wx + 6, ty, 40, 18, b"Back");
    draw_btn(wx + 50, ty, 52, 18, b"Fwd");
    draw_btn(wx + 106, ty, 28, 18, b"Up");
    draw_btn(wx + 138, ty, 52, 18, b"Refr");
    draw_btn(wx + 194, ty, 64, 18, b"Comp.");

    // Address bar
    let ay = ty + 22;
    graphics::fill_rect(wx + 6, ay, ww - 16, 18, COL_ADDR);
    graphics::border_rect(wx + 6, ay, ww - 16, 18, COL_BORDER);
    graphics::draw_str(wx + 10, ay + 5, "Address:", COL_DIM);
    unsafe {
        let mut i = 0usize;
        while i < CWD_LEN && i < 28 {
            graphics::draw_char(wx + 80 + i * 8, ay + 5, CWD[i], COL_TEXT);
            i += 1;
        }
    }

    let list_y = ay + 24;
    let list_h = body_h.saturating_sub(70);

    unsafe {
        match VIEW {
            VIEW_COMPUTER => draw_computer(wx, list_y, ww, list_h),
            VIEW_ROOT => draw_listing(wx, list_y, ww, list_h),
            VIEW_TEXT => draw_text_view(wx, list_y, ww, list_h),
            VIEW_PROPS => draw_props(wx, list_y, ww, list_h),
            _ => {}
        }
        if CTX {
            draw_context(wx, wy, ww, wh);
        }
        if CONFIRM_DEL {
            draw_confirm(wx, wy, ww, wh);
        }
    }

    // Status bar
    let sy = wy + wh - 18;
    graphics::fill_rect(wx + 3, sy, ww - 6, 16, COL_TOOL);
    graphics::border_rect(wx + 3, sy, ww - 6, 16, COL_BORDER);
    unsafe {
        let mut i = 0usize;
        while i < STATUS_LEN {
            graphics::draw_char(wx + 8 + i * 8, sy + 4, STATUS[i], COL_TEXT);
            i += 1;
        }
    }
}

fn draw_btn(x: usize, y: usize, w: usize, h: usize, label: &[u8]) {
    graphics::fill_rect(x, y, w, h, COL_TOOL);
    graphics::border_rect(x, y, w, h, COL_BORDER);
    let mut i = 0usize;
    while i < label.len() {
        graphics::draw_char(x + 4 + i * 8, y + 5, label[i], COL_TEXT);
        i += 1;
    }
}

fn draw_computer(wx: usize, y: usize, ww: usize, _h: usize) {
    graphics::draw_str(wx + 12, y, "Computer — storage devices", COL_TEXT);
    let mut row = y + 20;
    // Physical ATA if present
    if crate::drivers::ata::hw_present() {
        let sec = crate::drivers::ata::hw_sectors();
        let mb = (sec as u64 * 512) / (1024 * 1024);
        graphics::draw_str(wx + 12, row, "[HDD] Local Disk (C:)", COL_TEXT);
        graphics::draw_str(wx + 20, row + 14, "ATA device — size MB:", COL_DIM);
        draw_num(wx + 180, row + 14, mb as u32);
        let mut model = [0u8; 40];
        let ml = crate::drivers::ata::model_bytes(&mut model);
        if ml > 0 {
            let mut i = 0usize;
            while i < ml && i < 28 {
                graphics::draw_char(wx + 20 + i * 8, row + 28, model[i], COL_DIM);
                i += 1;
            }
        }
        row += 48;
    } else {
        graphics::draw_str(wx + 12, row, "[HDD] Local Disk (C:): not found", COL_DIM);
        row += 18;
    }
    // AetherFS volume
    let total = crate::drivers::ata::total_sectors();
    let files = crate::fs::file_count();
    graphics::draw_str(wx + 12, row, "[Disk] AetherFS (A:)", COL_TEXT);
    graphics::draw_str(wx + 20, row + 14, "Filesystem: AetherFS", COL_DIM);
    if crate::drivers::ata::is_ramdisk() {
        graphics::draw_str(wx + 20, row + 28, "Type: RAM block 32 KiB", COL_DIM);
    } else {
        graphics::draw_str(wx + 20, row + 28, "Type: block device", COL_DIM);
    }
    graphics::draw_str(wx + 20, row + 42, "Sectors:", COL_DIM);
    draw_num(wx + 92, row + 42, total);
    graphics::draw_str(wx + 20, row + 56, "Files:", COL_DIM);
    draw_num(wx + 76, row + 56, files);
    graphics::draw_str(wx + 12, row + 76, "Double-click A: to browse /", COL_DIM);
    graphics::draw_str(wx + 12, row + 92, "SATA/NVMe/USB: Not implemented", COL_DIM);
}


fn draw_listing(wx: usize, y: usize, ww: usize, h: usize) {
    if !fs::is_mounted() {
        graphics::draw_str(wx + 12, y, "AetherFS not mounted", 0x00800000);
        return;
    }
    let mut items = [fs::ListItem {
        name: [0; 24],
        name_len: 0,
        size: 0,
        is_dir: false,
    }; 16];
    let n = fs::list_ex(&mut items);
    unsafe {
        if n == 0 {
            graphics::draw_str(wx + 12, y, "(empty folder)", COL_DIM);
            set_status(b"0 objects");
        } else {
            let mut buf = [0u8; 16];
            let mut p = 0usize;
            let mut v = n;
            if v == 0 {
                buf[0] = b'0';
                p = 1;
            } else {
                let mut d = [0u8; 8];
                let mut c = 0usize;
                while v > 0 {
                    d[c] = (v % 10) as u8 + b'0';
                    v /= 10;
                    c += 1;
                }
                while c > 0 {
                    c -= 1;
                    buf[p] = d[c];
                    p += 1;
                }
            }
            buf[p] = b' ';
            p += 1;
            let s = b"objects";
            let mut i = 0usize;
            while i < s.len() && p < 15 {
                buf[p] = s[i];
                p += 1;
                i += 1;
            }
            set_status(&buf[..p]);
        }
        let mut j = 0usize;
        while j < n {
            let row = y + j * 16;
            if row + 14 > y + h {
                break;
            }
            if SEL == j as i32 {
                graphics::fill_rect(wx + 6, row - 1, ww - 16, 15, COL_SEL);
            }
            let col = if SEL == j as i32 { COL_WHITE } else { COL_TEXT };
            if items[j].is_dir {
                graphics::draw_str(wx + 12, row, "[DIR]", 0x00000080);
            } else {
                graphics::draw_str(wx + 12, row, "[FILE]", COL_DIM);
            }
            let mut k = 0usize;
            while k < items[j].name_len {
                graphics::draw_char(wx + 60 + k * 8, row, items[j].name[k], col);
                k += 1;
            }
            if !items[j].is_dir {
                draw_num(wx + ww.saturating_sub(80), row, items[j].size);
                graphics::draw_str(wx + ww.saturating_sub(40), row, "B", COL_DIM);
            }
            j += 1;
        }
    }
}

fn draw_text_view(wx: usize, y: usize, _ww: usize, _h: usize) {
    graphics::draw_str(wx + 12, y, "File content:", COL_TEXT);
    unsafe {
        let mut i = 0usize;
        while i < PREVIEW_NL {
            graphics::draw_char(wx + 12 + i * 8, y + 16, PREVIEW_NAME[i], COL_DIM);
            i += 1;
        }
        let mut row = 0usize;
        let mut col = 0usize;
        i = 0;
        while i < PREVIEW_LEN {
            let c = PREVIEW[i];
            if c == b'\n' || col > 36 {
                row += 1;
                col = 0;
                if c == b'\n' {
                    i += 1;
                    continue;
                }
            }
            graphics::draw_char(wx + 12 + col * 8, y + 36 + row * 12, c, COL_TEXT);
            col += 1;
            i += 1;
        }
    }
}

fn draw_props(wx: usize, y: usize, _ww: usize, _h: usize) {
    graphics::draw_str(wx + 12, y, "Properties", COL_TEXT);
    unsafe {
        if SEL < 0 {
            graphics::draw_str(wx + 12, y + 20, "No selection", COL_DIM);
            return;
        }
        let mut items = [fs::ListItem {
            name: [0; 24],
            name_len: 0,
            size: 0,
            is_dir: false,
        }; 16];
        let n = fs::list_ex(&mut items);
        let si = SEL as usize;
        if si >= n {
            return;
        }
        graphics::draw_str(wx + 12, y + 20, "Name:", COL_DIM);
        let mut k = 0usize;
        while k < items[si].name_len {
            graphics::draw_char(wx + 60 + k * 8, y + 20, items[si].name[k], COL_TEXT);
            k += 1;
        }
        graphics::draw_str(wx + 12, y + 36, "Path: /", COL_DIM);
        k = 0;
        while k < items[si].name_len {
            graphics::draw_char(wx + 68 + k * 8, y + 36, items[si].name[k], COL_TEXT);
            k += 1;
        }
        if items[si].is_dir {
            graphics::draw_str(wx + 12, y + 52, "Type: Folder", COL_TEXT);
        } else {
            graphics::draw_str(wx + 12, y + 52, "Type: File", COL_TEXT);
            graphics::draw_str(wx + 12, y + 68, "Size:", COL_DIM);
            draw_num(wx + 56, y + 68, items[si].size);
        }
        graphics::draw_str(wx + 12, y + 88, "Filesystem: AetherFS", COL_DIM);
    }
}

fn draw_context(_wx: usize, _wy: usize, _ww: usize, _wh: usize) {
    unsafe {
        let x = CTX_X as usize;
        let y = CTX_Y as usize;
        let h = if CTX_ON_ITEM { 72 } else { 56 };
        graphics::fill_rect(x, y, 120, h, COL_WHITE);
        graphics::border_rect(x, y, 120, h, COL_BORDER);
        if CTX_ON_ITEM {
            graphics::draw_str(x + 8, y + 6, "Open", COL_TEXT);
            graphics::draw_str(x + 8, y + 22, "Delete", COL_TEXT);
            graphics::draw_str(x + 8, y + 38, "Properties", COL_TEXT);
            graphics::draw_str(x + 8, y + 54, "Refresh", COL_TEXT);
        } else {
            graphics::draw_str(x + 8, y + 6, "New Folder", COL_TEXT);
            graphics::draw_str(x + 8, y + 22, "New Text File", COL_TEXT);
            graphics::draw_str(x + 8, y + 38, "Refresh", COL_TEXT);
        }
    }
}

fn draw_confirm(wx: usize, wy: usize, ww: usize, wh: usize) {
    let x = wx + ww / 2 - 90;
    let y = wy + wh / 2 - 40;
    graphics::fill_rect(x, y, 180, 70, COL_WHITE);
    graphics::border_rect(x, y, 180, 70, COL_BORDER);
    graphics::draw_str(x + 16, y + 12, "Delete selected?", COL_TEXT);
    draw_btn(x + 20, y + 40, 50, 18, b"Yes");
    draw_btn(x + 90, y + 40, 50, 18, b"No");
}

fn draw_num(x: usize, y: usize, mut n: u32) {
    if n == 0 {
        graphics::draw_char(x, y, b'0', COL_TEXT);
        return;
    }
    let mut d = [0u8; 10];
    let mut c = 0usize;
    while n > 0 {
        d[c] = (n % 10) as u8 + b'0';
        n /= 10;
        c += 1;
    }
    let mut i = 0usize;
    while i < c {
        graphics::draw_char(x + (c - 1 - i) * 8, y, d[i], COL_TEXT);
        i += 1;
    }
}

/// Mouse click relative to window client. Returns true if handled.
pub fn on_click(wx: i32, wy: i32, ww: i32, wh: i32, title_h: i32, mx: i32, my: i32, right: bool) -> bool {
    let body_y = wy + title_h;
    if my < body_y || my > wy + wh {
        return false;
    }
    let rel_x = mx - wx;
    let rel_y = my - body_y;

    unsafe {
        if CONFIRM_DEL {
            // Yes/No roughly center
            let cx = ww / 2;
            let cy = wh / 2;
            if my > wy + cy && my < wy + cy + 30 {
                if mx < wx + cx {
                    do_delete();
                }
                CONFIRM_DEL = false;
            }
            return true;
        }

        if CTX {
            let hx = CTX_X;
            let hy = CTX_Y;
            if mx >= hx && mx < hx + 120 && my >= hy && my < hy + 80 {
                let row = ((my - hy) / 16) as i32;
                if CTX_ON_ITEM {
                    match row {
                        0 => open_selected(),
                        1 => {
                            CONFIRM_DEL = true;
                            CTX = false;
                        }
                        2 => {
                            VIEW = VIEW_PROPS;
                            CTX = false;
                        }
                        3 => {
                            CTX = false;
                            set_status(b"Refreshed");
                        }
                        _ => {}
                    }
                } else {
                    match row {
                        0 => {
                            let _ = fs::mkdir("/folder");
                            set_status(b"Created folder");
                            CTX = false;
                        }
                        1 => {
                            if fs::create("/hello.txt") {
                                let _ = fs::write("/hello.txt", b"Hello from Files");
                            }
                            set_status(b"Created hello.txt");
                            CTX = false;
                        }
                        2 => {
                            CTX = false;
                            set_status(b"Refreshed");
                        }
                        _ => {}
                    }
                }
                return true;
            }
            CTX = false;
            return true;
        }

        if right {
            CTX = true;
            CTX_X = mx;
            CTX_Y = my;
            let list_top = body_y + 50;
            CTX_ON_ITEM = SEL >= 0 && my >= list_top;
            return true;
        }

        // Toolbar
        if rel_y >= 4 && rel_y < 22 {
            if rel_x >= 6 && rel_x < 46 {
                go_back();
            } else if rel_x >= 50 && rel_x < 102 {
                go_fwd();
            } else if rel_x >= 106 && rel_x < 134 {
                go_up();
            } else if rel_x >= 138 && rel_x < 190 {
                set_status(b"Refreshed");
            } else if rel_x >= 194 && rel_x < 258 {
                VIEW = VIEW_COMPUTER;
                SEL = -1;
                set_status(b"Computer");
            }
            return true;
        }

        // List area
        let list_top = 50;
        if VIEW == VIEW_COMPUTER {
            if rel_y >= list_top && rel_y < list_top + 40 {
                // open root
                push_hist();
                CWD[0] = b'/';
                CWD_LEN = 1;
                VIEW = VIEW_ROOT;
                SEL = -1;
                set_status(b"Opened /");
            }
            return true;
        }

        if VIEW == VIEW_ROOT {
            let row = (rel_y - list_top) / 16;
            if row >= 0 && row < 16 {
                if SEL == row {
                    open_selected();
                } else {
                    SEL = row;
                }
            }
            return true;
        }

        if VIEW == VIEW_TEXT || VIEW == VIEW_PROPS {
            VIEW = VIEW_ROOT;
            return true;
        }
    }
    true
}

fn open_selected() {
    unsafe {
        if SEL < 0 {
            return;
        }
        let mut items = [fs::ListItem {
            name: [0; 24],
            name_len: 0,
            size: 0,
            is_dir: false,
        }; 16];
        let n = fs::list_ex(&mut items);
        let si = SEL as usize;
        if si >= n {
            return;
        }
        if items[si].is_dir {
            // Nested listing not fully supported — show empty with note
            push_hist();
            VIEW = VIEW_ROOT;
            set_status(b"Folder (flat FS note)");
            SEL = -1;
            return;
        }
        // text open
        let mut buf = [0u8; 128];
        if let Some(rn) = fs::read_name(&items[si].name, items[si].name_len, &mut buf) {
            let mut j = 0usize;
            while j < rn && j < 128 {
                PREVIEW[j] = buf[j];
                j += 1;
            }
            PREVIEW_LEN = rn;
            j = 0;
            while j < items[si].name_len && j < 24 {
                PREVIEW_NAME[j] = items[si].name[j];
                j += 1;
            }
            PREVIEW_NL = items[si].name_len;
            VIEW = VIEW_TEXT;
            set_status(b"Opened file");
        } else {
            set_status(b"No app for type");
        }
    }
}

fn do_delete() {
    unsafe {
        if SEL < 0 {
            return;
        }
        let mut items = [fs::ListItem {
            name: [0; 24],
            name_len: 0,
            size: 0,
            is_dir: false,
        }; 16];
        let n = fs::list_ex(&mut items);
        let si = SEL as usize;
        if si >= n {
            return;
        }
        if fs::delete_name(&items[si].name, items[si].name_len) {
            set_status(b"Deleted");
            SEL = -1;
        } else {
            set_status(b"Delete failed");
        }
    }
}

fn go_back() {
    unsafe {
        if HIST_I > 0 {
            HIST_I -= 1;
            let mut i = 0usize;
            while i < 32 {
                CWD[i] = HIST[HIST_I][i];
                i += 1;
            }
            CWD_LEN = HIST_LEN[HIST_I];
            VIEW = if CWD_LEN <= 1 {
                VIEW_ROOT
            } else {
                VIEW_ROOT
            };
            set_status(b"Back");
        } else {
            VIEW = VIEW_COMPUTER;
            set_status(b"Computer");
        }
        SEL = -1;
    }
}

fn go_fwd() {
    unsafe {
        if HIST_I + 1 < HIST_N {
            HIST_I += 1;
            set_status(b"Forward");
        }
        SEL = -1;
    }
}

fn go_up() {
    unsafe {
        VIEW = VIEW_COMPUTER;
        CWD[0] = b'/';
        CWD_LEN = 1;
        SEL = -1;
        set_status(b"Up to Computer");
    }
}
