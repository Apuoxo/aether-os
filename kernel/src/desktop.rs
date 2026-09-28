//! Aether Desktop v1.1 XP complete UI — Windows XP Luna visual style (homage)

use crate::graphics;
use crate::drivers::ps2;
use crate::fs;
use crate::serial;
use crate::input;

// XP Luna-inspired colors (original Aether theme, not Microsoft assets)
const COL_SKY_TOP: u32 = 0x003A6EA5;
const COL_SKY_MID: u32 = 0x005B9BD5;
const COL_SKY_LOW: u32 = 0x0087CEEB;
const COL_HILL: u32 = 0x003D8B37;
const COL_HILL2: u32 = 0x002E6B28;
const COL_BG: u32 = 0x005B9BD5; // fallback
const COL_TASKBAR: u32 = 0x00245EDC; // classic blue bar
const COL_TASKBAR_TOP: u32 = 0x003C7FB1;
const COL_START: u32 = 0x003C8A2E; // green start
const COL_START_HI: u32 = 0x0055B83A;
const COL_PANEL: u32 = 0x000A246A; // deep blue title
const COL_TITLE_ACT: u32 = 0x000A246A;
const COL_TITLE_INACT: u32 = 0x007A96DF;
const COL_TITLE_TEXT: u32 = 0x00FFFFFF;
const COL_CLIENT: u32 = 0x00ECE9D8; // classic beige
const COL_ACCENT: u32 = 0x00316AC5;
const COL_TITLE: u32 = 0x00FFFFFF;
const COL_TEXT: u32 = 0x00000000;
const COL_TEXT_DIM: u32 = 0x00404040;
const COL_TERM_BG: u32 = 0x00000000;
const COL_TERM_FG: u32 = 0x00C0C0C0;
const COL_WIN_BORDER: u32 = 0x000A246A;
const COL_FOCUS: u32 = 0x000A246A;
const COL_INACTIVE: u32 = 0x007A96DF;
const COL_CURSOR: u32 = 0x00FFFFFF;
const COL_CLOSE: u32 = 0x00E81123;
const COL_MAX: u32 = 0x002D7D46;
const COL_MIN: u32 = 0x002D5A27;
const COL_ABOUT_BG: u32 = 0x00ECE9D8;
const COL_DOCK: u32 = 0x00245EDC;
const COL_BTN_FACE: u32 = 0x00D4D0C8;
const COL_MENU_BG: u32 = 0x00FFFFFF;
const COL_MENU_HDR: u32 = 0x001665CA;

const MAX_WIN: usize = 12;
const TITLE_H: i32 = 26;
const TASKBAR_H: usize = 30;

#[derive(Clone, Copy, PartialEq)]
enum WinKind {
    Terminal,
    About,
    Network,
    Sound,
    Video,
    Files,
    DateTime,
    MyComputer,
    SysProps,
    Settings,
    MediaPlayer,
    Keyboard,
}

struct Window {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    kind: WinKind,
    visible: bool,
    z: i32,
    minimized: bool,
    maximized: bool,
    rx: i32, // restore geometry
    ry: i32,
    rw: i32,
    rh: i32,
}

static mut WINS: [Window; MAX_WIN] = [
    Window { x: 20, y: 36, w: 760, h: 340, kind: WinKind::Terminal, visible: true, z: 5,
        minimized: false, maximized: false, rx: 20, ry: 36, rw: 760, rh: 340 },
    Window { x: 20, y: 36, w: 220, h: 110, kind: WinKind::About, visible: false, z: 1,
        minimized: false, maximized: false, rx: 20, ry: 36, rw: 220, rh: 110 },
    Window { x: 90, y: 55, w: 650, h: 430, kind: WinKind::Network, visible: false, z: 2,
        minimized: false, maximized: false, rx: 90, ry: 55, rw: 650, rh: 430 },
    Window { x: 100, y: 170, w: 280, h: 150, kind: WinKind::Sound, visible: false, z: 3,
        minimized: false, maximized: false, rx: 100, ry: 170, rw: 280, rh: 150 },
    Window { x: 160, y: 190, w: 300, h: 160, kind: WinKind::Video, visible: false, z: 4,
        minimized: false, maximized: false, rx: 160, ry: 190, rw: 300, rh: 160 },
    Window { x: 180, y: 100, w: 340, h: 260, kind: WinKind::Files, visible: false, z: 6,
        minimized: false, maximized: false, rx: 180, ry: 100, rw: 340, rh: 260 },
    Window { x: 280, y: 200, w: 240, h: 140, kind: WinKind::DateTime, visible: false, z: 7,
        minimized: false, maximized: false, rx: 280, ry: 200, rw: 240, rh: 140 },
    Window { x: 80, y: 40, w: 520, h: 360, kind: WinKind::MyComputer, visible: false, z: 8,
        minimized: false, maximized: false, rx: 80, ry: 40, rw: 520, rh: 360 },
    Window { x: 140, y: 50, w: 420, h: 360, kind: WinKind::SysProps, visible: false, z: 9,
        minimized: false, maximized: false, rx: 140, ry: 50, rw: 420, rh: 360 },
    Window { x: 120, y: 60, w: 500, h: 360, kind: WinKind::Settings, visible: false, z: 10,
        minimized: false, maximized: false, rx: 120, ry: 60, rw: 500, rh: 360 },
    Window { x: 70, y: 70, w: 660, h: 390, kind: WinKind::MediaPlayer, visible: false, z: 11,
        minimized: false, maximized: false, rx: 70, ry: 70, rw: 660, rh: 390 },
    Window { x: 30, y: 275, w: 740, h: 295, kind: WinKind::Keyboard, visible: false, z: 12,
        minimized: false, maximized: false, rx: 30, ry: 275, rw: 740, rh: 295 },
];

static mut FOCUS: usize = 0; // terminal
static mut MX: i32 = 400;
static mut MY: i32 = 300;
static mut MB: u8 = 0;
static mut PREV_MB: u8 = 0;
static mut DRAGGING: bool = false;
static mut DRAG_WIN: usize = 0;
static mut DRAG_OX: i32 = 0;
static mut DRAG_OY: i32 = 0;
static mut DRAG_OLD_X: i32 = 0;
static mut DRAG_OLD_Y: i32 = 0;
static mut DRAG_OLD_W: i32 = 0;
static mut DRAG_OLD_H: i32 = 0;
static mut DIRTY_FULL: bool = true;
static mut SELECTED_ICON: usize = usize::MAX;
static mut DIRTY_CURSOR: bool = false;
static mut CURSOR_SAVED: bool = false;
static mut CURSOR_OX: i32 = -1;
static mut CURSOR_OY: i32 = -1;
static mut CURSOR_BG: [u32; 256] = [0; 256]; // 16x16
static mut LAST_SEC: u8 = 255;
static mut RESIZING: bool = false;
static mut RESIZE_WIN: usize = 0;
static mut CLICK_FRAME: u32 = 0;
static mut LAST_CLICK_FRAME: u32 = 0;
static mut LAST_CLICK_WIN: usize = 255;
static mut FRAME_N: u32 = 0;
static mut LAST_FILES_CLICK_FRAME: u32 = 0;
static mut LAST_FILES_CLICK_ROW: i32 = -1;
static mut LAST_DESKTOP_ICON_FRAME: u32 = 0;
static mut LAST_DESKTOP_ICON: usize = usize::MAX;
static mut START_MENU: bool = false;
static mut CTX_MENU: bool = false;
static mut CTX_X: i32 = 0;
static mut CTX_Y: i32 = 0;
static mut Z_TOP: i32 = 3;
static mut SETTINGS_VIEW: u8 = 0;
static mut CURSOR_COLOR: u32 = COL_CURSOR;
static mut CURSOR_PENDING: u8 = 0;

static mut MEDIA_OPEN_DIALOG: bool = false;
static mut MEDIA_DIR: [u8; 96] = [0; 96];
static mut MEDIA_DIR_LEN: usize = 0;
static mut KEYBOARD_TARGET: usize = 0;
static mut KEYBOARD_SHIFT: bool = false;
static mut KEYBOARD_CAPS: bool = false;
static mut KEYBOARD_CTRL: bool = false;
static mut KEYBOARD_ALT: bool = false;


// Windows 7-style Wi-Fi UI state. The list is deliberately empty until the
// native Intel 2230 scan backend returns real 802.11 results.
static mut WIFI_UI_SCAN_REQUESTED: bool = false;
static mut WIFI_UI_SELECTED: i32 = -1;
static mut WIFI_UI_CONNECT_DIALOG: bool = false;
static mut WIFI_UI_PASSWORD_LEN: usize = 0;
static mut WIFI_UI_PASSWORD: [u8; 64] = [0; 64];
static mut WIFI_UI_STATUS: u8 = 0; // 0=idle, 1=scanning, 2=connected, 3=error

// Terminal buffer
const TERM_ROWS: usize = 128;
const TERM_COLS: usize = 128;
const TERM_VIEW_ROWS_MAX: usize = 48; // safety cap; actual viewport follows terminal window height
static mut TERM_LINES: [[u8; TERM_COLS]; TERM_ROWS] = [[0; TERM_COLS]; TERM_ROWS];
static mut TERM_LEN: [usize; TERM_ROWS] = [0; TERM_ROWS];
static mut TERM_ROW: usize = 0;
static mut TERM_VIEW: usize = 0; // 0 = live bottom, larger = scrolled up
static mut TERM_SCROLL_DRAG: bool = false;
// Diagnostic output pager: long command output is shown page-by-page so each
// page fits the terminal and can be captured in one screenshot.
static mut TERM_PAGE_MODE: bool = false;
static mut INPUT: [u8; 64] = [0; 64];
static mut INPUT_LEN: usize = 0;

fn term_clear() {
    unsafe {
        let mut r = 0usize;
        while r < TERM_ROWS {
            TERM_LEN[r] = 0;
            let mut c = 0usize;
            while c < TERM_COLS {
                TERM_LINES[r][c] = 0;
                c += 1;
            }
            r += 1;
        }
        TERM_ROW = 0;
        TERM_VIEW = 0;
    }
}

fn term_text_cols() -> usize {
    unsafe {
        if FOCUS < MAX_WIN && WINS[FOCUS].kind == WinKind::Terminal && WINS[FOCUS].visible {
            let usable = if WINS[FOCUS].w > 34 { (WINS[FOCUS].w - 34) as usize } else { 1 };
            let cols = usable / 8;
            if cols < 8 { 8 } else if cols > TERM_COLS { TERM_COLS } else { cols }
        } else {
            TERM_COLS
        }
    }
}

fn term_visible_rows() -> usize {
    unsafe {
        if FOCUS < MAX_WIN && WINS[FOCUS].kind == WinKind::Terminal && WINS[FOCUS].visible {
            let usable = (WINS[FOCUS].h - TITLE_H - 14) as usize;
            let rows = usable / 10;
            if rows < 2 { 2 } else if rows > TERM_VIEW_ROWS_MAX { TERM_VIEW_ROWS_MAX } else { rows }
        } else {
            2
        }
    }
}

fn term_scroll_up() {
    unsafe {
        let rows = term_visible_rows();
        let max_view = if TERM_ROW + 1 > rows { TERM_ROW + 1 - rows } else { 0 };
        if TERM_VIEW < max_view {
            TERM_VIEW += 1;
            DIRTY_FULL = true;
        }
    }
}

fn term_scroll_down() {
    unsafe {
        if TERM_VIEW > 0 {
            TERM_VIEW -= 1;
            DIRTY_FULL = true;
        }
    }
}

// Advance one full terminal page. Space is deliberately used only while the
// pager is active (or while a manual scroll position exists), so normal
// command input can still contain spaces.
fn term_page_next() {
    unsafe {
        if TERM_VIEW == 0 {
            TERM_PAGE_MODE = false;
            return;
        }
        // Use the same viewport height as draw_window(). The footer occupies
        // the last visible row, so the page step must match the scroll range.
        let rows = term_visible_rows();
        if TERM_VIEW > rows {
            TERM_VIEW -= rows;
        } else {
            TERM_VIEW = 0;
            TERM_PAGE_MODE = false;
        }
        DIRTY_FULL = true;
    }
}

fn term_page_prev() {
    unsafe {
        let total = TERM_ROW + 1;
        let rows = term_visible_rows();
        let max_view = if total > rows { total - rows } else { 0 };
        if max_view == 0 {
            TERM_VIEW = 0;
            TERM_PAGE_MODE = false;
            return;
        }
        if TERM_VIEW == 0 {
            // Re-opened terminal starts at the live bottom. Backspace must
            // enter the previous page from there.
            TERM_VIEW = rows.min(max_view);
            TERM_PAGE_MODE = true;
            DIRTY_FULL = true;
            return;
        }
        if TERM_VIEW < max_view {
            TERM_VIEW = (TERM_VIEW + rows).min(max_view);
            TERM_PAGE_MODE = true;
            DIRTY_FULL = true;
        }
    }
}

fn term_page_begin(start_row: usize) {
    unsafe {
        let total = TERM_ROW + 1;
        let rows = term_visible_rows();
        if total <= rows || total <= start_row {
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            return;
        }
        let output_lines = total - start_row;
        if output_lines <= rows {
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            return;
        }
        let max_view = total - rows;
        let target = if start_row < max_view { start_row } else { max_view };
        TERM_VIEW = max_view - target;
        TERM_PAGE_MODE = true;
        DIRTY_FULL = true;
    }
}

fn term_scroll_set_from_mouse(my: i32) {
    unsafe {
        if FOCUS >= MAX_WIN || !WINS[FOCUS].visible || WINS[FOCUS].kind != WinKind::Terminal {
            return;
        }
        let w = &WINS[FOCUS];
        let bar_top = w.y + TITLE_H + 4;
        let bar_bottom = w.y + w.h - 6;
        let track_top = bar_top + 14;
        let track_bottom = bar_bottom - 14;
        if track_bottom <= track_top { return; }
        let total = TERM_ROW + 1;
        let rows = term_visible_rows();
        let max_view = if total > rows { total - rows } else { 0 };
        if max_view == 0 { TERM_VIEW = 0; return; }
        let track_h = track_bottom - track_top;
        let thumb_h0 = (((track_h as usize) * rows) / total.max(rows)) as i32;
        let thumb_h = if thumb_h0 < 12 { 12 } else if thumb_h0 > track_h { track_h } else { thumb_h0 };
        let travel = track_h - thumb_h;
        if travel <= 0 { TERM_VIEW = 0; return; }
        let mut y = my - track_top - thumb_h / 2;
        if y < 0 { y = 0; }
        if y > travel { y = travel; }
        TERM_VIEW = ((y as usize) * max_view) / (travel as usize);
        DIRTY_FULL = true;
    }
}

fn term_newline() {
    unsafe {
        if TERM_ROW + 1 < TERM_ROWS {
            TERM_ROW += 1;
            TERM_LEN[TERM_ROW] = 0;
        } else {
            let mut r = 0usize;
            while r + 1 < TERM_ROWS {
                let mut c = 0usize;
                while c < TERM_COLS {
                    TERM_LINES[r][c] = TERM_LINES[r + 1][c];
                    c += 1;
                }
                TERM_LEN[r] = TERM_LEN[r + 1];
                r += 1;
            }
            TERM_LEN[TERM_ROWS - 1] = 0;
            let mut c = 0usize;
            while c < TERM_COLS {
                TERM_LINES[TERM_ROWS - 1][c] = 0;
                c += 1;
            }
        }
    }
}

fn term_putc(ch: u8) {
    unsafe {
        if ch == b'\n' {
            term_newline();
            return;
        }
        let cols = term_text_cols();
        if TERM_LEN[TERM_ROW] >= cols {
            term_newline();
        }
        if TERM_LEN[TERM_ROW] < TERM_COLS {
            let c = TERM_LEN[TERM_ROW];
            TERM_LINES[TERM_ROW][c] = ch;
            TERM_LEN[TERM_ROW] = c + 1;
        }
    }
}


fn term_write_hex(mut v: usize) {
    if v == 0 {
        term_putc(b'0');
        return;
    }
    let mut digits = [0u8; 16];
    let mut n = 0usize;
    while v > 0 && n < 16 {
        let d = (v & 0xF) as u8;
        digits[n] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
        v >>= 4;
        n += 1;
    }
    while n > 0 {
        n -= 1;
        term_putc(digits[n]);
    }
}

pub fn terminal_write(s: &str) {
    for b in s.bytes() {
        term_putc(b);
    }
    unsafe {
        DIRTY_FULL = true;
        TERM_VIEW = 0; // new output follows live bottom
    }
}
/// Single-character output bridge for the unified kernel command executor.
pub fn terminal_write_char(ch: u8) {
    term_putc(ch);
    unsafe { DIRTY_FULL = true; }
}


fn eq_cmd(cmd: &[u8], clen: usize, expect: &[u8]) -> bool {
    if clen != expect.len() {
        return false;
    }
    let mut i = 0usize;
    while i < clen {
        if cmd[i] != expect[i] {
            return false;
        }
        i += 1;
    }
    true
}



fn bring_to_front(idx: usize) {
    unsafe {
        Z_TOP += 1;
        WINS[idx].z = Z_TOP;
        FOCUS = idx;
        DIRTY_FULL = true;
    }
}

fn hit_test(mx: i32, my: i32) -> Option<usize> {
    unsafe {
        let mut best: Option<usize> = None;
        let mut best_z = (-2147483647-1);
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible {
                let w = &WINS[i];
                if mx >= w.x && mx < w.x + w.w && my >= w.y && my < w.y + w.h {
                    if w.z >= best_z {
                        best_z = w.z;
                        best = Some(i);
                    }
                }
            }
            i += 1;
        }
        best
    }
}

fn in_title(idx: usize, mx: i32, my: i32) -> bool {
    unsafe {
        let w = &WINS[idx];
        mx >= w.x && mx < w.x + w.w && my >= w.y && my < w.y + TITLE_H
    }
}


fn in_max(idx: usize, mx: i32, my: i32) -> bool {
    unsafe {
        let w = &WINS[idx];
        let cx = w.x + w.w - 42;
        let cy = w.y + 4;
        mx >= cx && mx < cx + 16 && my >= cy && my < cy + 16
    }
}
fn in_min(idx: usize, mx: i32, my: i32) -> bool {
    unsafe {
        let w = &WINS[idx];
        let cx = w.x + w.w - 62;
        let cy = w.y + 4;
        mx >= cx && mx < cx + 16 && my >= cy && my < cy + 16
    }
}

fn toggle_maximize(idx: usize) {
    unsafe {
        let sw = graphics::width() as i32;
        let sh = graphics::height() as i32;
        if WINS[idx].maximized {
            WINS[idx].x = WINS[idx].rx;
            WINS[idx].y = WINS[idx].ry;
            WINS[idx].w = WINS[idx].rw;
            WINS[idx].h = WINS[idx].rh;
            WINS[idx].maximized = false;
        } else {
            WINS[idx].rx = WINS[idx].x;
            WINS[idx].ry = WINS[idx].y;
            WINS[idx].rw = WINS[idx].w;
            WINS[idx].rh = WINS[idx].h;
            WINS[idx].x = 0;
            WINS[idx].y = 30;
            WINS[idx].w = sw;
            WINS[idx].h = sh - 70;
            WINS[idx].maximized = true;
            WINS[idx].minimized = false;
        }
        DIRTY_FULL = true;
    }
}

fn toggle_minimize(idx: usize) {
    unsafe {
        if WINS[idx].minimized {
            WINS[idx].minimized = false;
            WINS[idx].visible = true;
        } else {
            WINS[idx].minimized = true;
            // keep visible=false for main draw; dock icon still shows
            WINS[idx].visible = false;
            // focus another
            let mut i = 0usize;
            while i < MAX_WIN {
                if WINS[i].visible && !WINS[i].minimized {
                    FOCUS = i;
                    break;
                }
                i += 1;
            }
        }
        DIRTY_FULL = true;
    }
}

fn in_resize(idx: usize, mx: i32, my: i32) -> bool {
    unsafe {
        let w = &WINS[idx];
        if w.maximized || !w.visible {
            return false;
        }
        mx >= w.x + w.w - 14 && mx < w.x + w.w && my >= w.y + w.h - 14 && my < w.y + w.h
    }
}

fn win_title(kind: WinKind) -> &'static str {
    match kind {
        WinKind::Terminal => "Terminal",
        WinKind::About => "About",
        WinKind::Network => "Network",
        WinKind::Sound => "Sound",
        WinKind::Video => "Video",
        WinKind::Files => "Windows Explorer",
        WinKind::DateTime => "Date/Time",
        WinKind::MyComputer => "My Computer",
        WinKind::SysProps => "System Properties",
        WinKind::Settings => "Settings",
        WinKind::MediaPlayer => "Aether Media Player",
        WinKind::Keyboard => "On-Screen Keyboard",
    }
}

pub fn open_media_path(path:&str)->bool {
    if !crate::media_player::open(path) {
        return false;
    }
    let _=crate::media_player::add_to_playlist(path);
    // Media Player is window slot 10. Keep the Explorer in the background.
    unsafe {
        WINS[10].minimized=false;
        WINS[10].visible=true;
        bring_to_front(10);
        DIRTY_FULL=true;
    }
    true
}

fn open_win(slot: usize) {
    unsafe {
        if slot == 11 && FOCUS < MAX_WIN && WINS[FOCUS].visible { KEYBOARD_TARGET=FOCUS; }
        if slot == 99 {
            return; // Recycle Bin: empty / not implemented
        }
        if slot >= MAX_WIN {
            return;
        }
        if WINS[slot].kind == WinKind::Settings {
            SETTINGS_VIEW = 0;
            CURSOR_PENDING = 0;
        }
        if WINS[slot].kind == WinKind::Terminal {
            // A closed terminal is reopened at the live bottom. Do not leave
            // stale pager/focus state from the previous window instance.
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            INPUT_LEN = 0;
        }
        let was = WINS[slot].visible && !WINS[slot].minimized;
        WINS[slot].minimized = false;
        WINS[slot].visible = true;
        if slot == 5 && !was {
            crate::files_mgr::reset();
        }
        bring_to_front(slot);
        DIRTY_FULL = true;
    }
}


fn draw_xp_wallpaper(w: usize, h: usize) {
    // Stable gradient sky + hills (no mode override)
    if w < 16 || h < 16 {
        graphics::fill(0x003A6EA5);
        return;
    }
    let mut y = 0usize;
    while y < h {
        let t = (y * 255) / (h + 1);
        let r = 58u32 + (t as u32 * 40) / 255;
        let g = 110u32 + (t as u32 * 50) / 255;
        let b = 165u32;
        let c = (r << 16) | (g << 8) | b;
        graphics::fill_rect(0, y, w, 1, c);
        y += 1;
    }
    // hills
    let h1 = h * 62 / 100;
    graphics::fill_rect(0, h1, w, h - h1, 0x003D8B37);
    let h2 = h * 75 / 100;
    graphics::fill_rect(0, h2, w, h - h2, 0x002E6B28);
    // soft sun
    let sx = w * 3 / 4;
    let sy = h / 6;
    graphics::fill_rect(sx, sy, 36, 36, 0x00FFE066);
}


fn draw_desktop_icons() {
    use crate::gui::icon::{self, IconId};
    use crate::gui::font;
    use crate::gui::theme;
    // Classic desktop layout: column of icons with labels
    let items: [(IconId, usize); 7] = [
        (IconId::MyComputer, 7),
        (IconId::MyDocuments, 5),
        (IconId::Terminal, 0),
        (IconId::Network, 2),
        (IconId::Settings, 4),
        (IconId::RecycleBin, 99),
        (IconId::File, 10),
    ];
    let mut i = 0usize;
    while i < 7 {
        let (id, _slot) = items[i];
        let x = 24usize;
        let y = 36 + i * 72;
        let sel = unsafe { SELECTED_ICON == i };
        icon::blit(id, x, y, sel);
        // label under icon (white with shadow like XP)
        let lab = icon::label(id);
        let tw = lab.len() * 8;
        let lx = if tw < 48 { x + (48 - tw) / 2 } else { x };
        font::draw_shadowed(lx, y + theme::ICON_SIZE + 4, lab, 0x00FFFFFF, 0x00404040);
        i += 1;
    }
}


fn hit_desktop_icon(mx: i32, my: i32) -> Option<usize> {
    // Must match draw_desktop_icons: 7 icons, y = 36 + i*72
    let mut i = 0usize;
    while i < 7 {
        let y = 36 + (i as i32) * 72;
        if mx >= 16 && mx < 80 && my >= y && my < y + 64 {
            return Some(i); // index into items list
        }
        i += 1;
    }
    None
}

fn draw_taskbar_buttons(w: usize, h: usize) {
    let mut x = 78usize; // after Start
    let y = h.saturating_sub(26);
    unsafe {
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible || WINS[i].minimized {
                let title = win_title(WINS[i].kind);
                // short label for taskbar
                let short = match WINS[i].kind {
                    WinKind::Terminal => "Term",
                    WinKind::MyComputer => "Comp",
                    WinKind::Files => "Files",
                    WinKind::Network => "Net",
                    WinKind::Sound => "Snd",
                    WinKind::Video => "Vid",
                    WinKind::SysProps => "Sys",
                    WinKind::DateTime => "Time",
                    WinKind::About => "About",
                    WinKind::Settings => "Settings",
                    WinKind::MediaPlayer => "Media",
                    WinKind::Keyboard => "Keyboard",
                };
                let bw = short.len() * 8 + 20;
                if x + bw > w.saturating_sub(100) {
                    break;
                }
                let pressed = i == FOCUS && WINS[i].visible && !WINS[i].minimized;
                let col = if pressed { 0x001C4F9C } else { 0x003C7FB1 };
                graphics::fill_rect(x, y, bw, 22, col);
                graphics::border_rect(x, y, bw, 22, if pressed { 0x000A246A } else { 0x006B9ACD });
                if pressed {
                    graphics::fill_rect(x + 1, y + 1, bw - 2, 1, 0x000A246A);
                } else {
                    graphics::fill_rect(x + 1, y + 1, bw - 2, 1, 0x006BB0E0);
                }
                graphics::draw_str(x + 10, y + 7, short, COL_TITLE_TEXT);
                x += bw + 3;
            }
            i += 1;
        }
    }
}


fn hit_taskbar(mx: i32, my: i32) -> Option<usize> {
    let h = graphics::height() as i32;
    if my < h - (TASKBAR_H as i32) {
        return None;
    }
    let mut x = 78i32;
    unsafe {
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible || WINS[i].minimized {
                let title = win_title(WINS[i].kind);
                let bw = (title.len() as i32) * 8 + 16;
                if mx >= x && mx < x + bw {
                    return Some(i);
                }
                x += bw + 6;
            }
            i += 1;
        }
    }
    None
}

fn in_close(idx: usize, mx: i32, my: i32) -> bool {
    unsafe {
        let w = &WINS[idx];
        let cx = w.x + w.w - 22;
        let cy = w.y + 4;
        mx >= cx && mx < cx + 16 && my >= cy && my < cy + 16
    }
}

fn clamp_win(idx: usize) {
    unsafe {
        let sw = graphics::width() as i32;
        let sh = graphics::height() as i32;
        if WINS[idx].x < 0 {
            WINS[idx].x = 0;
        }
        if WINS[idx].y < 28 {
            WINS[idx].y = 28;
        }
        if WINS[idx].x + 40 > sw {
            WINS[idx].x = sw - 40;
        }
        if WINS[idx].y + 30 > sh - 40 {
            WINS[idx].y = sh - 70;
        }
    }
}

fn apply_mouse_delta(dx: i32, dy: i32) {
    unsafe {
        let sw = graphics::width() as i32;
        let sh = graphics::height() as i32;
        MX += dx;
        MY += dy;
        if MX < 0 {
            MX = 0;
        }
        if MY < 0 {
            MY = 0;
        }
        if MX >= sw {
            MX = sw - 1;
        }
        if MY >= sh {
            MY = sh - 1;
        }
        DIRTY_FULL = true;
    }
}

fn handle_terminal_scroll_click(mx: i32, my: i32) -> bool {
    unsafe {
        // Resolve the terminal under the pointer, not merely the current focus.
        // This makes the scrollbar usable even when another window owns focus.
        let idx = match hit_window(mx, my) {
            Some(i) if WINS[i].kind == WinKind::Terminal => i,
            _ => return false,
        };
        FOCUS = idx;
        let w = &WINS[idx];
        let bar_x = w.x + w.w - 18;
        let bar_top = w.y + TITLE_H as i32 + 4;
        let bar_bottom = w.y + w.h - 6;
        if mx < bar_x || mx >= bar_x + 14 || my < bar_top || my >= bar_bottom {
            return false;
        }
        let total = TERM_ROW + 1;
        let view_rows = term_visible_rows();
        let max_view = if total > view_rows { total - view_rows } else { 0 };
        if my < bar_top + 14 {
            term_scroll_down();
        } else if my >= bar_bottom - 14 {
            term_scroll_up();
        } else if max_view > 0 {
            let track_top = bar_top + 14;
            let track_bottom = bar_bottom - 14;
            let track_h = track_bottom - track_top;
            let thumb_h0 = (((track_h as usize) * view_rows) / total.max(view_rows)) as i32;
            let thumb_h = if thumb_h0 < 12 { 12 } else if thumb_h0 > track_h { track_h } else { thumb_h0 };
            let travel = track_h - thumb_h;
            let thumb_y = if max_view == 0 || travel <= 0 {
                track_top
            } else {
                track_top + (travel * TERM_VIEW as i32) / max_view as i32
            };
            // Grab the thumb with a small hit tolerance.  Clicking anywhere
            // inside the thumb must start a continuous drag, not a page jump.
            let mut hit_top = thumb_y - 4;
            let mut hit_bottom = thumb_y + thumb_h + 4;
            if hit_top < track_top { hit_top = track_top; }
            if hit_bottom > track_bottom { hit_bottom = track_bottom; }
            if my >= hit_top && my < hit_bottom {
                TERM_SCROLL_DRAG = true;
                term_scroll_set_from_mouse(my);
            } else if my < track_top + track_h / 2 {
                term_scroll_down();
            } else {
                term_scroll_up();
            }
        }
        DIRTY_FULL = true;
        true
    }
}

fn redraw_input_window(idx: usize) {
    unsafe {
        cursor_restore();
        draw_window(idx);
        CURSOR_SAVED = false;
        cursor_save_and_draw(MX, MY);
    }
}

fn media_dir_string<'a>() -> &'a str { unsafe { core::str::from_utf8_unchecked(&MEDIA_DIR[..MEDIA_DIR_LEN]) } }

fn media_open_dialog_reset() {
    unsafe { MEDIA_DIR_LEN=1; MEDIA_DIR[0]=b'/'; MEDIA_OPEN_DIALOG=true; }
}

fn media_item_path(item:&fs::ListItem,out:&mut [u8;96])->Option<usize> {
    unsafe {
        let mut n=0usize;
        while n<MEDIA_DIR_LEN&&n<out.len(){out[n]=MEDIA_DIR[n];n+=1;}
        if n==0{out[0]=b'/';n=1;}
        if n>1&&out[n-1]!=b'/'{if n>=out.len(){return None;}out[n]=b'/';n+=1;}
        let mut i=0usize;
        while i<item.name_len&&n<out.len(){out[n]=item.name[i];n+=1;i+=1;}
        if i!=item.name_len{return None;}Some(n)
    }
}

fn media_open_dialog_click(mx:i32,my:i32,wx:i32,wy:i32)->bool {
    unsafe {
        if !MEDIA_OPEN_DIALOG{return false;}
        let dx=wx+52;let dy=wy+46;let dw=556i32;let dh=292i32;
        if mx<dx||mx>=dx+dw||my<dy||my>=dy+dh{MEDIA_OPEN_DIALOG=false;DIRTY_FULL=true;return true;}
        if my>=dy+30&&my<dy+56{
            if mx>=dx+10&&mx<dx+86{
                if MEDIA_DIR_LEN>1{
                    let mut p=MEDIA_DIR_LEN-1;while p>0&&MEDIA_DIR[p-1]!=b'/'{p-=1;}
                    if p<=1{MEDIA_DIR_LEN=1;MEDIA_DIR[0]=b'/';}else{MEDIA_DIR_LEN=p;}
                }
                DIRTY_FULL=true;return true;
            }
            if mx>=dx+dw-90&&mx<dx+dw-10{MEDIA_OPEN_DIALOG=false;DIRTY_FULL=true;return true;}
        }
        let mut items=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
        let count=fs::list_ex_path(media_dir_string(),&mut items);
        let ly=dy+62;
        if my>=ly&&my<ly+224{
            let row=((my-ly)/14)as usize;
            if row<count{
                let item=items[row];let mut path=[0u8;96];
                if let Some(n)=media_item_path(&item,&mut path){
                    if item.is_dir{
                        let mut j=0usize;while j<n{MEDIA_DIR[j]=path[j];j+=1;}
                        MEDIA_DIR_LEN=n;DIRTY_FULL=true;return true;
                    }
                    if item.name_len>=4{
                        let a=item.name[item.name_len-3]|0x20;let b=item.name[item.name_len-2]|0x20;
                        let c=item.name[item.name_len-1]|0x20;let d=item.name[item.name_len-4]|0x20;
                        let ok=(d==b'.'&&a==b'm'&&b==b'p'&&c==b'3')||(d==b'.'&&a==b'w'&&b==b'a'&&c==b'v');
                        if ok{
                            if let Ok(p)=core::str::from_utf8(&path[..n]){
                                if crate::media_player::open(p){
                                    let _=crate::media_player::add_to_playlist(p);
                                    MEDIA_OPEN_DIALOG=false;WINS[10].visible=true;WINS[10].minimized=false;
                                    bring_to_front(10);DIRTY_FULL=true;
                                }
                            }
                        }
                    }
                }
                return true;
            }
        }
        true
    }
}

fn keyboard_key_to_ascii(label:&str,shift:bool,caps:bool)->Option<u8>{
    let b=label.as_bytes();
    if b.len()!=1{return None;}
    let c=b[0];
    if c>=97&&c<=122{return Some(if shift^caps{c-32}else{c});}
    if c>=48&&c<=57{
        return Some(if shift{match c{
            49=>33,50=>64,51=>35,52=>36,53=>37,54=>94,55=>38,56=>42,57=>40,48=>41,_=>c
        }}else{c});
    }
    Some(match(c,shift){
        (45,false)=>45,(45,true)=>95,(61,false)=>61,(61,true)=>43,
        (91,false)=>91,(91,true)=>123,(93,false)=>93,(93,true)=>125,
        (59,false)=>59,(59,true)=>58,(39,false)=>39,(39,true)=>34,
        (44,false)=>44,(44,true)=>60,(46,false)=>46,(46,true)=>62,
        (47,false)=>47,(47,true)=>63,(92,false)=>92,(92,true)=>124,_=>c
    })
}

fn keyboard_emit(label:&str){
    unsafe{
        let target=KEYBOARD_TARGET;if target>=MAX_WIN||!WINS[target].visible{return;}
        if label=="SHIFT"{KEYBOARD_SHIFT=!KEYBOARD_SHIFT;DIRTY_FULL=true;return;}
        if label=="CAPS"{KEYBOARD_CAPS=!KEYBOARD_CAPS;DIRTY_FULL=true;return;}
        if label=="CTRL"{KEYBOARD_CTRL=!KEYBOARD_CTRL;DIRTY_FULL=true;return;}
        if label=="ALT"{KEYBOARD_ALT=!KEYBOARD_ALT;DIRTY_FULL=true;return;}
        let old=FOCUS;FOCUS=target;
        if label=="ENTER"{handle_key(b'\n');}
        else if label=="BACK"{handle_key(0x08);}
        else if label=="SPACE"{handle_key(b' ');}
        else if label=="ESC"{handle_key(0x1B);}
        else if label=="TAB"{handle_key(0x09);}
        else if label=="UP"{let _=handle_special_key(0x52);}
        else if label=="DOWN"{let _=handle_special_key(0x51);}
        else if label=="LEFT"{let _=handle_special_key(0x50);}
        else if label=="RIGHT"{let _=handle_special_key(0x4F);}
        else if label=="HOME"{let _=handle_special_key(0x4A);}
        else if label=="END"{let _=handle_special_key(0x4D);}
        else if label=="DEL"{let _=handle_special_key(0x4C);}
        else if label.starts_with('F'){
            match label {
                "F1"=>open_win(1), "F2"=>open_win(5), "F3"=>open_win(10),
                "F4"=>open_win(2), "F5"=>open_win(9), "F6"=>open_win(3),
                "F7"=>open_win(4), "F8"=>open_win(7), "F9"=>open_win(6),
                "F10"=>open_win(8), "F11"=>open_win(10), "F12"=>open_win(11),
                _=>{}
            }
        }else if let Some(ch)=keyboard_key_to_ascii(label,KEYBOARD_SHIFT,KEYBOARD_CAPS){
            handle_key(ch);if KEYBOARD_SHIFT{KEYBOARD_SHIFT=false;}
        }
        FOCUS=old;DIRTY_FULL=true;
    }
}

fn handle_keyboard_click(mx:i32,my:i32)->bool{
    unsafe{
        if FOCUS>=MAX_WIN||WINS[FOCUS].kind!=WinKind::Keyboard{return false;}
        let kx=WINS[FOCUS].x+10;let ky=WINS[FOCUS].y+TITLE_H+42;
        let rows:[&[&str];5]=[
            &["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12"],
            &["1","2","3","4","5","6","7","8","9","0","-","=","BACK"],
            &["TAB","q","w","e","r","t","y","u","i","o","p","[","]"],
            &["CAPS","a","s","d","f","g","h","j","k","l",";","'","ENTER"],
            &["SHIFT","z","x","c","v","b","n","m",",",".","/","UP","DOWN"]
        ];
        let widths:[i32;13]=[46,50,50,50,50,50,50,50,50,50,50,50,92];
        let mut r=0usize;
        while r<rows.len(){
            let mut x=kx;let y=ky+(r as i32)*36;let mut c=0usize;
            while c<rows[r].len(){
                let w=widths[c];if mx>=x&&mx<x+w&&my>=y&&my<y+30{keyboard_emit(rows[r][c]);return true;}
                x+=w+3;c+=1;
            }r+=1;
        }
        let y=ky+180;let bottom:[(&str,i32);8]=[("CTRL",66),("ALT",66),("SPACE",270),("LEFT",60),("RIGHT",60),("HOME",60),("END",60),("DEL",60)];
        let mut x=kx;let mut i=0usize;
        while i<bottom.len(){let(label,w)=bottom[i];if mx>=x&&mx<x+w&&my>=y&&my<y+30{keyboard_emit(label);return true;}x+=w+3;i+=1;}
        false
    }
}

fn handle_mouse_buttons(buttons: u8) {
    unsafe {
        let left = buttons & 1;

        // Terminal scrollbar drag has priority over every other mouse handler.
        // Without this early path, a held button can be consumed by window
        // handling before the thumb receives the next mouse position.
        if TERM_SCROLL_DRAG && left != 0 {
            term_scroll_set_from_mouse(MY);
            PREV_MB = buttons;
            MB = buttons;
            DIRTY_FULL = true;
            return;
        }
        let prev_left = PREV_MB & 1;
        let mx = MX;
        let my = MY;
        FRAME_N = FRAME_N.wrapping_add(1);

                // Desktop context menu (right-click empty area)
        let right = buttons & 2;
        let prev_right = PREV_MB & 2;
        if right != 0 && prev_right == 0 {
            // if click on desktop (not on taskbar, not on window)
            let sh = graphics::height() as i32;
            let on_taskbar = my >= sh - (TASKBAR_H as i32);
            let on_win = hit_window(mx, my).is_some();
            if !on_taskbar && !on_win {
                if let Some(act) = hit_ctx_menu(mx, my) {
                    // already open - handle below
                } else if CTX_MENU {
                    if let Some(act) = hit_ctx_menu(mx, my) {
                        let _ = act;
                    } else {
                        CTX_MENU = false;
                        DIRTY_FULL = true;
                    }
                } else {
                    CTX_X = mx;
                    CTX_Y = my;
                    CTX_MENU = true;
                    START_MENU = false;
                    DIRTY_FULL = true;
                }
            }
        }
        if CTX_MENU && (buttons & 1) != 0 && (PREV_MB & 1) == 0 {
            if let Some(act) = hit_ctx_menu(mx, my) {
                CTX_MENU = false;
                DIRTY_FULL = true;
                match act {
                    0 => { DIRTY_FULL = true; } // Refresh
                    1 => {
                        // New text file on AetherFS
                        if crate::fs::is_mounted() {
                            let _ = crate::fs::create("NEW.TXT");
                            let _ = crate::fs::write("NEW.TXT", b"New file\n");
                        }
                    }
                    2 => open_win(0),
                    3 => open_win(7),
                    4 => open_win(8),
                    _ => {}
                }
            } else {
                CTX_MENU = false;
                DIRTY_FULL = true;
            }
        }
// Right-click Files context
        let right = buttons & 2;
        let prev_right = PREV_MB & 2;
        if right != 0 && prev_right == 0 {
            if let Some(idx) = hit_test(mx, my) {
                if WINS[idx].kind == WinKind::Files {
                    bring_to_front(idx);
                    serial::write_str("[MOUSE] right Files hit\n");
                    let changed = crate::files_mgr::on_click(
                        WINS[idx].x, WINS[idx].y, WINS[idx].w, WINS[idx].h,
                        TITLE_H, mx, my, true, false,
                    );
                    if changed { redraw_input_window(idx); }
                }
            }
        }
        if left != 0 && prev_left == 0 {
            // Terminal scrollbar gets first refusal on a mouse-down.
            // Do not let generic window dragging/content handlers consume it.
            if handle_terminal_scroll_click(mx, my) {
                PREV_MB = buttons;
                MB = buttons;
                return;
            }
            // XP Start button (bottom-left)
            let sh = graphics::height() as i32;
            if my >= sh - (TASKBAR_H as i32) && mx < 74 {
                START_MENU = !START_MENU;
                DIRTY_FULL = true;
            }
            // Quick Launch T/F/N
            if my >= sh - (TASKBAR_H as i32) {
                if mx >= 92 && mx < 108 {
                    open_win(0);
                } else if mx >= 112 && mx < 128 {
                    open_win(5);
                } else if mx >= 132 && mx < 148 {
                    open_win(2);
                }
            }
            // Start menu items
            if START_MENU {
                if let Some(act) = hit_start_menu(mx, my) {
                    START_MENU = false;
                    match act {
                        0 => open_win(0), // Terminal
                        1 => open_win(5), // Files
                        2 => open_win(7), // My Computer
                        3 => open_win(2), // Network
                        4 => open_win(9), // Settings
                        5 => open_win(10), // Media Player
                        6 => open_win(5), // Documents -> Files
                        7 => open_win(11), // On-Screen Keyboard
                        8 => open_win(99), // Recycle Bin
                        _ => {}
                    }
                } else if my >= 28 {
                    // click outside menu closes it (unless handled below)
                    // don't close if clicking dock/icons/windows — only empty
                }
            }
            // System tray
            if let Some(ti) = hit_tray(mx, my) {
                match ti {
                    1 => open_win(2), // Network
                    2 => open_win(3), // Sound
                    3 => open_win(4), // Video
                    4 => open_win(6), // DateTime
                    _ => {}
                }
            }
            // Taskbar window buttons
            if let Some(idx) = hit_taskbar(mx, my) {
                open_win(idx);
            }
            // Dock launcher left
            let sh = graphics::height() as i32;
            if my >= sh - (TASKBAR_H as i32) && mx >= 90 && mx < 320 {
                let slot = if mx < 48 { 0 }
                    else if mx < 96 { 2 }
                    else if mx < 144 { 3 }
                    else if mx < 192 { 4 }
                    else if mx < 248 { 5 }
                    else if mx < 320 { 1 }
                    else { 255 };
                if slot < MAX_WIN {
                    open_win(slot);
                }
            }
            // Desktop icons: first click selects, second click opens.
            if let Some(idx) = hit_desktop_icon(mx, my) {
                let dbl = LAST_DESKTOP_ICON == idx
                    && FRAME_N.wrapping_sub(LAST_DESKTOP_ICON_FRAME) < 25;
                SELECTED_ICON = idx;
                LAST_DESKTOP_ICON = idx;
                LAST_DESKTOP_ICON_FRAME = FRAME_N;
                DIRTY_FULL = true;
                if dbl {
                    match idx {
                        0 => open_win(7),
                        1 => open_win(5),
                        2 => open_win(0),
                        3 => open_win(2),
                        4 => open_win(9),
                        6 => open_win(10),
                        _ => {}
                    }
                    LAST_DESKTOP_ICON = usize::MAX;
                }
            }
            // Windows
            if let Some(idx) = hit_test(mx, my) {
                bring_to_front(idx);
                if in_close(idx, mx, my) {
                    WINS[idx].visible = false;
                    WINS[idx].minimized = false;
                    let mut i = 0usize;
                    while i < MAX_WIN {
                        if WINS[i].visible {
                            FOCUS = i;
                            break;
                        }
                        i += 1;
                    }
                    DIRTY_FULL = true;
                } else if in_max(idx, mx, my) {
                    toggle_maximize(idx);
                } else if in_min(idx, mx, my) {
                    toggle_minimize(idx);
                } else if in_resize(idx, mx, my) {
                    RESIZING = true;
                    RESIZE_WIN = idx;
                    DRAG_OLD_W = WINS[idx].w;
                    DRAG_OLD_H = WINS[idx].h;
                } else if WINS[idx].kind == WinKind::Network
                    && my >= WINS[idx].y + TITLE_H
                {
                    let nx = WINS[idx].x;
                    let ny = WINS[idx].y;
                    let nw = WINS[idx].w;
                    let list_y = ny + TITLE_H + 130;
                    let button_y = list_y + 166 + 10;
                    let list_x = nx + 170;

                    // Refresh is a real UI request. It does not fabricate networks:
                    // the driver must later consume this request and return real scan results.
                    if mx >= list_x && mx < list_x + 92
                        && my >= button_y && my < button_y + 26
                    {
                        WIFI_UI_SCAN_REQUESTED = true;
                        WIFI_UI_STATUS = 1;
                        DIRTY_FULL = true;
                    }
                    // Connect only operates on a real selected scan result.
                    if mx >= list_x + 102 && mx < list_x + 206
                        && my >= button_y && my < button_y + 26
                    {
                        if WIFI_UI_SELECTED >= 0 {
                            WIFI_UI_CONNECT_DIALOG = true;
                            WIFI_UI_PASSWORD_LEN = 0;
                            DIRTY_FULL = true;
                        }
                    }
                    // Disconnect is deliberately a no-op until the native association
                    // backend exists; never pretend that a click disconnected hardware.
                    if mx >= list_x + 214 && mx < list_x + 308
                        && my >= button_y && my < button_y + 26
                    {
                        WIFI_UI_STATUS = 0;
                        DIRTY_FULL = true;
                    }
                } else if WINS[idx].kind == WinKind::MyComputer
                    && my >= WINS[idx].y + TITLE_H
                {
                    // My Computer and the Files icon must share one Explorer
                    // implementation. This removes the legacy click path.
                    let right = (buttons & 2) != 0;
                    let mut double_click = false;
                    if !right {
                        let row = (my - (WINS[idx].y + TITLE_H + 40 + 30)) / 32;
                        double_click = LAST_FILES_CLICK_ROW >= 0
                            && FRAME_N.wrapping_sub(LAST_FILES_CLICK_FRAME) < 25
                            && LAST_FILES_CLICK_ROW == row;
                        LAST_FILES_CLICK_FRAME = FRAME_N;
                        LAST_FILES_CLICK_ROW = row;
                    }
                    let changed = crate::files_mgr::on_click(
                        WINS[idx].x, WINS[idx].y, WINS[idx].w, WINS[idx].h,
                        TITLE_H, mx, my, right, double_click,
                    );
                    if changed { redraw_input_window(idx); }
                } else if WINS[idx].kind == WinKind::Files
                    && my >= WINS[idx].y + TITLE_H
                {
                    let right = (buttons & 2) != 0;
                    let mut double_click = false;
                    if !right {
                        let row = (my - (WINS[idx].y + TITLE_H + 40 + 30)) / 32;
                        double_click = LAST_FILES_CLICK_ROW >= 0
                            && FRAME_N.wrapping_sub(LAST_FILES_CLICK_FRAME) < 25
                            && LAST_FILES_CLICK_ROW == row;
                        LAST_FILES_CLICK_FRAME = FRAME_N;
                        LAST_FILES_CLICK_ROW = row;
                    }
                    let changed = crate::files_mgr::on_click(
                        WINS[idx].x, WINS[idx].y, WINS[idx].w, WINS[idx].h,
                        TITLE_H, mx, my, right, double_click,
                    );
                    if changed { redraw_input_window(idx); }
                } else if WINS[idx].kind == WinKind::Settings
                    && my >= WINS[idx].y + TITLE_H
                {
                    let sx = WINS[idx].x;
                    let sy = WINS[idx].y;
                    let sw = WINS[idx].w;
                    let sh = WINS[idx].h;
                    if SETTINGS_VIEW == 0 {
                        // Ten native Settings categories. Mouse is functional;
                        // the rest open explicit future-subsystem placeholders.
                        let mut i = 0usize;
                        while i < 10 {
                            let row_y = sy + 66 + (i as i32) * 25;
                            if mx >= sx + 14 && mx < sx + 177 && my >= row_y - 2 && my < row_y + 21 {
                                SETTINGS_VIEW = (i + 1) as u8;
                                if i == 0 {
                                    CURSOR_PENDING = match CURSOR_COLOR {
                                        0x00000000 => 1,
                                        0x00E81123 => 2,
                                        0x0000A000 => 3,
                                        0x000000CC => 4,
                                        _ => 0,
                                    };
                                }
                                DIRTY_FULL = true;
                                break;
                            }
                            i += 1;
                        }
                    } else if SETTINGS_VIEW == 1 {
                        let colors: [u32; 5] = [0x00FFFFFF, 0x00000000, 0x00E81123, 0x0000A000, 0x000000CC];
                        let mut i = 0usize;
                        while i < 5 {
                            let bx = sx + 28 + (i as i32) * 88;
                            if mx >= bx && mx < bx + 68 && my >= sy + 112 && my < sy + 194 {
                                CURSOR_PENDING = i as u8;
                                DIRTY_FULL = true;
                            }
                            i += 1;
                        }
                        let okx = sx + sw - 184;
                        let cancelx = sx + sw - 94;
                        let by = sy + sh - 42;
                        if my >= by && my < by + 24 {
                            if mx >= okx && mx < okx + 78 {
                                CURSOR_COLOR = colors[CURSOR_PENDING as usize];
                                SETTINGS_VIEW = 0;
                                DIRTY_FULL = true;
                            } else if mx >= cancelx && mx < cancelx + 78 {
                                SETTINGS_VIEW = 0;
                                CURSOR_PENDING = 0;
                                DIRTY_FULL = true;
                            }
                        }
                    } else {
                        // Future-category placeholder: Back returns to Settings home.
                        let bx = sx + sw - 94;
                        let by = sy + sh - 42;
                        if mx >= bx && mx < bx + 78 && my >= by && my < by + 24 {
                            SETTINGS_VIEW = 0;
                            DIRTY_FULL = true;
                        }
                    }
                } else if WINS[idx].kind == WinKind::MediaPlayer
                    && my >= WINS[idx].y + TITLE_H
                {
                    let bx=WINS[idx].x; let by=WINS[idx].y + TITLE_H;
                    let cy=by;
                    let py=cy+154;
                    if my >= cy+240 && my < cy+240+48 {
                        let row=((my-(cy+240))/16) as usize;
                        if row < crate::media_player::builtin_count() && mx >= bx+18 && mx < bx+300 {
                            let _=crate::media_player::select_builtin(row);
                            let _=crate::media_player::open_builtin(row);
                            DIRTY_FULL=true;
                        }
                    } else if my >= py && my < py+28 {
                        if mx >= bx+24 && mx < bx+96 {
                            let _=crate::media_player::previous();
                        } else if mx >= bx+108 && mx < bx+190 {
                            let selected=crate::media_player::selected_builtin();
                            if crate::media_player::data_bytes()==0 {
                                let _=crate::media_player::open_builtin(selected);
                            }
                            crate::media_player::toggle_play();
                        } else if mx >= bx+202 && mx < bx+284 {
                            crate::media_player::stop();
                        } else if mx >= bx+296 && mx < bx+378 {
                            let _=crate::media_player::next();
                        } else if mx >= bx+390 && mx < bx+472 {
                            crate::media_player::toggle_mute();
                        } else if mx >= bx+520 && mx < bx+620 {
                            let rel=(mx-(bx+520)).max(0).min(100) as u32;
                            crate::media_player::set_volume(rel as u8);
                            if crate::media_player::muted() && rel > 0 {
                                crate::media_player::toggle_mute();
                            }
                        }
                        DIRTY_FULL=true;
                    } else if my >= cy+126 && my < cy+142 {
                        let barw=(WINS[idx].w-36).max(1) as i32;
                        let rel=(mx-(bx+18)).max(0).min(barw) as u16;
                        let perm=((rel as u32)*1000/(barw as u32)) as u16;
                        crate::media_player::seek_permille(perm);
                        DIRTY_FULL=true;
                    }
                } else if WINS[idx].kind == WinKind::Sound
                    && my >= WINS[idx].y + TITLE_H
                {
                    let sx=WINS[idx].x;
                    let sy=WINS[idx].y;
                    if my >= sy+54 && my < sy+76 && mx >= sx+12 && mx < sx+232 {
                        let rel=(mx-(sx+12)).max(0).min(220) as u32;
                        crate::media_player::set_volume((rel*100/220) as u8);
                        if crate::media_player::muted() && rel > 0 {
                            crate::media_player::toggle_mute();
                        }
                    } else if my >= sy+78 && my < sy+102 && mx >= sx+12 && mx < sx+86 {
                        crate::media_player::toggle_mute();
                    } else {
                        crate::drivers::audio::beep();
                    }
                    DIRTY_FULL=true;
                } else if in_title(idx, mx, my) {
                    // Double-click title → maximize
                    if LAST_CLICK_WIN == idx && FRAME_N.wrapping_sub(LAST_CLICK_FRAME) < 25 {
                        toggle_maximize(idx);
                        LAST_CLICK_WIN = 255;
                    } else {
                        LAST_CLICK_WIN = idx;
                        LAST_CLICK_FRAME = FRAME_N;
                        if !WINS[idx].maximized {
                            DRAGGING = true;
                            DRAG_WIN = idx;
                            DRAG_OX = mx - WINS[idx].x;
                            DRAG_OY = my - WINS[idx].y;
                            DRAG_OLD_X = WINS[idx].x;
                            DRAG_OLD_Y = WINS[idx].y;
                            DRAG_OLD_W = WINS[idx].w;
                            DRAG_OLD_H = WINS[idx].h;
                        }
                    }
                }
            }
        }
        if left == 0 && prev_left != 0 {
            if DRAGGING || RESIZING || TERM_SCROLL_DRAG {
                DIRTY_FULL = true;
            }
            DRAGGING = false;
            RESIZING = false;
            TERM_SCROLL_DRAG = false;
        }
        if TERM_SCROLL_DRAG && left != 0 {
            term_scroll_set_from_mouse(my);
            PREV_MB = buttons;
            MB = buttons;
            return;
        }
        if DRAGGING && left != 0 {
            WINS[DRAG_WIN].x = mx - DRAG_OX;
            WINS[DRAG_WIN].y = my - DRAG_OY;
            clamp_win(DRAG_WIN);
            render_drag_step();
        }
        if RESIZING && left != 0 {
            let idx = RESIZE_WIN;
            let mut nw = mx - WINS[idx].x;
            let mut nh = my - WINS[idx].y;
            if nw < 160 { nw = 160; }
            if nh < 80 { nh = 80; }
            let sw = graphics::width() as i32;
            let sh = graphics::height() as i32;
            if WINS[idx].x + nw > sw { nw = sw - WINS[idx].x; }
            if WINS[idx].y + nh > sh - 40 { nh = sh - 40 - WINS[idx].y; }
            WINS[idx].w = nw;
            WINS[idx].h = nh;
            WINS[idx].rw = nw;
            WINS[idx].rh = nh;
            DIRTY_FULL = true;
        }
        PREV_MB = buttons;
        MB = buttons;
    }
}

fn draw_cursor(x: i32, y: i32) {
    let x = if x < 0 { 0usize } else { x as usize };
    let y = if y < 0 { 0usize } else { y as usize };
    let col = unsafe { CURSOR_COLOR };
    graphics::fill_rect(x, y, 2, 16, col);
    graphics::fill_rect(x, y, 12, 2, col);
    graphics::fill_rect(x + 2, y + 4, 8, 2, col);
    graphics::fill_rect(x + 2, y + 8, 6, 2, col);
    graphics::put_pixel(x + 3, y + 12, col);
    graphics::put_pixel(x + 4, y + 13, col);
}

fn draw_window(idx: usize) {
    unsafe {
        if !WINS[idx].visible {
            return;
        }
        let w = &WINS[idx];
        let focused = FOCUS == idx;
        let title_bg = if focused { COL_TITLE_ACT } else { COL_TITLE_INACT };
        let border = if focused { COL_FOCUS } else { COL_INACTIVE };
        let wx = w.x as usize;
        let wy = w.y as usize;
        let ww = w.w as usize;
        let wh = w.h as usize;

        // outer frame + drop shadow
        graphics::fill_rect(wx + 3, wy + 3, ww, wh, 0x00404040);
        graphics::fill_rect(wx, wy, ww, wh, COL_CLIENT);
        graphics::border_rect(wx, wy, ww, wh, border);
        graphics::border_rect(wx + 1, wy + 1, ww - 2, wh - 2, 0x00FFFFFF);
        // XP blue/gray title bar
        graphics::fill_rect(wx + 2, wy + 2, ww - 4, TITLE_H as usize - 2, title_bg);
        if focused {
            graphics::fill_rect(wx + 2, wy + 2, ww - 4, 3, 0x00166ACB);
            graphics::fill_rect(wx + 2, wy + TITLE_H as usize - 5, ww - 4, 2, 0x00082A5A);
        }
        // 16x16 window icon in title bar
        {
            use crate::gui::icon::{self, IconId};
            let iid = match w.kind {
                WinKind::Terminal => IconId::Terminal,
                WinKind::MyComputer | WinKind::SysProps => IconId::MyComputer,
                WinKind::Settings => IconId::Settings,
                WinKind::Files => IconId::Folder,
                WinKind::Network => IconId::Network,
                WinKind::Sound | WinKind::Video | WinKind::DateTime => IconId::Settings,
                WinKind::MediaPlayer => IconId::File,
                _ => IconId::File,
            };
            let img = icon::generate(iid);
            let mut yy = 0usize;
            while yy < 16 {
                let mut xx = 0usize;
                while xx < 16 {
                    let c = img[(yy * 2) * 32 + xx * 2];
                    if (c >> 24) > 0 {
                        graphics::put_pixel(wx + 6 + xx, wy + 5 + yy, c & 0x00FFFFFF);
                    }
                    xx += 1;
                }
                yy += 1;
            }
        }
        // title text after icon
        graphics::draw_str(wx + 26, wy + 9, win_title(w.kind), COL_TITLE_TEXT);
        // caption buttons
        let cy = wy + 5;
        let close_x = wx + ww - 22;
        let max_x = wx + ww - 42;
        let min_x = wx + ww - 62;
        graphics::fill_rect(min_x, cy, 16, 14, COL_BTN_FACE);
        graphics::border_rect(min_x, cy, 16, 14, 0x00404040);
        graphics::fill_rect(min_x + 3, cy + 10, 10, 2, 0x00000000);
        graphics::fill_rect(max_x, cy, 16, 14, COL_BTN_FACE);
        graphics::border_rect(max_x, cy, 16, 14, 0x00404040);
        graphics::border_rect(max_x + 3, cy + 3, 10, 8, 0x00000000);
        graphics::fill_rect(close_x, cy, 16, 14, COL_CLOSE);
        graphics::border_rect(close_x, cy, 16, 14, 0x00800000);
        graphics::draw_str(close_x + 4, cy + 3, "X", COL_TITLE_TEXT);
        if !w.maximized {
            graphics::fill_rect(wx + ww - 12, wy + wh - 12, 8, 2, 0x00808080);
            graphics::fill_rect(wx + ww - 12, wy + wh - 8, 8, 2, 0x00808080);
        }

        match w.kind {
            WinKind::Terminal => {
                graphics::fill_rect(
                    wx + 3,
                    wy + TITLE_H as usize,
                    ww - 6,
                    wh - TITLE_H as usize - 3,
                    COL_TERM_BG,
                );
                let total = TERM_ROW + 1;
                let view_rows = term_visible_rows();
                let output_rows = view_rows.saturating_sub(1).max(1);
                let max_start = if total > view_rows { total - view_rows } else { 0 };
                let mut view = TERM_VIEW;
                if view > max_start { view = max_start; }
                let start = max_start - view;
                let mut r = 0usize;
                while r < view_rows {
                    let idx = start + r;
                    if idx < total {
                        let y = wy + TITLE_H as usize + 6 + r * 10;
                        if y + 8 < wy + wh {
                            let len = TERM_LEN[idx];
                            let mut k = 0usize;
                            while k < len {
                                graphics::draw_char(wx + 8 + k * 8, y, TERM_LINES[idx][k], COL_TERM_FG);
                                k += 1;
                            }
                        }
                    }
                    r += 1;
                }
                // Visible terminal scrollbar: up/down buttons + proportional thumb.
                let bar_x = wx + ww - 18;
                let bar_top = wy + TITLE_H as usize + 4;
                let bar_bottom = wy + wh - 6;
                if bar_bottom > bar_top + 24 {
                    graphics::fill_rect(bar_x, bar_top, 14, bar_bottom - bar_top, 0x00303030);
                    graphics::fill_rect(bar_x + 2, bar_top + 2, 10, 10, 0x00606060);
                    graphics::draw_str(bar_x + 3, bar_top + 3, "^", COL_TERM_FG);
                    graphics::fill_rect(bar_x + 2, bar_bottom - 12, 10, 10, 0x00606060);
                    graphics::draw_str(bar_x + 3, bar_bottom - 11, "v", COL_TERM_FG);
                    let track_top = bar_top + 14;
                    let track_bottom = bar_bottom - 14;
                    if track_bottom > track_top {
                        let track_h = track_bottom - track_top;
                        let thumb_h = if max_start == 0 { track_h } else {
                            let h = (track_h * view_rows) / total.max(view_rows);
                            if h < 12 { 12 } else if h > track_h { track_h } else { h }
                        };
                        let travel = track_h - thumb_h;
                        let thumb_y = if max_start == 0 || travel == 0 {
                            track_top
                        } else {
                            track_top + (travel * view) / max_start
                        };
                        graphics::fill_rect(bar_x + 2, thumb_y, 10, thumb_h, COL_ACCENT);
                    }
                }
                let y = wy + TITLE_H as usize + 6 + (view_rows.saturating_sub(1)) * 10;
                if y + 8 < wy + wh && focused && !TERM_PAGE_MODE {
                    graphics::draw_str(wx + 8, y, "aether> ", 0x0000FF00);
                    let mut k = 0usize;
                    while k < INPUT_LEN {
                        graphics::draw_char(wx + 8 + 64 + k * 8, y, INPUT[k], COL_TERM_FG);
                        k += 1;
                    }
                    graphics::fill_rect(wx + 8 + 64 + INPUT_LEN * 8, y, 6, 8, COL_ACCENT);
                }
                let footer_y = wy + wh - 22;
                graphics::fill_rect(wx + 4, footer_y, ww - 8, 18, 0x00202020);
                graphics::border_rect(wx + 4, footer_y, ww - 8, 18, 0x00505050);
                graphics::draw_str(wx + 10, footer_y + 5,
                    "SPACE: NEXT SCREEN  BACKSPACE: PREVIOUS SCREEN",
                    if TERM_PAGE_MODE { 0x00FFFF00 } else { 0x00A0A0A0 });
            }
            WinKind::MediaPlayer => {
                let cy = wy + TITLE_H as usize;
                graphics::fill_rect(wx + 3, cy, ww - 6, wh - TITLE_H as usize - 3, 0x00F2F2F2);
                graphics::fill_rect(wx + 14, cy + 14, ww - 28, 74, 0x001B1B1B);
                let mut tb=[0u8; 96];
                let tn=crate::media_player::title(&mut tb);
                if tn>0 {
                    let mut k=0; while k<tn && k<96 { tb[k]=tb[k]; k+=1; }
                    if tn <= 32 { 
                        let mut title=[0u8; 33]; let mut j=0; while j<tn {title[j]=tb[j];j+=1;} title[tn]=0;
                        // draw_str requires UTF-8; filesystem paths are ASCII in the current player UI.
                        if let Ok(s)=core::str::from_utf8(&title[..tn]) { graphics::draw_str(wx+28,cy+44,s,0x00FFFFFF); }
                    } else { graphics::draw_str(wx+28,cy+44,"Aether audio track",0x00FFFFFF); }
                } else {
                    graphics::draw_str(wx+28,cy+44,"No track loaded",0x00C0C0C0);
                }
                graphics::draw_str(wx+18, cy + 106, "Playback", COL_TEXT_DIM);
                let barx=wx+18; let bary=cy+126; let barw=ww-36;
                graphics::fill_rect(barx,bary,barw,8,0x00C0C0C0);
                let total=crate::media_player::data_bytes();
                let pos=crate::media_player::position_bytes();
                if total>0 { graphics::fill_rect(barx,bary,barw*pos.min(total)/total,8,COL_ACCENT); }
                let by=cy+154;
                graphics::fill_rect(wx+24,by,72,28,COL_BTN_FACE); graphics::border_rect(wx+24,by,72,28,0x00606060);
                graphics::draw_str(wx+43,by+9,"Prev",COL_TEXT);
                graphics::fill_rect(wx+108,by,82,28,COL_BTN_FACE); graphics::border_rect(wx+108,by,82,28,0x00606060);
                graphics::draw_str(wx+128,by+9,"Play",COL_TEXT);
                graphics::fill_rect(wx+202,by,82,28,COL_BTN_FACE); graphics::border_rect(wx+202,by,82,28,0x00606060);
                graphics::draw_str(wx+222,by+9,"Stop",COL_TEXT);
                graphics::fill_rect(wx+296,by,82,28,COL_BTN_FACE); graphics::border_rect(wx+296,by,82,28,0x00606060);
                graphics::draw_str(wx+316,by+9,"Next",COL_TEXT);
                graphics::fill_rect(wx+390,by,82,28,COL_BTN_FACE); graphics::border_rect(wx+390,by,82,28,0x00606060);
                graphics::draw_str(wx+408,by+9,"Mute",COL_TEXT);
                graphics::draw_str(wx+488,by+9,"Vol",COL_TEXT_DIM);
                let vol=crate::media_player::volume() as usize;
                let muted=crate::media_player::muted();
                graphics::fill_rect(wx+520,by+8,100,10,0x00C0C0C0);
                graphics::fill_rect(wx+520,by+8,(100usize*vol/100),10,
                    if muted { 0x00808080 } else { COL_ACCENT });
                let knob_x=wx+520+(100usize*vol/100).min(99);
                graphics::fill_rect(knob_x,by+5,3,16,if muted { 0x00606060 } else { COL_ACCENT });
                if muted {
                    graphics::draw_str(wx+520,by+22,"MUTED",0x00800000);
                } else {
                    draw_u32(wx+568,by+22,vol as u32,COL_TEXT_DIM);
                    graphics::draw_str(wx+588,by+22,"%",COL_TEXT_DIM);
                }
                let st=match crate::media_player::state() {
                    crate::media_player::State::Playing=>"PLAYING",
                    crate::media_player::State::Paused=>"PAUSED",
                    crate::media_player::State::Stopped=>"STOPPED",
                    crate::media_player::State::Error=>"ERROR",
                    _=>"EMPTY",
                };
                graphics::draw_str(wx+18,cy+204,st,COL_TEXT_DIM);
                graphics::draw_str(wx+18,cy+230,"Bundled test media:",COL_TEXT_DIM);
                let selected=crate::media_player::selected_builtin();
                let mut bi=0usize;
                while bi<crate::media_player::builtin_count() {
                    let yy=cy+248+bi*16;
                    let name=crate::media_player::builtin_name(bi);
                    let kind=crate::media_player::builtin_kind(bi);
                    if bi==selected {
                        graphics::fill_rect(wx+20,yy-2,292,14,0x00D8E8FF);
                    }
                    graphics::draw_str(wx+28,yy,name,if bi==selected { 0x00000080 } else { COL_TEXT });
                    graphics::draw_str(wx+112,yy,kind,COL_TEXT_DIM);
                    bi+=1;
                }
                graphics::draw_str(wx+330,cy+248,"Select a track, then Play",COL_TEXT_DIM);
                graphics::draw_str(wx+330,cy+264,"MP3: native decoder",COL_TEXT_DIM);
            }
            WinKind::Keyboard => {
                let ky=wy+TITLE_H as usize+42;let kx=wx+10;
                graphics::fill_rect(wx+3,wy+TITLE_H as usize,ww-6,wh-TITLE_H as usize-3,0x00ECE9D8);
                graphics::draw_str(wx+14,wy+32,"Click keys to type into the previously focused window",COL_TEXT_DIM);
                let rows:[&[&str];5]=[&["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12"],&["1","2","3","4","5","6","7","8","9","0","-","=","BACK"],&["TAB","q","w","e","r","t","y","u","i","o","p","[","]"],&["CAPS","a","s","d","f","g","h","j","k","l",";","'","ENTER"],&["SHIFT","z","x","c","v","b","n","m",",",".","/","UP","DOWN"]];
                let widths:[i32;13]=[46,50,50,50,50,50,50,50,50,50,50,50,92];
                let mut r=0usize;
                while r<rows.len(){let mut x=kx;let y=ky+r*36;let mut c=0usize;
                    while c<rows[r].len(){let label=rows[r][c];let w=widths[c] as usize;
                        let active=unsafe{(label=="SHIFT"&&KEYBOARD_SHIFT)||(label=="CAPS"&&KEYBOARD_CAPS)||(label=="CTRL"&&KEYBOARD_CTRL)||(label=="ALT"&&KEYBOARD_ALT)};
                        graphics::fill_rect(x as usize,y,w,30,if active{0x00B8D4FF}else{COL_BTN_FACE});
                        graphics::border_rect(x as usize,y,w,30,0x00606060);
                        graphics::draw_str(x as usize+(w.saturating_sub(label.len()*8))/2,y+10,label,COL_TEXT);
                        x+=widths[c]+3;c+=1;}r+=1;}
                let y=ky+180;let bottom:[(&str,i32);8]=[("CTRL",66),("ALT",66),("SPACE",270),("LEFT",60),("RIGHT",60),("HOME",60),("END",60),("DEL",60)];
                let mut x=kx;let mut i=0usize;
                while i<bottom.len(){let(label,w)=bottom[i];let active=unsafe{(label=="CTRL"&&KEYBOARD_CTRL)||(label=="ALT"&&KEYBOARD_ALT)};
                    graphics::fill_rect(x as usize,y,w as usize,30,if active{0x00B8D4FF}else{COL_BTN_FACE});
                    graphics::border_rect(x as usize,y,w as usize,30,0x00606060);
                    graphics::draw_str(x as usize+((w as usize).saturating_sub(label.len()*8))/2,y+10,label,COL_TEXT);
                    x+=w+3;i+=1;}
            }
            WinKind::About => {
                                graphics::fill_rect(
                    wx + 3,
                    wy + TITLE_H as usize,
                    ww - 6,
                    wh - TITLE_H as usize - 3,
                    COL_ABOUT_BG,
                );
                graphics::draw_str(wx + 12, wy + 36, "Aether OS", COL_TEXT);
                graphics::draw_str(wx + 12, wy + 50, "Desktop XP style", COL_TEXT_DIM);
                graphics::draw_str(wx + 12, wy + 64, "Native kernel UI", COL_TEXT_DIM);
            }
            WinKind::Network => {
                // Windows 7 Network Connections-inspired surface:
                // light glass/Aero header, navigation pane, network list,
                // signal/security glyphs, and action buttons.
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, wh - TITLE_H as usize - 3, 0x00F4F7FB);

                // Header / task area
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, 58, 0x00EAF2FB);
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize + 57, ww - 6, 1, 0x00B7C9DE);
                graphics::draw_str(wx + 18, wy + TITLE_H as usize + 13, "Connect to a network", 0x001F4E79);
                graphics::draw_str(wx + 18, wy + TITLE_H as usize + 31,
                    "Choose a wireless network to connect to.", 0x00444F5C);

                // Left navigation pane
                graphics::fill_rect(wx + 10, wy + TITLE_H as usize + 68, 142, wh - TITLE_H as usize - 112, 0x00E8EEF5);
                graphics::border_rect(wx + 10, wy + TITLE_H as usize + 68, 142, wh - TITLE_H as usize - 112, 0x00C7D2DF);
                graphics::draw_str(wx + 22, wy + TITLE_H as usize + 88, "Network", 0x001F4E79);
                graphics::draw_str(wx + 22, wy + TITLE_H as usize + 111, "Wireless", 0x00333333);
                graphics::draw_str(wx + 22, wy + TITLE_H as usize + 132, "Intel 2230", 0x005A6673);
                graphics::draw_str(wx + 22, wy + TITLE_H as usize + 164, "Network settings", 0x001E5AA8);
                graphics::draw_str(wx + 22, wy + TITLE_H as usize + 186, "Adapter properties", 0x001E5AA8);

                // Adapter summary
                graphics::draw_str(wx + 174, wy + TITLE_H as usize + 78, "Wireless Network Connection", 0x001F4E79);
                if crate::drivers::wifi::found() {
                    graphics::draw_str(wx + 174, wy + TITLE_H as usize + 96, "Intel Centrino Wireless-N 2230", 0x00333333);
                    graphics::draw_str(wx + 174, wy + TITLE_H as usize + 112, "802.11 b/g/n  |  PCI 8:0.0  |  8086:0887", 0x00606A73);
                } else {
                    graphics::draw_str(wx + 174, wy + TITLE_H as usize + 96, "Wireless adapter not detected", 0x00800000);
                }

                // Network list
                let lx = wx + 170;
                let ly = wy + TITLE_H as usize + 130;
                let lw = ww.saturating_sub(190);
                let lh = 166usize;
                graphics::fill_rect(lx, ly, lw, lh, 0x00FFFFFF);
                graphics::border_rect(lx, ly, lw, lh, 0x00AAB7C5);

                if unsafe { WIFI_UI_STATUS == 1 } {
                    graphics::draw_str(lx + 16, ly + 20, "Searching for wireless networks...", 0x00444F5C);
                } else if unsafe { WIFI_UI_STATUS == 3 } {
                    graphics::draw_str(lx + 16, ly + 20, "Native Wi-Fi scan is not ready.", 0x00800000);
                    graphics::draw_str(lx + 16, ly + 38, "Run WF once, then press Refresh.", 0x00606A73);
                } else {
                    graphics::draw_str(lx + 16, ly + 20, "No networks scanned yet.", 0x00606A73);
                    graphics::draw_str(lx + 16, ly + 38, "Click Refresh to perform a native scan.", 0x00606A73);
                }

                // Selected network row / future real scan data area.
                graphics::fill_rect(lx + 8, ly + 64, lw - 16, 44, 0x00F3F7FC);
                graphics::border_rect(lx + 8, ly + 64, lw - 16, 44, 0x00D5DFEA);
                graphics::draw_str(lx + 20, ly + 75, "SSID", 0x00717D89);
                graphics::draw_str(lx + 20, ly + 91, "Security and signal information will appear here.", 0x00717D89);

                // Windows 7-style command buttons
                let by = ly + lh + 10;
                graphics::fill_rect(lx, by, 92, 26, 0x00F3F3F3);
                graphics::border_rect(lx, by, 92, 26, 0x008A8A8A);
                graphics::draw_str(lx + 22, by + 9, "Refresh", 0x00222222);

                graphics::fill_rect(lx + 102, by, 104, 26, 0x00F3F3F3);
                graphics::border_rect(lx + 102, by, 104, 26, 0x008A8A8A);
                graphics::draw_str(lx + 25, by + 9, "Connect", 0x00222222);

                graphics::fill_rect(lx + 214, by, 94, 26, 0x00F3F3F3);
                graphics::border_rect(lx + 214, by, 94, 26, 0x008A8A8A);
                graphics::draw_str(lx + 24, by + 9, "Disconnect", 0x00222222);

                // Status bar
                let sy = wy + wh - 34;
                graphics::fill_rect(wx + 3, sy, ww - 6, 30, 0x00E8EDF3);
                graphics::fill_rect(wx + 3, sy, ww - 6, 1, 0x00C0CBD7);
                let status = unsafe { WIFI_UI_STATUS };
                if status == 2 {
                    graphics::draw_str(wx + 18, sy + 10, "Connected", 0x00008000);
                } else if status == 1 {
                    graphics::draw_str(wx + 18, sy + 10, "Scanning...", 0x001E5AA8);
                } else {
                    graphics::draw_str(wx + 18, sy + 10,
                        if crate::drivers::wifi::needs_firmware() { "Ready for firmware bring-up" } else { "Adapter ready" },
                        0x005A6673);
                }
                graphics::draw_str(wx + ww - 245, sy + 10, "Aether native Wi-Fi", 0x00717D89);
            }
            WinKind::Sound => {
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, wh - TITLE_H as usize - 3, COL_CLIENT);
                graphics::draw_str(wx + 12, wy + 38, "Master volume", COL_TEXT);
                let vol=crate::media_player::volume() as usize;
                let muted=crate::media_player::muted();
                graphics::fill_rect(wx + 12, wy + 58, 220, 12, 0x00C0C0C0);
                graphics::fill_rect(wx + 12, wy + 58, 220*vol/100, 12,
                    if muted { 0x00808080 } else { COL_ACCENT });
                let knob=wx + 12 + (220*vol/100).min(219);
                graphics::fill_rect(knob, wy + 54, 4, 20, if muted { 0x00606060 } else { COL_ACCENT });
                let mute_x=wx + 12;
                let mute_y=wy + 78;
                graphics::fill_rect(mute_x, mute_y, 74, 24, COL_BTN_FACE);
                graphics::border_rect(mute_x, mute_y, 74, 24, 0x00404040);
                graphics::draw_str(mute_x + 17, mute_y + 8, if muted { "Unmute" } else { "Mute" },
                    if muted { 0x00800000 } else { COL_TEXT });
                draw_u32(wx + 100, mute_y + 8, vol as u32, COL_TEXT_DIM);
                graphics::draw_str(wx + 124, mute_y + 8, "%", COL_TEXT_DIM);
                if crate::drivers::audio::speaker_ok() {
                    graphics::draw_str(wx + 12, wy + 40, "PC Speaker: available", 0x00008000);
                    graphics::draw_str(wx + 12, wy + 56, "Click window = beep test", COL_TEXT_DIM);
                } else {
                    graphics::draw_str(wx + 12, wy + 40, "PC Speaker: not available", 0x00800000);
                }
                if crate::drivers::audio::hda_found() {
                    if crate::drivers::audio::hda_stream_running() {
                        graphics::draw_str(wx + 12, wy + 76, "HDA: found (PCM active)", 0x00008000);
                    } else if crate::drivers::audio::hda_stream_ready() {
                        graphics::draw_str(wx + 12, wy + 76, "HDA: found (PCM ready)", 0x00008000);
                    } else {
                        graphics::draw_str(wx + 12, wy + 76, "HDA: found (PCM not selected)", 0x00808000);
                    }
                } else {
                    graphics::draw_str(wx + 12, wy + 76, "HDA PCM: Not available", COL_TEXT_DIM);
                }
            }
            WinKind::Video => {
                                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, wh - TITLE_H as usize - 3, COL_CLIENT);
                if crate::drivers::video::ready() {
                    graphics::draw_str(wx + 12, wy + 40, "Framebuffer: OK", 0x00008000);
                } else {
                    graphics::draw_str(wx + 12, wy + 40, "Framebuffer: FAIL", 0x00800000);
                }
                graphics::draw_str(wx + 12, wy + 56, "Resolution:", COL_TEXT_DIM);
                // width x height from graphics
                let mut x = wx + 108;
                draw_u32(x, wy + 56, graphics::width() as u32, COL_TEXT);
                x += 40;
                graphics::draw_str(x, wy + 56, "x", COL_TEXT);
                draw_u32(x + 12, wy + 56, graphics::height() as u32, COL_TEXT);
                graphics::draw_str(wx + 12, wy + 76, "Color: 32bpp software", COL_TEXT_DIM);
                graphics::draw_str(wx + 12, wy + 92, "GPU accel: none", COL_TEXT_DIM);
                graphics::draw_str(wx + 12, wy + 108, "Source: Multiboot2 FB", COL_TEXT_DIM);
            }
            WinKind::Files => {
                                crate::files_mgr::draw(wx, wy, ww, wh, TITLE_H as usize);
            }
            WinKind::DateTime => {
                                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, wh - TITLE_H as usize - 3, COL_CLIENT);
                let mut tbuf = [0u8; 20];
                let n = crate::time::format_datetime(&mut tbuf);
                graphics::draw_str(wx + 16, wy + 48, "System clock (RTC)", COL_TEXT_DIM);
                let mut i = 0usize;
                while i < n {
                    graphics::draw_char(wx + 16 + i * 8, wy + 68, tbuf[i], COL_TEXT);
                    i += 1;
                }
                graphics::draw_str(wx + 16, wy + 92, "Source: CMOS 0x70", COL_TEXT_DIM);
            }
            WinKind::MyComputer => {
                // My Computer is the Explorer entry point. Do not render the
                // legacy storage window here: route this window through the
                // single native Windows-7-style Explorer implementation.
                crate::files_mgr::draw(wx, wy, ww, wh, TITLE_H as usize);
            }
            WinKind::Settings => {
                draw_settings(wx, wy, ww, wh);
            }
            WinKind::SysProps => {
                                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, wh - TITLE_H as usize - 3, COL_CLIENT);
                // Tab strip
                graphics::fill_rect(wx + 10, wy + 34, 70, 18, 0x00FFFFFF);
                graphics::border_rect(wx + 10, wy + 34, 70, 18, 0x00808080);
                graphics::draw_str(wx + 18, wy + 38, "General", COL_TEXT);

                graphics::border_rect(wx + 10, wy + 52, ww - 24, 130, 0x00808080);
                graphics::draw_str(wx + 18, wy + 60, "System:", COL_TEXT_DIM);
                graphics::draw_str(wx + 18, wy + 76, "Aether Operating System", COL_TEXT);
                graphics::draw_str(wx + 18, wy + 92, "Native kernel (not Windows)", 0x00800000);
                graphics::draw_str(wx + 18, wy + 108, "Desktop shell: XP-style UI", COL_TEXT_DIM);
                graphics::draw_str(wx + 18, wy + 124, "Ring3 userspace: active", COL_TEXT_DIM);
                graphics::draw_str(wx + 18, wy + 148, "Registered to: Aether User", COL_TEXT_DIM);

                graphics::border_rect(wx + 10, wy + 190, ww - 24, 100, 0x00808080);
                graphics::draw_str(wx + 18, wy + 198, "Computer:", COL_TEXT_DIM);
                graphics::draw_str(wx + 18, wy + 214, "x86-64 compatible processor", COL_TEXT);
                // RAM like XP: "X MB of RAM"
                let pages = crate::mm::total_count();
                let free_p = crate::mm::free_count();
                let ram_kib = (pages as u32) * 4;
                let ram_mib = ram_kib / 1024;
                let free_kib = (free_p as u32) * 4;
                let free_mib = free_kib / 1024;
                graphics::draw_str(wx + 18, wy + 230, "Total RAM:", COL_TEXT);
                if ram_mib > 0 {
                    draw_u32(wx + 110, wy + 230, ram_mib, COL_TEXT);
                    graphics::draw_str(wx + 160, wy + 230, "MB", COL_TEXT);
                } else {
                    draw_u32(wx + 110, wy + 230, ram_kib, COL_TEXT);
                    graphics::draw_str(wx + 170, wy + 230, "KB", COL_TEXT);
                }
                graphics::draw_str(wx + 18, wy + 246, "Available:", COL_TEXT);
                if free_mib > 0 {
                    draw_u32(wx + 110, wy + 246, free_mib, COL_TEXT);
                    graphics::draw_str(wx + 160, wy + 246, "MB", COL_TEXT);
                } else {
                    draw_u32(wx + 110, wy + 246, free_kib, COL_TEXT);
                    graphics::draw_str(wx + 170, wy + 246, "KB", COL_TEXT);
                }
                graphics::draw_str(wx + 18, wy + 262, "Pages total/free:", COL_TEXT_DIM);
                draw_u32(wx + 160, wy + 262, pages as u32, COL_TEXT_DIM);
                graphics::draw_str(wx + 220, wy + 262, "/", COL_TEXT_DIM);
                draw_u32(wx + 230, wy + 262, free_p as u32, COL_TEXT_DIM);

                // Storage summary line
                graphics::draw_str(wx + 12, wy + 300, "Storage: ", COL_TEXT_DIM);
                if crate::drivers::ata::is_hardware() {
                    graphics::draw_str(wx + 84, wy + 300, "ATA HDD present", COL_TEXT);
                } else if crate::drivers::ata::is_ramdisk() {
                    graphics::draw_str(wx + 84, wy + 300, "RAM disk only", COL_TEXT);
                } else {
                    graphics::draw_str(wx + 84, wy + 300, "none", COL_TEXT);
                }
            }
        }
    }
}

fn draw_settings(wx: usize, wy: usize, ww: usize, wh: usize) {
    unsafe {
        graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6, wh - TITLE_H as usize - 3, COL_CLIENT);

        // Settings categories are intentionally present as native placeholders.
        // Only Mouse is functional today; the other pages document the future
        // driver/subsystem surfaces without pretending they are implemented.
        let cats: [&str; 10] = [
            "Mouse",
            "Display",
            "Sound",
            "Network",
            "Date and Time",
            "Keyboard",
            "Storage",
            "Power",
            "Devices",
            "System",
        ];

        if SETTINGS_VIEW == 1 {
            graphics::draw_str(wx + 18, wy + 44, "Mouse", COL_TEXT);
            graphics::draw_str(wx + 18, wy + 62, "Mouse settings", COL_TEXT_DIM);
            graphics::border_rect(wx + 14, wy + 76, ww - 28, 150, 0x00808080);
            graphics::draw_str(wx + 26, wy + 88, "Choose cursor", COL_TEXT);
            let colors: [u32; 5] = [0x00FFFFFF, 0x00000000, 0x00E81123, 0x0000A000, 0x000000CC];
            let names: [&str; 5] = ["White", "Black", "Red", "Green", "Blue"];
            let mut i = 0usize;
            while i < 5 {
                let bx = wx + 28 + i * 88;
                let by = wy + 112;
                let selected = CURSOR_PENDING == i as u8;
                graphics::fill_rect(bx, by, 68, 82, if selected { 0x00DCEBFA } else { 0x00FFFFFF });
                graphics::border_rect(bx, by, 68, 82, if selected { 0x00316AC5 } else { 0x00808080 });
                let cx = bx + 24;
                let cy = by + 10;
                let col = colors[i];
                graphics::fill_rect(cx, cy, 2, 18, col);
                graphics::fill_rect(cx, cy, 13, 2, col);
                graphics::fill_rect(cx + 2, cy + 5, 9, 2, col);
                graphics::fill_rect(cx + 2, cy + 10, 7, 2, col);
                graphics::put_pixel(cx + 3, cy + 14, col);
                graphics::put_pixel(cx + 4, cy + 15, col);
                graphics::draw_str(bx + 8, by + 58, names[i], COL_TEXT);
                i += 1;
            }
            graphics::draw_str(wx + 18, wy + 244, "Current cursor:", COL_TEXT_DIM);
            let current = match CURSOR_PENDING { 0 => "White", 1 => "Black", 2 => "Red", 3 => "Green", _ => "Blue" };
            graphics::draw_str(wx + 126, wy + 244, current, COL_TEXT);
            let okx = wx + ww - 184;
            let cancelx = wx + ww - 94;
            let by = wy + wh - 42;
            graphics::fill_rect(okx, by, 78, 24, COL_BTN_FACE);
            graphics::border_rect(okx, by, 78, 24, 0x00404040);
            graphics::draw_str(okx + 22, by + 8, "Choose", COL_TEXT);
            graphics::fill_rect(cancelx, by, 78, 24, COL_BTN_FACE);
            graphics::border_rect(cancelx, by, 78, 24, 0x00404040);
            graphics::draw_str(cancelx + 22, by + 8, "Cancel", COL_TEXT);
            return;
        }

        if SETTINGS_VIEW == 0 {
            graphics::fill_rect(wx + 8, wy + 34, 175, wh - 50, 0x00FFFFFF);
            graphics::border_rect(wx + 8, wy + 34, 175, wh - 50, 0x00808080);
            graphics::draw_str(wx + 20, wy + 44, "Settings", COL_TEXT);

            let mut i = 0usize;
            while i < cats.len() {
                let row_y = wy + 66 + i * 25;
                let selected = i == 0;
                if selected {
                    graphics::fill_rect(wx + 14, row_y - 2, 163, 23, 0x00DCEBFA);
                    graphics::border_rect(wx + 14, row_y - 2, 163, 23, 0x00316AC5);
                }
                graphics::draw_str(wx + 24, row_y + 4, cats[i], COL_TEXT);
                i += 1;
            }

            graphics::border_rect(wx + 196, wy + 34, ww - 212, wh - 50, 0x00808080);
            graphics::draw_str(wx + 212, wy + 48, "Settings", COL_TEXT);
            graphics::draw_str(wx + 212, wy + 76, "Configure Aether OS.", COL_TEXT_DIM);
            graphics::draw_str(wx + 212, wy + 98, "Mouse", COL_TEXT);
            graphics::draw_str(wx + 212, wy + 116, "Pointer and cursor settings are available.", COL_TEXT_DIM);
            graphics::draw_str(wx + 212, wy + 150, "Other categories are prepared", COL_TEXT_DIM);
            graphics::draw_str(wx + 212, wy + 166, "for future native drivers and services.", COL_TEXT_DIM);
            return;
        }

        // Placeholder page for a future subsystem.
        let idx = (SETTINGS_VIEW - 2) as usize;
        if idx < cats.len() {
            graphics::draw_str(wx + 18, wy + 44, cats[idx], COL_TEXT);
            graphics::draw_str(wx + 18, wy + 62, "Settings", COL_TEXT_DIM);
            graphics::border_rect(wx + 14, wy + 76, ww - 28, wh - 132, 0x00808080);
            graphics::draw_str(wx + 30, wy + 104, "This section is prepared for future", COL_TEXT);
            graphics::draw_str(wx + 30, wy + 122, "native Aether OS implementation.", COL_TEXT);
            graphics::draw_str(wx + 30, wy + 150, "Status: not implemented yet", 0x00808000);
            graphics::draw_str(wx + 30, wy + 178, "No fake controls are exposed.", COL_TEXT_DIM);
            let bx = wx + ww - 94;
            let by = wy + wh - 42;
            graphics::fill_rect(bx, by, 78, 24, COL_BTN_FACE);
            graphics::border_rect(bx, by, 78, 24, 0x00404040);
            graphics::draw_str(bx + 24, by + 8, "Back", COL_TEXT);
        }
    }
}
fn draw_u32(x: usize, y: usize, n: u32, color: u32) {
    if n == 0 {
        graphics::draw_char(x, y, b'0', color);
        return;
    }
    let mut digs = [0u8; 10];
    let mut nd = 0usize;
    let mut v = n;
    while v > 0 && nd < 10 {
        digs[nd] = b'0' + (v % 10) as u8;
        v /= 10;
        nd += 1;
    }
    let mut i = nd;
    let mut cx = x;
    while i > 0 {
        i -= 1;
        graphics::draw_char(cx, y, digs[i], color);
        cx += 8;
    }
}

fn draw_mouse_diag(y: usize) {
    use crate::drivers::ps2;
    use crate::drivers::xhci;
    let ok = 0x0040C060u32;
    let fail = 0x00C04040u32;
    let txt = 0x00E0E8F0u32;
    graphics::draw_str(8, y, "[PS2] ctrl=", txt);
    if ps2::diag_controller_ok() {
        graphics::draw_str(8 + 11 * 8, y, "OK", ok);
    } else {
        graphics::draw_str(8 + 11 * 8, y, "FAIL", fail);
    }
    graphics::draw_str(8 + 16 * 8, y, " ack=", txt);
    if ps2::diag_ack_ok() {
        graphics::draw_str(8 + 21 * 8, y, "OK", ok);
    } else {
        graphics::draw_str(8 + 21 * 8, y, "FAIL", fail);
    }
    graphics::draw_str(8 + 26 * 8, y, " stream=", txt);
    if ps2::diag_stream_ok() {
        graphics::draw_str(8 + 34 * 8, y, "OK", ok);
    } else {
        graphics::draw_str(8 + 34 * 8, y, "FAIL", fail);
    }
    graphics::draw_str(8 + 39 * 8, y, " pkts=", txt);
    draw_u32(8 + 45 * 8, y, ps2::diag_packets(), txt);

    let y2 = y + 12;
    graphics::draw_str(8, y2, "[USB] xHCI=", txt);
    if xhci::diag_xhci_ok() {
        graphics::draw_str(8 + 12 * 8, y2, "OK", ok);
    } else {
        graphics::draw_str(8 + 12 * 8, y2, "FAIL", fail);
    }
    graphics::draw_str(8 + 17 * 8, y2, " mouse=", txt);
    if xhci::diag_dev_found() || xhci::diag_mouse_if() {
        graphics::draw_str(8 + 24 * 8, y2, "FOUND", ok);
    } else {
        graphics::draw_str(8 + 24 * 8, y2, "NONE", fail);
    }
    graphics::draw_str(8 + 30 * 8, y2, " HID=", txt);
    if xhci::diag_hid_ok() {
        graphics::draw_str(8 + 35 * 8, y2, "OK", ok);
    } else {
        graphics::draw_str(8 + 35 * 8, y2, "FAIL", fail);
    }
    graphics::draw_str(8 + 40 * 8, y2, " R=", txt);
    draw_u32(8 + 43 * 8, y2, xhci::diag_reports(), txt);
    graphics::draw_str(8 + 50 * 8, y2, "C=", txt);
    draw_u32(8 + 52 * 8, y2, xhci::diag_completions(), txt);
    graphics::draw_str(8 + 58 * 8, y2, "E=", txt);
    draw_u32(8 + 60 * 8, y2, xhci::diag_mouse_events(), txt);
}



const CURSOR_W: usize = 12;
const CURSOR_H: usize = 16;

fn cursor_restore() {
    unsafe {
        if !CURSOR_SAVED || CURSOR_OX < 0 {
            return;
        }
        let ox = CURSOR_OX as usize;
        let oy = CURSOR_OY as usize;
        let mut row = 0usize;
        while row < CURSOR_H {
            let mut col = 0usize;
            while col < CURSOR_W {
                graphics::put_pixel(ox + col, oy + row, CURSOR_BG[row * CURSOR_W + col]);
                col += 1;
            }
            row += 1;
        }
        CURSOR_SAVED = false;
    }
}

fn cursor_save_and_draw(x: i32, y: i32) {
    unsafe {
        cursor_restore();
        let sw = graphics::width() as i32;
        let sh = graphics::height() as i32;
        let x = if x < 0 { 0 } else if x + CURSOR_W as i32 > sw { sw - CURSOR_W as i32 } else { x };
        let y = if y < 0 { 0 } else if y + CURSOR_H as i32 > sh { sh - CURSOR_H as i32 } else { y };
        let ux = x as usize;
        let uy = y as usize;
        let mut row = 0usize;
        while row < CURSOR_H {
            let mut col = 0usize;
            while col < CURSOR_W {
                CURSOR_BG[row * CURSOR_W + col] = graphics::get_pixel(ux + col, uy + row);
                col += 1;
            }
            row += 1;
        }
        CURSOR_OX = x;
        CURSOR_OY = y;
        CURSOR_SAVED = true;
        draw_cursor(x, y);
    }
}

fn redraw_status_strip() {
    let w = graphics::width();
    let h = graphics::height();
    if w == 0 || h == 0 {
        return;
    }
    let tb = TASKBAR_H;
    let y0 = h.saturating_sub(tb);
    graphics::fill_rect(0, y0, w, tb, COL_TASKBAR);
    graphics::fill_rect(0, y0, w, 2, COL_TASKBAR_TOP);
    draw_taskbar_buttons(w, h);
    draw_status_icons_and_clock(w, h);
    draw_start_button(w, h);
}



fn draw_start_button(_w: usize, h: usize) {
    let tb = TASKBAR_H;
    let ty = h.saturating_sub(tb);
    let sb_w = 72usize;
    graphics::fill_rect(0, ty, sb_w, tb, COL_START);
    graphics::fill_rect(0, ty, sb_w, 3, COL_START_HI);
    graphics::fill_rect(0, ty, 3, tb, COL_START_HI);
    graphics::border_rect(0, ty, sb_w, tb, 0x00FFFFFF);
    graphics::fill_rect(6, ty + 6, 7, 7, 0x00F0F0F0);
    graphics::fill_rect(15, ty + 6, 7, 7, 0x00F0F0F0);
    graphics::fill_rect(6, ty + 15, 7, 7, 0x00F0F0F0);
    graphics::fill_rect(15, ty + 15, 7, 7, 0x00F0F0F0);
    graphics::draw_str(26, ty + 11, "start", 0x00FFFFFF);
}


fn draw_status_icons_and_clock(w: usize, h: usize) {
    let cy = h.saturating_sub(24);
    // Compact tray from right: clock (5 chars HH:MM) + 3 icons 16px
    // layout right-to-left so clock always fits
    let clock_w = 5 * 8 + 8; // "12:34" + pad
    let icon_slot = 20usize;
    let tray_w = clock_w + icon_slot * 3 + 8;
    let tray_x = if w > tray_w + 4 { w - tray_w - 4 } else { 4 };
    // background strip
    graphics::fill_rect(tray_x, cy - 1, tray_w, 18, 0x001C4F9C);
    // Network icon (simple computer+link)
    let nx = tray_x + 2;
    graphics::fill_rect(nx + 2, cy + 3, 8, 6, 0x00C0C0C0);
    graphics::fill_rect(nx + 3, cy + 4, 6, 4, 0x002060A0);
    graphics::fill_rect(nx + 10, cy + 5, 4, 2, 0x00FFFFFF);
    // Volume speaker
    let vx = tray_x + icon_slot + 2;
    graphics::fill_rect(vx + 2, cy + 5, 4, 6, 0x00E0E0E0);
    graphics::fill_rect(vx + 6, cy + 3, 2, 10, 0x00E0E0E0);
    graphics::fill_rect(vx + 9, cy + 4, 2, 2, 0x00A0A0A0);
    graphics::fill_rect(vx + 9, cy + 8, 2, 2, 0x00A0A0A0);
    graphics::fill_rect(vx + 11, cy + 2, 2, 2, 0x00A0A0A0);
    graphics::fill_rect(vx + 11, cy + 10, 2, 2, 0x00A0A0A0);
    // Display monitor
    let dx = tray_x + icon_slot * 2 + 2;
    graphics::fill_rect(dx + 1, cy + 3, 12, 8, 0x00A0A0A0);
    graphics::fill_rect(dx + 2, cy + 4, 10, 6, 0x000A246A);
    graphics::fill_rect(dx + 5, cy + 11, 4, 2, 0x00808080);
    // Clock HH:MM only
    let mut tbuf = [0u8; 20];
    let n = crate::time::format_datetime(&mut tbuf);
    // extract time portion if "YYYY-MM-DD HH:MM" or use last 5 chars
    let (tx, tlen) = if n >= 16 {
        // assume "YYYY-MM-DD HH:MM.."
        (11usize, 5usize)
    } else if n >= 5 {
        (n - 5, 5)
    } else {
        (0usize, n)
    };
    let cx = tray_x + icon_slot * 3 + 4;
    let mut i = 0usize;
    while i < tlen && tx + i < n {
        graphics::draw_char(cx + i * 8, cy + 3, tbuf[tx + i], 0x00FFFFFF);
        i += 1;
    }
}


fn hit_tray(mx: i32, my: i32) -> Option<u8> {
    // 1=net 2=vol 3=display 4=clock — match draw_status_icons_and_clock
    let w = graphics::width() as i32;
    let h = graphics::height() as i32;
    if my < h - (TASKBAR_H as i32) {
        return None;
    }
    let clock_w = 5 * 8 + 8;
    let icon_slot = 20i32;
    let tray_w = clock_w as i32 + icon_slot * 3 + 8;
    let tray_x = if w > tray_w + 4 { w - tray_w - 4 } else { 4 };
    if mx >= tray_x && mx < tray_x + icon_slot {
        return Some(1);
    }
    if mx >= tray_x + icon_slot && mx < tray_x + icon_slot * 2 {
        return Some(2);
    }
    if mx >= tray_x + icon_slot * 2 && mx < tray_x + icon_slot * 3 {
        return Some(3);
    }
    if mx >= tray_x + icon_slot * 3 {
        return Some(4);
    }
    None
}


/// Drag frame: erase old window rect + redraw windows/panel — NO full framebuffer fill
fn render_drag_step() {
    unsafe {
        if !DRAGGING || DRAG_WIN >= MAX_WIN {
            return;
        }
        let w = graphics::width();
        let h = graphics::height();
        // 1) Erase UNION of old + new footprint (reduces trail flicker)
        let nx = WINS[DRAG_WIN].x;
        let ny = WINS[DRAG_WIN].y;
        let nw = WINS[DRAG_WIN].w;
        let nh = WINS[DRAG_WIN].h;
        let left = if DRAG_OLD_X < nx { DRAG_OLD_X } else { nx };
        let top = if DRAG_OLD_Y < ny { DRAG_OLD_Y } else { ny };
        let right = if DRAG_OLD_X + DRAG_OLD_W > nx + nw {
            DRAG_OLD_X + DRAG_OLD_W
        } else {
            nx + nw
        };
        let bottom = if DRAG_OLD_Y + DRAG_OLD_H > ny + nh {
            DRAG_OLD_Y + DRAG_OLD_H
        } else {
            ny + nh
        };
        let ox = if left < 0 { 0usize } else { left as usize };
        let oy = if top < 0 { 0usize } else { top as usize };
        let ow = if right - left < 0 { 0 } else { (right - left) as usize + 6 };
        let oh = if bottom - top < 0 { 0 } else { (bottom - top) as usize + 6 };
        graphics::fill_rect(ox, oy, ow, oh, COL_BG);
        // 2) If old rect hit panel/dock, restore strips
        if oy < 30 {
            graphics::fill_rect(0, 0, w, 28, COL_PANEL);
            graphics::fill_rect(0, 28, w, 2, COL_ACCENT);
            graphics::draw_str(12, 10, "Aether Desktop v1.1 XP", COL_TITLE);
        }
        let dock_y = h.saturating_sub(40);
        if oy + oh > dock_y {
            redraw_status_strip();
        }
        // 3) Redraw all windows in z-order (no full clear)
        let mut order = [0usize; MAX_WIN];
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible {
                order[n] = i;
                n += 1;
            }
            i += 1;
        }
        let mut a = 0usize;
        while a < n {
            let mut b = 0usize;
            while b + 1 < n {
                if WINS[order[b]].z > WINS[order[b + 1]].z {
                    let tmp = order[b];
                    order[b] = order[b + 1];
                    order[b + 1] = tmp;
                }
                b += 1;
            }
            a += 1;
        }
        i = 0;
        while i < n {
            draw_window(order[i]);
            i += 1;
        }
        // Update old rect to current for next step
        DRAG_OLD_X = WINS[DRAG_WIN].x;
        DRAG_OLD_Y = WINS[DRAG_WIN].y;
        DRAG_OLD_W = WINS[DRAG_WIN].w;
        DRAG_OLD_H = WINS[DRAG_WIN].h;
        CURSOR_SAVED = false;
        cursor_save_and_draw(MX, MY);
    }
}




fn hit_window(mx: i32, my: i32) -> Option<usize> {
    unsafe {
        let mut best: Option<usize> = None;
        let mut best_z = -1i32;
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible && !WINS[i].minimized {
                let w = &WINS[i];
                if mx >= w.x && mx < w.x + w.w as i32 && my >= w.y && my < w.y + w.h as i32 {
                    if w.z as i32 > best_z {
                        best_z = w.z as i32;
                        best = Some(i);
                    }
                }
            }
            i += 1;
        }
        best
    }
}

fn draw_ctx_menu() {
    unsafe {
        if !CTX_MENU { return; }
        let mx = CTX_X as usize;
        let my = CTX_Y as usize;
        let w = 160usize;
        let h = 120usize;
        let sw = graphics::width();
        let sh = graphics::height();
        let x = if mx + w > sw { sw.saturating_sub(w) } else { mx };
        let y = if my + h > sh.saturating_sub(30) { sh.saturating_sub(30 + h) } else { my };
        graphics::fill_rect(x + 2, y + 2, w, h, 0x00404040);
        graphics::fill_rect(x, y, w, h, 0x00FFFFFF);
        graphics::border_rect(x, y, w, h, 0x00808080);
        let items = ["Refresh", "New Text File", "Open Terminal", "Open Computer", "Properties"];
        let mut i = 0usize;
        while i < 5 {
            graphics::draw_str(x + 12, y + 10 + i * 20, items[i], 0x00000000);
            i += 1;
        }
    }
}

fn hit_ctx_menu(mx: i32, my: i32) -> Option<usize> {
    unsafe {
        if !CTX_MENU { return None; }
        let x0 = CTX_X;
        let y0 = CTX_Y;
        let w = 160i32;
        let h = 120i32;
        if mx < x0 || mx >= x0 + w || my < y0 || my >= y0 + h {
            return None;
        }
        let idx = ((my - y0 - 6) / 20) as usize;
        if idx < 5 { Some(idx) } else { None }
    }
}

fn draw_start_menu() {
    use crate::gui::icon::{self, IconId};
    let h = graphics::height();
    let tb = TASKBAR_H;
    let menu_h = 336usize;
    let menu_w = 220usize;
    let mx = 2usize;
    let my = h.saturating_sub(tb + menu_h);
    // XP two-column start menu shell
    graphics::fill_rect(mx + 2, my + 2, menu_w, menu_h, 0x00404040); // shadow
    graphics::fill_rect(mx, my, menu_w, menu_h, 0x00FFFFFF);
    // left green user strip
    graphics::fill_rect(mx, my, 4, menu_h, COL_START);
    graphics::fill_rect(mx, my, menu_w, 28, 0x00245EDC);
    graphics::draw_str(mx + 12, my + 10, "Aether User", 0x00FFFFFF);
    // items with icons
    let items: [(IconId, &str, usize); 9] = [
        (IconId::Terminal, "Terminal", 0),
        (IconId::Folder, "Files", 5),
        (IconId::MyComputer, "My Computer", 7),
        (IconId::Network, "Network", 2),
        (IconId::Settings, "Settings", 8),
        (IconId::File, "Media Player", 10),
        (IconId::MyDocuments, "Documents", 5),
        (IconId::File, "On-Screen Keyboard", 11),
        (IconId::RecycleBin, "Recycle Bin", 99),
    ];
    let mut i = 0usize;
    while i < 9 {
        let (id, name, _) = items[i];
        let iy = my + 36 + i * 28;
        icon::blit(id, mx + 10, iy, false);
        // scale: blit is 32px — draw smaller label only, clip by only showing label
        graphics::draw_str(mx + 48, iy + 10, name, 0x00000000);
        i += 1;
    }
    // bottom shutdown strip
    graphics::fill_rect(mx, my + menu_h - 28, menu_w, 28, 0x00D4D0C8);
    graphics::draw_str(mx + 16, my + menu_h - 18, "Turn Off Computer", 0x00000000);
}



fn hit_start_menu(mx: i32, my: i32) -> Option<usize> {
    let h = graphics::height() as i32;
    let tb = TASKBAR_H as i32;
    let menu_h = 336i32;
    let menu_w = 220i32;
    let x0 = 2i32;
    let y0 = h - tb - menu_h;
    if mx < x0 || mx >= x0 + menu_w || my < y0 || my >= y0 + menu_h {
        return None;
    }
    // map to action index 0..6
    let rel = my - (y0 + 36);
    if rel < 0 {
        return None;
    }
    let idx = (rel / 28) as usize;
    if idx < 9 {
        Some(idx)
    } else {
        None
    }
}

fn render() {
    let w = graphics::width();
    let h = graphics::height();
    if w == 0 || h == 0 {
        return;
    }
    // XP-style bliss-like sky/hills (procedural, not MS wallpaper)
    draw_xp_wallpaper(w, h);
    draw_desktop_icons();
    // XP blue taskbar bottom
    let tb = TASKBAR_H;
    graphics::fill_rect(0, h.saturating_sub(tb), w, tb, COL_TASKBAR);
    graphics::fill_rect(0, h.saturating_sub(tb), w, 2, COL_TASKBAR_TOP);
    let ty = h.saturating_sub(tb);
    draw_taskbar_buttons(w, h);
    draw_status_icons_and_clock(w, h);
    // Start button LAST so nothing covers it (critical for visibility)
    draw_start_button(w, h);
    // windows then start menu on top
    unsafe {
        let mut order = [0usize; MAX_WIN];
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible {
                order[n] = i;
                n += 1;
            }
            i += 1;
        }
        let mut a = 0usize;
        while a < n {
            let mut b = 0usize;
            while b + 1 < n {
                if WINS[order[b]].z > WINS[order[b + 1]].z {
                    let tmp = order[b];
                    order[b] = order[b + 1];
                    order[b + 1] = tmp;
                }
                b += 1;
            }
            a += 1;
        }
        i = 0;
        while i < n {
            draw_window(order[i]);
            i += 1;
        }
        if START_MENU {
            draw_start_menu();
        }
        if CTX_MENU {
            draw_ctx_menu();
        }
        CURSOR_SAVED = false;
        cursor_save_and_draw(MX, MY);
    }
}


fn handle_special_key(hid_code: u8) -> bool {
    unsafe {
        if FOCUS >= MAX_WIN || !WINS[FOCUS].visible {
            return false;
        }
        if WINS[FOCUS].kind == WinKind::Files {
            match hid_code {
                0x52 => { crate::files_mgr::on_nav_key(0x48) }
                0x51 => { crate::files_mgr::on_nav_key(0x50) }
                _ => false,
            }
        } else if WINS[FOCUS].kind == WinKind::Terminal {
            match hid_code {
                0x52 | 0x4B => { term_scroll_up(); true }
                0x51 | 0x4E => { term_scroll_down(); true }
                _ => false,
            }
        } else {
            false
        }
    }
}

fn handle_key(ch: u8) {
    unsafe {
        if ch == 0x1B {
            // Esc: let Explorer close its modal/context state before closing desktop menus.
            if FOCUS < MAX_WIN && WINS[FOCUS].visible && WINS[FOCUS].kind == WinKind::Files {
                if crate::files_mgr::on_escape() {
                    DIRTY_FULL = true;
                    return;
                }
            }
            START_MENU = false;
            CTX_MENU = false;
            DIRTY_FULL = true;
            return;
        }
        if FOCUS >= MAX_WIN || !WINS[FOCUS].visible || WINS[FOCUS].kind != WinKind::Terminal {
            return;
        }
        if ch == b'\n' {
            // Echo the exact byte buffer before dispatch so GUI command routing
            // is directly observable during hardware diagnostics.
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            terminal_write("aether> CMD-IN=[");
            let mut k = 0usize;
            while k < INPUT_LEN {
                term_putc(INPUT[k]);
                k += 1;
            }
            terminal_write("]\n");
            let output_start = TERM_ROW;
            crate::shell::run_command_from_gui(&INPUT, INPUT_LEN);
            INPUT_LEN = 0;
            term_page_begin(output_start);
            DIRTY_FULL = true;
        } else if ch == b' ' && INPUT_LEN == 0 && (TERM_PAGE_MODE || TERM_VIEW > 0) {
            term_page_next();
        } else if ch == 0x08 && INPUT_LEN == 0 && (TERM_PAGE_MODE || TERM_VIEW > 0) {
            term_page_prev();
        } else if ch == 0x08 {
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            if INPUT_LEN > 0 {
                INPUT_LEN -= 1;
                DIRTY_FULL = true;
            }
        } else if ch >= 32 && ch < 127 && INPUT_LEN < 63 {
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            INPUT[INPUT_LEN] = ch;
            INPUT_LEN += 1;
            DIRTY_FULL = true;
        }
    }
}

pub fn run() -> ! {
    unsafe {
        let sw = graphics::width() as i32;
        let sh = graphics::height() as i32;

        // Keep the original normal terminal geometry as the restore target.
        // The previous patch overwrote rx/ry/rw/rh with the maximized geometry,
        // so clicking Restore simply restored the same fullscreen rectangle.
        WINS[0].rx = 20;
        WINS[0].ry = 36;
        WINS[0].rw = 760;
        WINS[0].rh = 340;

        WINS[0].x = 0;
        WINS[0].y = 30;
        WINS[0].w = sw;
        WINS[0].h = sh - 70;
        WINS[0].maximized = true;
    }
    serial::write_str("\n======== Aether Desktop v1.1 XP ========\n");
    crate::drivers::audio::init();
    // Real HDA PCM startup smoke test: use the embedded PCM WAV, not the
    // legacy PC-speaker beep. Playback is serviced by playback_poll() below.
    let startup_wav_ok = crate::media_player::open_embedded_wav();
    if startup_wav_ok {
        crate::media_player::play();
    }
    serial::write_str("--- MOUSE DIAG ---\n");
    serial::write_str(if crate::drivers::ps2::diag_controller_ok() { "[PS2] controller OK\n" } else { "[PS2] controller FAIL\n" });
    serial::write_str(if crate::drivers::ps2::diag_ack_ok() { "[PS2] mouse ACK OK\n" } else { "[PS2] mouse ACK FAIL\n" });
    serial::write_str(if crate::drivers::ps2::diag_stream_ok() { "[PS2] streaming OK\n" } else { "[PS2] streaming FAIL\n" });
    serial::write_str("[PS2] packets=");
    serial::write_usize(crate::drivers::ps2::diag_packets() as usize);
    serial::write_str("\n");
    serial::write_str(if crate::drivers::xhci::diag_xhci_ok() { "[USB] xHCI OK\n" } else { "[USB] xHCI FAIL\n" });
    serial::write_str(if crate::drivers::xhci::diag_dev_found() || crate::drivers::xhci::diag_mouse_if() {
        "[USB] mouse FOUND\n"
    } else {
        "[USB] mouse NOT FOUND\n"
    });
    serial::write_str(if crate::drivers::xhci::diag_hid_ok() { "[USB] HID OK\n" } else { "[USB] HID FAIL\n" });
    serial::write_str("[USB] reports=");
    serial::write_usize(crate::drivers::xhci::diag_reports() as usize);
    serial::write_str("\n");
    term_clear();
    terminal_write("Aether Desktop v1.1 XP\n");
    terminal_write("AUTOSTART: PCI/USB diag on serial\n");
    terminal_write("Re-run: type 1  then plug mouse\n");
    terminal_write(if startup_wav_ok {
        "AUDIO: HDA startup WAV playback requested\n"
    } else {
        "AUDIO: HDA startup WAV open failed\n"
    });
    terminal_write("aether> ");
    unsafe {
        DIRTY_FULL = true;
        MX = (graphics::width() / 2) as i32;
        MY = (graphics::height() / 2) as i32;
    }
    render();

    loop {
        ps2::poll();
        let (pmx, pmy) = ps2::mouse_pos();
        let pbtn = ps2::mouse_buttons();
        unsafe {
            if pmx != MX || pmy != MY {
                MX = pmx;
                MY = pmy;
                let sw = graphics::width() as i32;
                let sh = graphics::height() as i32;
                if MX < 0 { MX = 0; }
                if MY < 0 { MY = 0; }
                if MX >= sw { MX = sw - 1; }
                if MY >= sh { MY = sh - 1; }
                if DRAGGING && (MB & 1) != 0 {
                    WINS[DRAG_WIN].x = MX - DRAG_OX;
                    WINS[DRAG_WIN].y = MY - DRAG_OY;
                    clamp_win(DRAG_WIN);
                    render_drag_step(); // dirty region, not full fill
                } else {
                    DIRTY_CURSOR = true;
                }
            }
            if pbtn != MB {
                handle_mouse_buttons(pbtn);
            }
        }

        while let Some(ev) = input::poll_mouse() {
            apply_mouse_delta(ev.dx as i32, ev.dy as i32);
            unsafe {
                if DRAGGING && (ev.buttons & 1) != 0 {
                    WINS[DRAG_WIN].x = MX - DRAG_OX;
                    WINS[DRAG_WIN].y = MY - DRAG_OY;
                    clamp_win(DRAG_WIN);
                }
            }
            handle_mouse_buttons(ev.buttons);
        }

        let sc = ps2::last_scancode();
        if sc != 0 {
            let ext = ps2::last_scancode_extended();
            if ext {
                let files_focused = unsafe {
                    FOCUS < MAX_WIN && WINS[FOCUS].visible && WINS[FOCUS].kind == WinKind::Files
                };
                if files_focused && (sc == 0x48 || sc == 0x50) {
                    crate::files_mgr::on_nav_key(sc);
                } else {
                    match sc {
                        0x48 | 0x49 => { term_scroll_up(); }
                        0x50 | 0x51 => { term_scroll_down(); }
                        _ => {
                            ps2::scancode_to_ascii(0xE0);
                            if let Some(ch) = ps2::scancode_to_ascii(sc) {
                                handle_key(ch);
                            }
                        }
                    }
                }
            } else if let Some(ch) = ps2::scancode_to_ascii(sc) {
                handle_key(ch);
            }
        }
        while let Some(ev) = input::poll() {
            if ev.pressed {
                if handle_special_key(ev.hid_code) {
                    continue;
                }
                if ev.key != 0 {
                    handle_key(ev.key);
                }
            }
        }

        unsafe {
            if WIFI_UI_SCAN_REQUESTED {
                WIFI_UI_SCAN_REQUESTED = false;
                if crate::drivers::wifi::alive_seen() && crate::drivers::wifi::command_queue_ready() {
                    WIFI_UI_STATUS = if crate::drivers::wifi::scan_24ghz() { 1 } else { 3 };
                } else {
                    WIFI_UI_STATUS = 3;
                }
                DIRTY_FULL = true;
            }
        }

        crate::drivers::xhci::poll_mouse_live();
        crate::drivers::audio::playback_poll();

        static mut CLOCK_TICK: u32 = 0;
        unsafe {
            CLOCK_TICK += 1;
            // Clock strip only when second changes — no full-screen fill
            if CLOCK_TICK >= 800 {
                CLOCK_TICK = 0;
                let (_y, _mo, _d, _h, _mi, s) = crate::time::rtc_read();
                if s != LAST_SEC {
                    LAST_SEC = s;
                    cursor_restore();
                    redraw_status_strip();
                    cursor_save_and_draw(MX, MY);
                }
            }
            if DIRTY_FULL {
                CURSOR_SAVED = false;
                render();
                DIRTY_FULL = false;
                DIRTY_CURSOR = false;
            } else if DIRTY_CURSOR {
                cursor_save_and_draw(MX, MY);
                DIRTY_CURSOR = false;
            }
        }
        let mut d = 0u32;
        while d < 200 {
            d += 1;
        }
    }
}