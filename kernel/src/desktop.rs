//! Aether Desktop v1.1 XP complete UI — Windows XP Luna visual style (homage)

use crate::graphics;
use crate::drivers::ps2;
use crate::fs;
use crate::serial;
use crate::input;
use crate::wallpaper;
use crate::mm;

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
const COL_TERM_PROMPT: u32 = 0x0000E676;
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

const MAX_WIN: usize = 16;
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
    Alarm,
    HelloExe,
    WinampExe,
    AIChat,
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
    Window { x: 145, y: 125, w: 430, h: 290, kind: WinKind::Video, visible: false, z: 4,
        minimized: false, maximized: false, rx: 145, ry: 125, rw: 430, rh: 290 },
    Window { x: 180, y: 100, w: 340, h: 260, kind: WinKind::Files, visible: false, z: 6,
        minimized: false, maximized: false, rx: 180, ry: 100, rw: 340, rh: 260 },
    Window { x: 280, y: 200, w: 240, h: 140, kind: WinKind::DateTime, visible: false, z: 7,
        minimized: false, maximized: false, rx: 280, ry: 200, rw: 240, rh: 140 },
    Window { x: 80, y: 40, w: 520, h: 360, kind: WinKind::MyComputer, visible: false, z: 8,
        minimized: false, maximized: false, rx: 80, ry: 40, rw: 520, rh: 360 },
    Window { x: 140, y: 50, w: 420, h: 360, kind: WinKind::SysProps, visible: false, z: 9,
        minimized: false, maximized: false, rx: 140, ry: 50, rw: 420, rh: 360 },
    Window { x: 120, y: 50, w: 500, h: 410, kind: WinKind::Settings, visible: false, z: 10,
        minimized: false, maximized: false, rx: 120, ry: 50, rw: 500, rh: 410 },
    Window { x: 70, y: 70, w: 660, h: 390, kind: WinKind::MediaPlayer, visible: false, z: 11,
        minimized: false, maximized: false, rx: 70, ry: 70, rw: 660, rh: 390 },
    Window { x: 525, y: 40, w: 230, h: 145, kind: WinKind::Keyboard, visible: false, z: 12,
        minimized: false, maximized: false, rx: 30, ry: 275, rw: 740, rh: 295 },
    Window { x: 120, y: 55, w: 470, h: 360, kind: WinKind::Alarm, visible: false, z: 13,
        minimized: false, maximized: false, rx: 120, ry: 55, rw: 470, rh: 360 },
    Window { x: 220, y: 120, w: 430, h: 250, kind: WinKind::HelloExe, visible: false, z: 14,
        minimized: false, maximized: false, rx: 220, ry: 120, rw: 430, rh: 250 },
    Window { x: 300, y: 120, w: 500, h: 300, kind: WinKind::WinampExe, visible: false, z: 15,
        minimized: false, maximized: false, rx: 300, ry: 120, rw: 500, rh: 300 },
    Window { x: 90, y: 70, w: 720, h: 420, kind: WinKind::AIChat, visible: true, z: 16,
        minimized: false, maximized: false, rx: 90, ry: 70, rw: 720, rh: 420 },
];

static mut FOCUS: usize = 0;
static mut VIRT_CRITICAL_ALERTED: bool = false; // terminal
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
// Display dialog state. Only modes with an implemented Aether hardware path are offered.
static mut VIDEO_PENDING_W: u16 = 800;
static mut VIDEO_PENDING_H: u16 = 600;
static mut VIDEO_STATUS: u8 = 0; // 0=idle, 1=applied, 2=failed
static mut CURSOR_ID: u8 = 0;
static mut CURSOR_PENDING: u8 = 0;

static mut MEDIA_OPEN_DIALOG: bool = false;
static mut MEDIA_DIR: [u8; 96] = [0; 96];
static mut MEDIA_DIR_LEN: usize = 0;
static mut KEYBOARD_TARGET: usize = 0;
static mut KEYBOARD_OPEN: bool = false;
static mut KEYBOARD_SHIFT: bool = false;
static mut KEYBOARD_CAPS: bool = false;
static mut KEYBOARD_CTRL: bool = false;
static mut KEYBOARD_ALT: bool = false;

// Native text-chat surface. This is an interface/transport endpoint, not a
// claim that a local language model is embedded in the kernel.
static mut AI_HISTORY: [[u8; 96]; 24] = [[0; 96]; 24];
static mut AI_HISTORY_LEN: [usize; 24] = [0; 24];
static mut AI_HISTORY_COUNT: usize = 0;
static mut AI_INPUT: [u8; 96] = [0; 96];
static mut AI_INPUT_LEN: usize = 0;
static mut AI_INPUT_CURSOR: usize = 0;
static mut AI_READY: bool = false;
static mut AI_BRIDGE_ACTIVE: bool = false;
static mut AI_WAITING: bool = false;
static mut AI_RX: [u8; 128] = [0; 128];
static mut AI_RX_LEN: usize = 0;
static mut AI_REQUEST_SEQ: u64 = 1;

fn ai_push_bytes(bytes: &[u8]) {
    unsafe {
        if AI_HISTORY_COUNT >= AI_HISTORY.len() {
            let mut r = 1usize;
            while r < AI_HISTORY.len() {
                AI_HISTORY[r - 1] = AI_HISTORY[r];
                AI_HISTORY_LEN[r - 1] = AI_HISTORY_LEN[r];
                r += 1;
            }
            AI_HISTORY_COUNT = AI_HISTORY.len() - 1;
        }
        let row = AI_HISTORY_COUNT;
        let n = bytes.len().min(AI_HISTORY[row].len());
        let mut i = 0usize;
        while i < n { AI_HISTORY[row][i] = bytes[i]; i += 1; }
        AI_HISTORY_LEN[row] = n;
        AI_HISTORY_COUNT += 1;
    }
}
fn ai_push(text: &str) { ai_push_bytes(text.as_bytes()); }

static mut AI_TERMINAL_TARGET: bool = false;

fn ai_buf_dec(buf: &mut [u8; 96], pos: &mut usize, mut v: usize) {
    let mut d = [0u8; 20];
    let mut n = 0usize;
    if v == 0 {
        if *pos < buf.len() { buf[*pos] = b'0'; *pos += 1; }
        return;
    }
    while v > 0 && n < d.len() {
        d[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    while n > 0 {
        n -= 1;
        if *pos < buf.len() { buf[*pos] = d[n]; *pos += 1; }
    }
}

fn ai_terminal_request(bytes: &[u8]) {
    let mut req = [0u8; 96];
    let mut n = 0usize;
    crate::virt_core::append_context(&mut req, &mut n);
    let pid = crate::process::current_pid();
    ai_buf_dec(&mut req, &mut n, pid);
    if n < 90 { req[n] = b' '; n += 1; }

    let mut i = 0usize;
    let tag = b"PROC=";
    i = 0;
    while i < tag.len() && n < 90 { req[n] = tag[i]; n += 1; i += 1; }
    let mut active = 0usize;
    let mut p = 1usize;
    while p <= crate::process::MAX_PROCESSES {
        if let Some(proc_) = crate::process::get(p) {
            active += 1;
            if n + 8 < 90 {
                if active == 1 {
                    req[n] = b'['; n += 1;
                } else {
                    req[n] = b','; n += 1;
                }
                ai_buf_dec(&mut req, &mut n, proc_.pid);
                req[n] = b':'; n += 1;
                let state = match proc_.state {
                    crate::process::State::Ready => b'R',
                    crate::process::State::Running => b'X',
                    crate::process::State::Exited => b'E',
                    crate::process::State::Empty => b'-',
                };
                req[n] = state; n += 1;
            }
        }
        p += 1;
    }
    if active > 0 && n < 90 { req[n] = b']'; n += 1; }
    if n < 90 { req[n] = b' '; n += 1; }

    let tag = b"RAM=";
    i = 0;
    while i < tag.len() && n < 90 { req[n] = tag[i]; n += 1; i += 1; }
    ai_buf_dec(&mut req, &mut n, mm::total_count() / 256);
    if n < 90 { req[n] = b'M'; n += 1; }
    if n < 90 { req[n] = b' '; n += 1; }

    let tag = b"FREE=";
    i = 0;
    while i < tag.len() && n < 90 { req[n] = tag[i]; n += 1; i += 1; }
    ai_buf_dec(&mut req, &mut n, mm::free_count() / 256);
    if n < 90 { req[n] = b'M'; n += 1; }
    if n < 90 { req[n] = b' '; n += 1; }

    let tag = b"FB=";
    i = 0;
    while i < tag.len() && n < 90 { req[n] = tag[i]; n += 1; i += 1; }
    ai_buf_dec(&mut req, &mut n, graphics::width());
    if n < 90 { req[n] = b'x'; n += 1; }
    ai_buf_dec(&mut req, &mut n, graphics::height());
    if n < 90 { req[n] = b' '; n += 1; }

    let room = 90usize.saturating_sub(n);
    let qn = bytes.len().min(room);
    i = 0;
    while i < qn {
        req[n + i] = if bytes[i] >= 32 && bytes[i] < 127 { bytes[i] } else { b' ' };
        i += 1;
    }
    n += qn;

    let request_id = unsafe {
        let id = AI_REQUEST_SEQ;
        AI_REQUEST_SEQ = AI_REQUEST_SEQ.wrapping_add(1);
        id
    };
    serial::ai_write_str("AI_REQ:REQ=U");
    serial::ai_write_usize(request_id as usize);
    serial::ai_write_str(" ");
    i = 0;
    while i < n { serial::ai_write_byte(req[i]); i += 1; }
    serial::ai_write_str("\n");
    terminal_write("VIRT: observing kernel state (read-only)...\n");
    unsafe {
        AI_WAITING = true;
        AI_BRIDGE_ACTIVE = true;
        AI_TERMINAL_TARGET = true;
    }
}

fn ai_terminal_command(bytes: &[u8]) -> bool {
    if bytes.len() < 4 { return false; }
    let mut i = 0usize;
    while i < 4 {
        let mut c = bytes[i];
        if c >= b'a' && c <= b'z' { c -= b'a' - b'A'; }
        if c != b"VIRT"[i] { return false; }
        i += 1;
    }
    bytes.len() == 4 || bytes[4] == b' '
}

fn ai_transport_request(bytes: &[u8]) {
    let n = bytes.len().min(90);
    let mut line = [0u8; 96];
    let prefix = b"YOU: ";
    let mut p = 0usize;
    while p < prefix.len() { line[p] = prefix[p]; p += 1; }
    let mut i = 0usize;
    while i < n { line[p + i] = bytes[i]; i += 1; }
    ai_push_bytes(&line[..p + n]);
    let request_id = unsafe {
        let id = AI_REQUEST_SEQ;
        AI_REQUEST_SEQ = AI_REQUEST_SEQ.wrapping_add(1);
        id
    };
    serial::ai_write_str("AI_REQ:REQ=U");
    serial::ai_write_usize(request_id as usize);
    serial::ai_write_str(" ");
    let mut j = 0usize;
    while j < n {
        let b = bytes[j];
        if b >= 32 && b < 127 && b != b'\r' && b != b'\n' {
            serial::ai_write_byte(b);
        } else {
            serial::ai_write_byte(b' ');
        }
        j += 1;
    }
    serial::ai_write_str("\n");
    unsafe { AI_WAITING = true; }
}

fn ai_init() {
    serial::init_ai();
    unsafe { if AI_READY { return; } AI_READY = true; }
    ai_push("AETHER AI: text interface online.");
    ai_push("BRIDGE: transport endpoint ready; waiting for external model.");
    ai_push("TYPE A MESSAGE AND PRESS ENTER.");
}

fn ai_submit() {
    unsafe {
        if AI_INPUT_LEN == 0 { return; }
        let input = AI_INPUT;
        let input_len = AI_INPUT_LEN;
        AI_INPUT_LEN = 0;
        AI_INPUT_CURSOR = 0;
        ai_transport_request(&input[..input_len]);
    }
}

fn ai_transport_poll() {
    let mut count = 0usize;
    while count < 64 {
        let b = match serial::ai_read_byte() {
            Some(v) => v,
            None => break,
        };
        count += 1;
        unsafe {
            if b == b'\r' || b == b'\n' {
                if AI_RX_LEN == 0 { continue; }
                let line = AI_RX;
                let len = AI_RX_LEN;
                AI_RX_LEN = 0;
                if len >= 7 && &line[..7] == b"AI_RES:" {
                    let body = &line[7..len];
                    let mut response_kind = 0u8;
                    let mut response_id = 0u64;
                    let mut body_start = 0usize;
                    if body.len() >= 6 && &body[..4] == b"REQ=" {
                        let source = body[4];
                        let mut p = 5usize;
                        while p < body.len() && body[p] >= b'0' && body[p] <= b'9' {
                            response_id = response_id.saturating_mul(10).saturating_add((body[p] - b'0') as u64);
                            p += 1;
                        }
                        if p < body.len() && body[p] == b':' {
                            body_start = p + 1;
                            response_kind = if source == b'R' { 1 } else if source == b'U' { 2 } else { 0 };
                        }
                    }
                    let body = &body[body_start..];
                    if response_kind == 1 {
                        let waiting = crate::virt_runtime::reasoning_waiting();
                        let expected = crate::virt_runtime::reasoning_request_id();
                        serial::ai_write_str("AI_RX:REQ=R");
                        serial::ai_write_usize(response_id as usize);
                        serial::ai_write_str(if waiting && response_id == expected { " ROUTE=ACCEPT\n" } else { " ROUTE=REJECT\n" });
                    }
                    if response_kind == 1
                        && crate::virt_runtime::reasoning_waiting()
                        && response_id == crate::virt_runtime::reasoning_request_id()
                    {
                        crate::virt_runtime::receive_reasoning_response(response_id, body);
                        serial::ai_write_str("AI_ACK:REQ=R");
                        serial::ai_write_usize(response_id as usize);
                        serial::ai_write_str(" CLASS=");
                        serial::ai_write_usize(crate::virt_runtime::last_reasoning_classification() as usize);
                        serial::ai_write_str("\n");
                        ai_push_bytes(body);
                    } else if response_kind == 2 && AI_TERMINAL_TARGET {
                        terminal_write("VIRT: ");
                        if let Ok(text) = core::str::from_utf8(body) {
                            terminal_write(text);
                        } else {
                            terminal_write("invalid UTF-8 response");
                        }
                        terminal_write("\n");
                        serial::ai_write_str("AI_ACK:REQ=U");
                        serial::ai_write_usize(response_id as usize);
                        serial::ai_write_str("\n");
                        AI_TERMINAL_TARGET = false;
                    } else {
                        ai_push_bytes(body);
                    }
                    AI_WAITING = false;
                    AI_BRIDGE_ACTIVE = true;
                } else if len >= 16 && &line[..16] == b"AI_STATUS:ACTIVE" {
                    AI_BRIDGE_ACTIVE = true;
                    ai_push("BRIDGE: external model active.");
                    // ACTIVE is the transport handshake. Repeat READY here so
                    // a bridge that connects after desktop startup cannot miss
                    // the one-time boot READY marker.
                    serial::ai_write_str("AI_STATUS:READY\n");
                } else if len >= 6 && &line[..6] == b"AI_IN:" {
                    let body = &line[6..len];
                    if !body.is_empty() { ai_transport_request(body); }
                }
            } else if AI_RX_LEN < AI_RX.len() {
                AI_RX[AI_RX_LEN] = b;
                AI_RX_LEN += 1;
            } else {
                AI_RX_LEN = 0;
            }
        }
    }
}

// Windows 7-style Wi-Fi UI state. The list is deliberately empty until the
// native Intel 2230 scan backend returns real 802.11 results.
static mut WIFI_UI_SCAN_REQUESTED: bool = false;
static mut WIFI_UI_SELECTED: i32 = -1;
static mut WIFI_UI_CONNECT_DIALOG: bool = false;
static mut WIFI_UI_PASSWORD_LEN: usize = 0;
static mut WIFI_UI_PASSWORD: [u8; 64] = [0; 64];
static mut WIFI_UI_STATUS: u8 = 0; // 0=idle, 1=scanning, 2=connected, 3=error

// Terminal buffer
const TERM_ROWS: usize = 2048;
const TERM_COLS: usize = 128;
const TERM_VIEW_ROWS_MAX: usize = 48; // safety cap; actual viewport follows terminal window height
static mut TERM_LINES: [[u8; TERM_COLS]; TERM_ROWS] = [[0; TERM_COLS]; TERM_ROWS];
static mut TERM_COLOR: [[u8; TERM_COLS]; TERM_ROWS] = [[7; TERM_COLS]; TERM_ROWS];
static mut TERM_LEN: [usize; TERM_ROWS] = [0; TERM_ROWS];
static mut TERM_ANSI_STATE: u8 = 0;
static mut TERM_ANSI_PARAMS: [u16; 8] = [0; 8];
static mut TERM_ANSI_PARAM_COUNT: usize = 0;
static mut TERM_ANSI_OVERFLOW: bool = false;
static mut TERM_ANSI_BASE: u8 = 7;
static mut TERM_ANSI_BOLD: bool = false;
static mut TERM_ANSI_CURRENT: u8 = 7;
static mut TERM_ROW: usize = 0;
static mut TERM_COL: usize = 0;
static mut TERM_SAVED_ROW: usize = 0;
static mut TERM_SAVED_COL: usize = 0;
static mut TERM_VIEW: usize = 0; // 0 = live bottom, larger = scrolled up
static mut TERM_SCROLL_DRAG: bool = false;
// Diagnostic output pager: long command output is shown page-by-page so each
// page fits the terminal and can be captured in one screenshot.
static mut TERM_PAGE_MODE: bool = false;
static mut TERM_SELECTING: bool = false;
static mut TERM_SEL_ANCHOR_ROW: usize = 0;
static mut TERM_SEL_ANCHOR_COL: usize = 0;
static mut TERM_SEL_ROW: usize = 0;
static mut TERM_SEL_COL: usize = 0;
static mut TERM_CLIPBOARD: [u8; 4096] = [0; 4096];
static mut TERM_CLIPBOARD_LEN: usize = 0;
static mut TERM_MENU: bool = false;
static mut TERM_MENU_X: i32 = 0;
static mut TERM_MENU_Y: i32 = 0;
static mut INPUT: [u8; 64] = [0; 64];
static mut INPUT_LEN: usize = 0;
static mut INPUT_CURSOR: usize = 0;
static mut TERM_HISTORY: [[u8; 64]; 16] = [[0; 64]; 16];
static mut TERM_HISTORY_LEN: [usize; 16] = [0; 16];
static mut TERM_HISTORY_COUNT: usize = 0;
static mut TERM_HISTORY_POS: usize = 0;
static mut TERM_SEARCH: [u8; 64] = [0; 64];
static mut TERM_SEARCH_LEN: usize = 0;
static mut TERM_SEARCH_HITS: [u16; 64] = [0; 64];
static mut TERM_SEARCH_HIT_COUNT: usize = 0;
static mut TERM_SEARCH_ACTIVE: bool = false;

fn term_selection_bounds() -> (usize, usize, usize, usize) {
    unsafe {
        if TERM_SEL_ANCHOR_ROW < TERM_SEL_ROW ||
           (TERM_SEL_ANCHOR_ROW == TERM_SEL_ROW && TERM_SEL_ANCHOR_COL <= TERM_SEL_COL) {
            (TERM_SEL_ANCHOR_ROW, TERM_SEL_ANCHOR_COL, TERM_SEL_ROW, TERM_SEL_COL)
        } else {
            (TERM_SEL_ROW, TERM_SEL_COL, TERM_SEL_ANCHOR_ROW, TERM_SEL_ANCHOR_COL)
        }
    }
}

fn term_selection_contains(row: usize, col: usize) -> bool {
    unsafe {
        if !TERM_SELECTING && TERM_SEL_ANCHOR_ROW == TERM_SEL_ROW &&
           TERM_SEL_ANCHOR_COL == TERM_SEL_COL {
            return false;
        }
        let (r0, c0, r1, c1) = term_selection_bounds();
        if row < r0 || row > r1 { return false; }
        if r0 == r1 {
            return col >= c0 && col <= c1;
        }
        if row == r0 { return col >= c0; }
        if row == r1 { return col <= c1; }
        true
    }
}

fn term_selection_clear() {
    unsafe {
        TERM_SELECTING = false;
        TERM_SEL_ANCHOR_ROW = 0;
        TERM_SEL_ANCHOR_COL = 0;
        TERM_SEL_ROW = 0;
        TERM_SEL_COL = 0;
        DIRTY_FULL = true;
    }
}

fn term_mouse_to_cell(mx: i32, my: i32) -> Option<(usize, usize)> {
    unsafe {
        let idx = match hit_window(mx, my) {
            Some(i) if WINS[i].kind == WinKind::Terminal => i,
            _ => return None,
        };
        let w = &WINS[idx];
        let body_top = w.y + TITLE_H + 34;
        let body_bottom = w.y + w.h - 24;
        let bar_x = w.x + w.w - 20;
        if mx < w.x + 5 || mx >= bar_x || my < body_top || my >= body_bottom {
            return None;
        }
        let total = TERM_ROW + 1;
        let view_rows = term_visible_rows();
        let max_start = if total > view_rows { total - view_rows } else { 0 };
        let mut view = TERM_VIEW;
        if view > max_start { view = max_start; }
        let start = max_start - view;
        let row = start + ((my - body_top) as usize / 10);
        if row >= total { return None; }
        let mut col = ((mx - (w.x + 10)) as usize) / 8;
        let cols = term_text_cols();
        if col >= cols { col = cols.saturating_sub(1); }
        Some((row, col))
    }
}

fn term_selection_begin(mx: i32, my: i32) -> bool {
    if let Some((row, col)) = term_mouse_to_cell(mx, my) {
        unsafe {
            TERM_SELECTING = true;
            TERM_SEL_ANCHOR_ROW = row;
            TERM_SEL_ANCHOR_COL = col;
            TERM_SEL_ROW = row;
            TERM_SEL_COL = col;
            TERM_MENU = false;
            DIRTY_FULL = true;
        }
        true
    } else {
        false
    }
}

fn term_selection_update(mx: i32, my: i32) {
    if let Some((row, col)) = term_mouse_to_cell(mx, my) {
        unsafe {
            TERM_SEL_ROW = row;
            TERM_SEL_COL = col;
            DIRTY_FULL = true;
        }
    }
}

fn term_copy_selection() {
    unsafe {
        let (r0, c0, r1, c1) = term_selection_bounds();
        if r0 == r1 && c0 == c1 {
            return;
        }
        let mut out = 0usize;
        let mut r = r0;
        while r <= r1 && r < TERM_ROWS {
            let len = TERM_LEN[r];
            let start = if r == r0 { c0.min(len) } else { 0 };
            let end = if r == r1 { c1.min(len.saturating_sub(1)) + 1 } else { len };
            let mut c = start;
            while c < end && out < TERM_CLIPBOARD.len() {
                TERM_CLIPBOARD[out] = TERM_LINES[r][c];
                out += 1;
                c += 1;
            }
            if r < r1 && out < TERM_CLIPBOARD.len() {
                TERM_CLIPBOARD[out] = b'\n';
                out += 1;
            }
            if r == r1 { break; }
            r += 1;
        }
        TERM_CLIPBOARD_LEN = out;
        TERM_MENU = false;
        DIRTY_FULL = true;
    }
}

fn term_paste_clipboard() {
    unsafe {
        if TERM_CLIPBOARD_LEN == 0 || FOCUS >= MAX_WIN ||
           WINS[FOCUS].kind != WinKind::Terminal {
            return;
        }
        let mut i = 0usize;
        while i < TERM_CLIPBOARD_LEN && INPUT_LEN < 63 {
            let mut ch = TERM_CLIPBOARD[i];
            if ch == b'\n' || ch == b'\r' || ch == b'\t' { ch = b' '; }
            if ch < 32 || ch >= 127 {
                i += 1;
                continue;
            }
            let mut p = INPUT_LEN;
            while p > INPUT_CURSOR {
                INPUT[p] = INPUT[p - 1];
                p -= 1;
            }
            INPUT[INPUT_CURSOR] = ch;
            INPUT_LEN += 1;
            INPUT_CURSOR += 1;
            i += 1;
        }
        TERM_MENU = false;
        TERM_PAGE_MODE = false;
        TERM_VIEW = 0;
        DIRTY_FULL = true;
    }
}

fn term_menu_hit(mx: i32, my: i32) -> Option<usize> {
    unsafe {
        if !TERM_MENU { return None; }
        let x = TERM_MENU_X;
        let y = TERM_MENU_Y;
        let w = 150i32;
        let h = 72i32;
        if mx < x || mx >= x + w || my < y || my >= y + h {
            return None;
        }
        let idx = ((my - y) / 24) as usize;
        if idx < 3 { Some(idx) } else { None }
    }
}

fn draw_term_menu() {
    unsafe {
        if !TERM_MENU { return; }
        let sw = graphics::width() as i32;
        let sh = graphics::height() as i32;
        let w = 150i32;
        let h = 72i32;
        let x = if TERM_MENU_X + w > sw { sw - w } else { TERM_MENU_X };
        let y = if TERM_MENU_Y + h > sh - 30 { sh - 30 - h } else { TERM_MENU_Y };
        graphics::fill_rect((x + 2) as usize, (y + 2) as usize, w as usize, h as usize, 0x00404040);
        graphics::fill_rect(x as usize, y as usize, w as usize, h as usize, 0x00FFFFFF);
        graphics::border_rect(x as usize, y as usize, w as usize, h as usize, 0x00707070);
        graphics::draw_str(x as usize + 12, y as usize + 7, "Copy", COL_TEXT);
        graphics::draw_str(x as usize + 12, y as usize + 31, "Paste", COL_TEXT);
        graphics::draw_str(x as usize + 12, y as usize + 55, "Clear Selection", COL_TEXT);
    }
}

fn term_clear() {
    unsafe {
        let mut r = 0usize;
        while r < TERM_ROWS {
            TERM_LEN[r] = 0;
            let mut c = 0usize;
            while c < TERM_COLS {
                TERM_LINES[r][c] = 0;
                TERM_COLOR[r][c] = 7;
                c += 1;
            }
            r += 1;
        }
        TERM_ROW = 0;
        TERM_COL = 0;
        TERM_SAVED_ROW = 0;
        TERM_SAVED_COL = 0;
        TERM_SEARCH_LEN = 0;
        TERM_SEARCH_HIT_COUNT = 0;
        TERM_SEARCH_ACTIVE = false;
        TERM_VIEW = 0;
        TERM_PAGE_MODE = false;
        TERM_SELECTING = false;
        TERM_MENU = false;
        TERM_CLIPBOARD_LEN = 0;
        TERM_ANSI_STATE = 0;
        TERM_ANSI_PARAM_COUNT = 0;
        TERM_ANSI_PARAMS = [0; 8];
        TERM_ANSI_CURRENT = 7;
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

fn terminal_keyboard_height(w: i32) -> usize {
    if w >= 1100 { 188 } else if w >= 700 { 168 } else { 150 }
}

fn term_visible_rows() -> usize {
    unsafe {
        if FOCUS < MAX_WIN && WINS[FOCUS].kind == WinKind::Terminal && WINS[FOCUS].visible {
            let keyboard_h = if KEYBOARD_OPEN { terminal_keyboard_height(WINS[FOCUS].w) as i32 } else { 0 };
            let usable = if WINS[FOCUS].h > TITLE_H + 31 + 22 + keyboard_h {
                (WINS[FOCUS].h - TITLE_H - 31 - 22 - keyboard_h) as usize
            } else {
                1
            };
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

fn term_history_save() {
    unsafe {
        if INPUT_LEN == 0 { return; }
        let mut same = false;
        if TERM_HISTORY_COUNT > 0 {
            let last = if TERM_HISTORY_COUNT < 16 { TERM_HISTORY_COUNT - 1 } else { 15 };
            if TERM_HISTORY_LEN[last] == INPUT_LEN {
                same = true;
                let mut i = 0usize;
                while i < INPUT_LEN {
                    if TERM_HISTORY[last][i] != INPUT[i] { same = false; break; }
                    i += 1;
                }
            }
        }
        if same { TERM_HISTORY_POS = TERM_HISTORY_COUNT; return; }
        let slot = if TERM_HISTORY_COUNT < 16 {
            let s = TERM_HISTORY_COUNT;
            TERM_HISTORY_COUNT += 1;
            s
        } else {
            let mut r = 1usize;
            while r < 16 {
                TERM_HISTORY[r - 1] = TERM_HISTORY[r];
                TERM_HISTORY_LEN[r - 1] = TERM_HISTORY_LEN[r];
                r += 1;
            }
            15
        };
        let mut i = 0usize;
        while i < 64 {
            TERM_HISTORY[slot][i] = 0;
            i += 1;
        }
        i = 0;
        while i < INPUT_LEN {
            TERM_HISTORY[slot][i] = INPUT[i];
            i += 1;
        }
        TERM_HISTORY_LEN[slot] = INPUT_LEN;
        TERM_HISTORY_POS = TERM_HISTORY_COUNT;
    }
}

fn term_history_load(pos: usize) {
    unsafe {
        if pos >= TERM_HISTORY_COUNT { return; }
        INPUT_LEN = TERM_HISTORY_LEN[pos];
        INPUT_CURSOR = INPUT_LEN;
        let mut i = 0usize;
        while i < INPUT_LEN {
            INPUT[i] = TERM_HISTORY[pos][i];
            i += 1;
        }
        while i < 64 {
            INPUT[i] = 0;
            i += 1;
        }
        TERM_HISTORY_POS = pos;
        TERM_PAGE_MODE = false;
        TERM_VIEW = 0;
        DIRTY_FULL = true;
    }
}

fn term_history_prev() {
    unsafe {
        if TERM_HISTORY_COUNT == 0 { return; }
        if TERM_HISTORY_POS > TERM_HISTORY_COUNT {
            TERM_HISTORY_POS = TERM_HISTORY_COUNT;
        }
        if TERM_HISTORY_POS > 0 {
            TERM_HISTORY_POS -= 1;
            term_history_load(TERM_HISTORY_POS);
        }
    }
}

fn term_history_next() {
    unsafe {
        if TERM_HISTORY_COUNT == 0 { return; }
        if TERM_HISTORY_POS + 1 < TERM_HISTORY_COUNT {
            TERM_HISTORY_POS += 1;
            term_history_load(TERM_HISTORY_POS);
        } else {
            TERM_HISTORY_POS = TERM_HISTORY_COUNT;
            INPUT_LEN = 0;
            INPUT_CURSOR = 0;
            let mut i = 0usize;
            while i < 64 {
                INPUT[i] = 0;
                i += 1;
            }
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            DIRTY_FULL = true;
        }
    }
}

pub fn term_search_set(query:&[u8],len:usize){
    unsafe{
        TERM_SEARCH_LEN=len.min(TERM_SEARCH.len());let mut i=0;while i<TERM_SEARCH_LEN{TERM_SEARCH[i]=query[i];i+=1;}
        TERM_SEARCH_HIT_COUNT=0;TERM_SEARCH_ACTIVE=TERM_SEARCH_LEN>0;if !TERM_SEARCH_ACTIVE{DIRTY_FULL=true;return;}
        let mut r=0;while r<=TERM_ROW&&r<TERM_ROWS&&TERM_SEARCH_HIT_COUNT<TERM_SEARCH_HITS.len(){let llen=TERM_LEN[r];if llen>=TERM_SEARCH_LEN{let mut c=0;while c+TERM_SEARCH_LEN<=llen&&TERM_SEARCH_HIT_COUNT<TERM_SEARCH_HITS.len(){let mut ok=true;let mut j=0;while j<TERM_SEARCH_LEN{if TERM_LINES[r][c+j].to_ascii_lowercase()!=TERM_SEARCH[j].to_ascii_lowercase(){ok=false;break;}j+=1;}if ok{TERM_SEARCH_HITS[TERM_SEARCH_HIT_COUNT]=r as u16;TERM_SEARCH_HIT_COUNT+=1;}c+=1;}}r+=1;}
        DIRTY_FULL=true;
    }
}
fn term_search_contains(row:usize,col:usize)->bool{
    unsafe{if !TERM_SEARCH_ACTIVE||TERM_SEARCH_LEN==0{return false;}let mut c=0;while c+TERM_SEARCH_LEN<=TERM_LEN[row]{let mut ok=true;let mut j=0;while j<TERM_SEARCH_LEN{if TERM_LINES[row][c+j].to_ascii_lowercase()!=TERM_SEARCH[j].to_ascii_lowercase(){ok=false;break;}j+=1;}if ok&&col>=c&&col<c+TERM_SEARCH_LEN{return true;}c+=1;}false}
}

pub fn terminal_clear() {
    term_clear();
    unsafe {
        INPUT_LEN = 0;
        INPUT_CURSOR = 0;
        TERM_PAGE_MODE = false;
        TERM_VIEW = 0;
        DIRTY_FULL = true;
    }
}

fn term_ansi_color(code: u8) -> u32 {
    match code {
        0 => 0x00000000, 1 => 0x00E81123, 2 => 0x00107C10, 3 => 0x00FFB900,
        4 => 0x000078D7, 5 => 0x005C2D91, 6 => 0x0000B7C3, 7 => 0x00C0C0C0,
        8 => 0x00808080, 9 => 0x00FF5C5C, 10 => 0x0066D966, 11 => 0x00FFD966,
        12 => 0x005CA8FF, 13 => 0x00B07CFF, 14 => 0x005CE1E6, 15 => 0x00FFFFFF,
        _ => 0x00C0C0C0,
    }
}

fn term_ansi_apply(final_byte: u8) {
    unsafe {
        let p = |n: usize, default: u16| -> u16 {
            if TERM_ANSI_PARAM_COUNT > n { TERM_ANSI_PARAMS[n] } else { default }
        };
        let p0 = p(0, 0);
        match final_byte {
            b'm' => {
                if TERM_ANSI_PARAM_COUNT == 0 {
                    TERM_ANSI_BASE = 7;
                    TERM_ANSI_BOLD = false;
                } else {
                    let mut i = 0usize;
                    while i < TERM_ANSI_PARAM_COUNT {
                        let v = TERM_ANSI_PARAMS[i] as u8;
                        if v == 38 || v == 48 {
                            if i + 1 < TERM_ANSI_PARAM_COUNT {
                                if TERM_ANSI_PARAMS[i + 1] == 2 { i = i.saturating_add(5); }
                                else { i = i.saturating_add(3); }
                            } else { i += 1; }
                            continue;
                        }
                        match v {
                            0 => { TERM_ANSI_BASE = 7; TERM_ANSI_BOLD = false; }
                            1 => TERM_ANSI_BOLD = true,
                            22 => TERM_ANSI_BOLD = false,
                            30..=37 => TERM_ANSI_BASE = v - 30,
                            39 => TERM_ANSI_BASE = 7,
                            90..=97 => TERM_ANSI_BASE = v - 90 + 8,
                            _ => {}
                        }
                        i += 1;
                    }
                }
                TERM_ANSI_CURRENT = if TERM_ANSI_BOLD && TERM_ANSI_BASE < 8 {
                    TERM_ANSI_BASE + 8
                } else { TERM_ANSI_BASE };
            }
            b'A' => { TERM_ROW = TERM_ROW.saturating_sub(p0.max(1) as usize); }
            b'B' => { TERM_ROW = (TERM_ROW + p0.max(1) as usize).min(TERM_ROWS - 1); }
            b'C' => { TERM_COL = (TERM_COL + p0.max(1) as usize).min(TERM_COLS - 1); }
            b'D' => { TERM_COL = TERM_COL.saturating_sub(p0.max(1) as usize); }
            b'H' | b'f' => {
                let row = p(0, 1).max(1) as usize;
                let col = p(1, 1).max(1) as usize;
                TERM_ROW = (row - 1).min(TERM_ROWS - 1);
                TERM_COL = (col - 1).min(TERM_COLS - 1);
            }
            b's' => { TERM_SAVED_ROW = TERM_ROW; TERM_SAVED_COL = TERM_COL; }
            b'u' => {
                TERM_ROW = TERM_SAVED_ROW.min(TERM_ROWS - 1);
                TERM_COL = TERM_SAVED_COL.min(TERM_COLS - 1);
            }
            b'J' => {
                if p0 == 2 {
                    let mut r = 0;
                    while r < TERM_ROWS { TERM_LEN[r] = 0; r += 1; }
                    TERM_ROW = 0; TERM_COL = 0; TERM_VIEW = 0;
                } else if p0 == 0 {
                    let mut c = TERM_COL;
                    while c < TERM_LEN[TERM_ROW] {
                        TERM_LINES[TERM_ROW][c] = 0;
                        TERM_COLOR[TERM_ROW][c] = 7;
                        c += 1;
                    }
                    TERM_LEN[TERM_ROW] = TERM_COL.min(TERM_COLS);
                }
            }
            b'K' => {
                // K/0K are intentionally no-op at the terminal's line-end
                // cursor position; explicit 1K/2K are the erase-line forms.
                if p0 == 1 || p0 == 2 {
                    TERM_LEN[TERM_ROW] = 0;
                    TERM_COL = 0;
                }
            }
            _ => {}
        }
        TERM_ANSI_STATE = 0;
        TERM_ANSI_PARAM_COUNT = 0;
        TERM_ANSI_OVERFLOW = false;
        TERM_ANSI_PARAMS = [0; 8];
    }
}

fn term_ansi_byte(ch: u8) {
    unsafe {
        // Restart on a new ESC even if the previous CSI is incomplete.
        if ch == 0x1B {
            TERM_ANSI_STATE = 1;
            TERM_ANSI_PARAM_COUNT = 0;
            TERM_ANSI_OVERFLOW = false;
            TERM_ANSI_PARAMS = [0; 8];
            return;
        }
        if TERM_ANSI_STATE == 1 {
            if ch == b'[' {
                TERM_ANSI_STATE = 2;
                TERM_ANSI_PARAM_COUNT = 0;
                TERM_ANSI_OVERFLOW = false;
                TERM_ANSI_PARAMS = [0; 8];
            } else { TERM_ANSI_STATE = 0; }
            return;
        }
        if TERM_ANSI_STATE == 2 {
            if ch >= b'0' && ch <= b'9' {
                if !TERM_ANSI_OVERFLOW {
                    if TERM_ANSI_PARAM_COUNT == 0 { TERM_ANSI_PARAM_COUNT = 1; }
                    let n = TERM_ANSI_PARAM_COUNT - 1;
                    TERM_ANSI_PARAMS[n] = TERM_ANSI_PARAMS[n].saturating_mul(10)
                        .saturating_add((ch - b'0') as u16);
                }
                return;
            }
            if ch == b';' {
                if TERM_ANSI_PARAM_COUNT < 8 && !TERM_ANSI_OVERFLOW {
                    TERM_ANSI_PARAM_COUNT += 1;
                } else {
                    TERM_ANSI_OVERFLOW = true;
                }
                return;
            }
            // CSI parameter/private/intermediate bytes (20..3F) are consumed.
            if ch >= 0x20 && ch <= 0x3F { return; }
            if ch >= 0x40 && ch <= 0x7E {
                term_ansi_apply(ch);
                return;
            }
            TERM_ANSI_STATE = 0;
            TERM_ANSI_PARAM_COUNT = 0;
            TERM_ANSI_OVERFLOW = false;
            return;
        }
        term_putc(ch);
    }
}

fn term_newline() {
    unsafe {
        TERM_COL=0;
        if TERM_ROW+1<TERM_ROWS {TERM_ROW+=1;TERM_LEN[TERM_ROW]=0;} else {let mut r=0;while r+1<TERM_ROWS{let mut c=0;while c<TERM_COLS{TERM_LINES[r][c]=TERM_LINES[r+1][c];TERM_COLOR[r][c]=TERM_COLOR[r+1][c];c+=1;}TERM_LEN[r]=TERM_LEN[r+1];r+=1;}TERM_LEN[TERM_ROWS-1]=0;let mut c=0;while c<TERM_COLS{TERM_LINES[TERM_ROWS-1][c]=0;TERM_COLOR[TERM_ROWS-1][c]=TERM_ANSI_CURRENT;c+=1;}}
    }
}
fn term_putc(ch:u8) {
    unsafe {
        if ch==b'\n'{term_newline();return;} if ch==b'\r'{TERM_COL=0;return;} if ch==b'\t'{TERM_COL=(((TERM_COL/8)+1)*8).min(TERM_COLS-1);return;}
        let cols=term_text_cols().min(TERM_COLS); if TERM_COL>=cols{term_newline();}
        if TERM_COL<TERM_COLS{let c=TERM_COL;TERM_LINES[TERM_ROW][c]=ch;TERM_COLOR[TERM_ROW][c]=TERM_ANSI_CURRENT;if TERM_LEN[TERM_ROW]<=c{TERM_LEN[TERM_ROW]=c+1;}TERM_COL=c+1;}
    }
}

pub fn terminal_write_usize(mut v: usize) {
    if v == 0 {
        term_putc(b'0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut n = 0usize;
    while v > 0 && n < digits.len() {
        digits[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    while n > 0 {
        n -= 1;
        term_putc(digits[n]);
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
        term_ansi_byte(b);
    }
    unsafe {
        DIRTY_FULL = true;
        TERM_VIEW = 0; // new output follows live bottom
    }
}
/// Single-character output bridge for the unified kernel command executor.
pub fn terminal_write_char(ch: u8) {
    term_ansi_byte(ch);
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
        // Clicking the already-focused front window does not change composition.
        // Avoid a full framebuffer repaint for an ordinary click.
        if FOCUS == idx && WINS[idx].z == Z_TOP {
            return;
        }
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
        BACKGROUND_DRAWN = false;
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
        BACKGROUND_DRAWN = false;
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
        WinKind::Alarm => "Alarm Clock",
        WinKind::HelloExe => "Hello.exe",
        WinKind::WinampExe => "Winamp.exe",
        WinKind::AIChat => "Aether AI",
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
        if WINS[slot].kind == WinKind::Video {
            VIDEO_PENDING_W = graphics::width() as u16;
            VIDEO_PENDING_H = graphics::height() as u16;
            VIDEO_STATUS = 0;
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
    let items: [(IconId, usize); 9] = [
        (IconId::MyComputer, 7),
        (IconId::MyDocuments, 5),
        (IconId::Terminal, 0),
        (IconId::Network, 2),
        (IconId::Settings, 4),
        (IconId::RecycleBin, 99),
        (IconId::File, 10),
        (IconId::File, 13), // first Windows PE target
        (IconId::File, 14), // real third-party Winamp target
    ];
    let mut i = 0usize;
    while i < 9 {
        let (id, _slot) = items[i];
        let x = if i >= 7 { 104usize + (i - 7) * 80 } else { 24usize };
        let y = if i >= 7 { 36usize } else { 36 + i * 72 };
        let sel = unsafe { SELECTED_ICON == i };
        icon::blit(id, x, y, sel);
        let lab = if i == 7 { "Hello.exe" } else if i == 8 { "Winamp.exe" } else { icon::label(id) };
        let tw = lab.len() * 8;
        let lx = if tw < 48 { x + (48 - tw) / 2 } else { x };
        font::draw_shadowed(lx, y + theme::ICON_SIZE + 4, lab, 0x00FFFFFF, 0x00404040);
        i += 1;
    }
}


fn hit_desktop_icon(mx: i32, my: i32) -> Option<usize> {
    // Must match draw_desktop_icons. Hello.exe occupies a second column so
    // it cannot overlap the taskbar on the 600px AH532 desktop.
    if mx >= 96 && mx < 160 && my >= 36 && my < 100 {
        return Some(7);
    }
    if mx >= 176 && mx < 240 && my >= 36 && my < 100 {
        return Some(8);
    }
    let mut i = 0usize;
    while i < 7 {
        let y = 36 + (i as i32) * 72;
        if mx >= 16 && mx < 80 && my >= y && my < y + 64 {
            return Some(i);
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
                    WinKind::Alarm => "Alarm",
                    WinKind::HelloExe => "Hello",
                    WinKind::WinampExe => "Winamp",
                    WinKind::AIChat => "AI",
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
        if FOCUS < MAX_WIN && WINS[FOCUS].visible && WINS[FOCUS].kind == WinKind::Alarm {
            let _ = crate::alarm::alarm_mouse_move(
                MX - WINS[FOCUS].x - 3,
                MY - WINS[FOCUS].y - TITLE_H,
            );
        }
        // Mouse movement must not trigger a full framebuffer repaint while the
        // terminal is focused. A moving cursor is a cursor-only change; the
        // previous full repaint here could coincide with keyboard input and
        // make every typed character visibly flash.
        if FOCUS < MAX_WIN && WINS[FOCUS].visible && WINS[FOCUS].kind == WinKind::Terminal {
            DIRTY_CURSOR = true;
        } else {
            DIRTY_FULL = true;
        }
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
                                    crate::media_player::play();
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
        (96,false)=>96,(96,true)=>126,
        (45,false)=>45,(45,true)=>95,(61,false)=>61,(61,true)=>43,
        (91,false)=>91,(91,true)=>123,(93,false)=>93,(93,true)=>125,
        (59,false)=>59,(59,true)=>58,(39,false)=>39,(39,true)=>34,
        (44,false)=>44,(44,true)=>60,(46,false)=>46,(46,true)=>62,
        (47,false)=>47,(47,true)=>63,(92,false)=>92,(92,true)=>124,_=>c
    })
}

fn keyboard_emit(label:&str){
    unsafe{
        let target=KEYBOARD_TARGET;
        if target>=MAX_WIN||!WINS[target].visible||
           (WINS[target].kind!=WinKind::Terminal && WINS[target].kind!=WinKind::AIChat){return;}
        if label=="SHIFT"{KEYBOARD_SHIFT=!KEYBOARD_SHIFT;DIRTY_FULL=true;return;}
        if label=="CAPS"{KEYBOARD_CAPS=!KEYBOARD_CAPS;DIRTY_FULL=true;return;}
        if label=="CTRL"{KEYBOARD_CTRL=!KEYBOARD_CTRL;DIRTY_FULL=true;return;}
        if label=="ALT"{KEYBOARD_ALT=!KEYBOARD_ALT;DIRTY_FULL=true;return;}
        if label=="META"{return;}
        let old=FOCUS;FOCUS=target;
        if label=="GRAVE" {
            handle_key(if KEYBOARD_SHIFT { 126 } else { 96 });
            if KEYBOARD_SHIFT { KEYBOARD_SHIFT=false; }
        } else if label=="ENTER"{handle_key(b'\n');}
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
        }else if let Some(mut ch)=keyboard_key_to_ascii(label,KEYBOARD_SHIFT,KEYBOARD_CAPS){
            if KEYBOARD_CTRL && ch>=97 && ch<=122 { ch = ch - 96; }
            handle_key(ch);
            if KEYBOARD_SHIFT{KEYBOARD_SHIFT=false;}
        }
        DIRTY_FULL=true;
        FOCUS=old;
    }
}

fn handle_terminal_keyboard_button(mx:i32,my:i32)->bool{
    unsafe{
        if FOCUS>=MAX_WIN||!WINS[FOCUS].visible||
           (WINS[FOCUS].kind!=WinKind::Terminal && WINS[FOCUS].kind!=WinKind::AIChat){return false;}
        let w=&WINS[FOCUS];
        let bx=w.x+w.w-84;
        let by=w.y+TITLE_H+4;
        if mx>=bx&&mx<bx+30&&my>=by&&my<by+20{
            KEYBOARD_OPEN=!KEYBOARD_OPEN;
            KEYBOARD_TARGET=FOCUS;
            KEYBOARD_SHIFT=false;
            KEYBOARD_CTRL=false;
            KEYBOARD_ALT=false;
            DIRTY_FULL=true;
            return true;
        }
        false
    }
}

fn draw_terminal_keyboard(idx:usize){
    unsafe{
        if !KEYBOARD_OPEN||idx>=MAX_WIN||!WINS[idx].visible||
           (WINS[idx].kind!=WinKind::Terminal && WINS[idx].kind!=WinKind::AIChat){return;}
        let w=&WINS[idx];
        let wx=w.x as usize;
        let wy=w.y as usize;
        let ww=w.w.max(220) as usize;
        let wh=w.h as usize;
        let kh=terminal_keyboard_height(w.w);
        let panel_y=wy+wh.saturating_sub(kh);
        graphics::fill_rect(wx+4,panel_y,ww.saturating_sub(8),kh.saturating_sub(4),0x0021262D);
        graphics::border_rect(wx+4,panel_y,ww.saturating_sub(8),kh.saturating_sub(4),0x005A6670);
        graphics::draw_str(wx+12,panel_y+7,"On-screen keyboard",0x00D7E3EA);
        let rows:[&[&str];6]=[
            &["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12"],
            &["GRAVE","1","2","3","4","5","6","7","8","9","0","-","=","BACK"],
            &["TAB","q","w","e","r","t","y","u","i","o","p","[","]"],
            &["CAPS","a","s","d","f","g","h","j","k","l",";","'","ENTER"],
            &["SHIFT","z","x","c","v","b","n","m",",",".","/","SHIFT","UP"],
            &["CTRL","ALT","META","SPACE","LEFT","DOWN","RIGHT","HOME","END","DEL"]
        ];
        let row_h=((kh.saturating_sub(32))/6).max(18);
        let gap=3usize;
        let inner_w=ww.saturating_sub(20);
        let unit=(inner_w.saturating_sub(14*gap))/15;
        let mut r=0usize;
        while r<rows.len(){
            let mut x=wx+10;
            let y=panel_y+26+r*row_h;
            let mut c=0usize;
            while c<rows[r].len(){
                let label=rows[r][c];
                let units=if r==1&&c==13{2}else if r==2&&c==0{2}else if r==3&&c==0{2}else if r==3&&c==12{2}else if r==4&&(c==0||c==11){2}else if r==5&&c==3{4}else{1};
                let kw=unit*units+gap*(units-1);
                let active=(label=="SHIFT"&&KEYBOARD_SHIFT)||(label=="CAPS"&&KEYBOARD_CAPS)||(label=="CTRL"&&KEYBOARD_CTRL)||(label=="ALT"&&KEYBOARD_ALT);
                graphics::fill_rect(x,y,kw,row_h.saturating_sub(3),if active{0x003E9CCB}else{0x00343B43});
                graphics::border_rect(x,y,kw,row_h.saturating_sub(3),0x007B8790);
                let short=match label{
                    "BACK"=>"BACK","ENTER"=>"ENTER","SHIFT"=>"SHIFT","CAPS"=>"CAPS",
                    "SPACE"=>"SPACE","CTRL"=>"CTRL","ALT"=>"ALT","META"=>"WIN",
                    "LEFT"=>"<","DOWN"=>"v","RIGHT"=>">","UP"=>"^","HOME"=>"HOME","END"=>"END","DEL"=>"DEL",
                    "TAB"=>"TAB","ESC"=>"ESC","GRAVE"=>"GR",_=>label
                };
                let tw=short.len()*8;
                if tw<=kw { graphics::draw_str(x+(kw-tw)/2,y+((row_h.saturating_sub(3)).saturating_sub(8))/2,short,0x00F0F3F5); }
                x+=kw+gap;
                c+=1;
            }
            r+=1;
        }
    }
}

fn handle_terminal_keyboard_click(mx:i32,my:i32)->bool{
    unsafe{
        if !KEYBOARD_OPEN||FOCUS>=MAX_WIN||!WINS[FOCUS].visible||
           (WINS[FOCUS].kind!=WinKind::Terminal && WINS[FOCUS].kind!=WinKind::AIChat){return false;}
        let w=&WINS[FOCUS];
        let wx=w.x;
        let wy=w.y;
        let ww=w.w.max(220);
        let kh=terminal_keyboard_height(w.w) as i32;
        let panel_y=wy+w.h-kh;
        if my<panel_y+26||my>=wy+w.h-4||mx<wx+8||mx>=wx+ww-8{return false;}
        let rows:[&[&str];6]=[
            &["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12"],
            &["GRAVE","1","2","3","4","5","6","7","8","9","0","-","=","BACK"],
            &["TAB","q","w","e","r","t","y","u","i","o","p","[","]"],
            &["CAPS","a","s","d","f","g","h","j","k","l",";","'","ENTER"],
            &["SHIFT","z","x","c","v","b","n","m",",",".","/","SHIFT","UP"],
            &["CTRL","ALT","META","SPACE","LEFT","DOWN","RIGHT","HOME","END","DEL"]
        ];
        let row_h=((kh as usize-32)/6).max(18);
        let gap=3i32;
        let inner_w=(ww as usize).saturating_sub(20);
        let unit=(inner_w.saturating_sub(14*(gap as usize)))/15;
        let mut r=0usize;
        while r<rows.len(){
            let y=panel_y+26+(r*row_h) as i32;
            if my>=y&&my<y+row_h as i32-3{
                let mut x=wx+10;
                let mut c=0usize;
                while c<rows[r].len(){
                    let label=rows[r][c];
                    let units=if r==1&&c==13{2}else if r==2&&c==0{2}else if r==3&&c==0{2}else if r==3&&c==12{2}else if r==4&&(c==0||c==11){2}else if r==5&&c==3{4}else{1};
                    let kw=(unit*units+gap as usize*(units-1)) as i32;
                    if mx>=x&&mx<x+kw{keyboard_emit(label);return true;}
                    x+=kw+gap;
                    c+=1;
                }
            }
            r+=1;
        }
        false
    }
}

fn handle_keyboard_click(mx:i32,my:i32)->bool{
    unsafe{
        if FOCUS>=MAX_WIN||WINS[FOCUS].kind!=WinKind::Keyboard{return false;}
        let kx=WINS[FOCUS].x+5;let ky=WINS[FOCUS].y+TITLE_H+30;
        let rows:[&[&str];5]=[
            &["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12"],
            &["1","2","3","4","5","6","7","8","9","0","-","=","BACK"],
            &["TAB","q","w","e","r","t","y","u","i","o","p","[","]"],
            &["CAPS","a","s","d","f","g","h","j","k","l",";","'","ENTER"],
            &["SHIFT","z","x","c","v","b","n","m",",",".","/","UP","DOWN"]
        ];
        let widths:[i32;13]=[15,15,15,15,15,15,15,15,15,15,15,15,28];
        let mut r=0usize;
        while r<rows.len(){
            let mut x=kx;let y=ky+(r as i32)*17;let mut c=0usize;
            while c<rows[r].len(){
                let w=widths[c];
                if mx>=x&&mx<x+w&&my>=y&&my<y+14{keyboard_emit(rows[r][c]);return true;}
                x+=w+2;c+=1;
            }r+=1;
        }
        let y=ky+85;
        let bottom:[(&str,i32);8]=[("CTRL",24),("ALT",24),("SPACE",62),("LEFT",20),("RIGHT",20),("HOME",20),("END",20),("DEL",20)];
        let mut x=kx;let mut i=0usize;
        while i<bottom.len(){let(label,w)=bottom[i];
            if mx>=x&&mx<x+w&&my>=y&&my<y+14{keyboard_emit(label);return true;}
            x+=w+2;i+=1;
        }
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
        let right = buttons & 2;
        let prev_right = PREV_MB & 2;
        let mx = MX;
        let my = MY;
        FRAME_N = FRAME_N.wrapping_add(1);

        // Native terminal selection: drag across rendered output, then use
        // the terminal context menu for Copy/Paste. This is independent from
        // desktop/Explorer context menus.
        if TERM_SELECTING {
            if left != 0 {
                term_selection_update(mx, my);
                PREV_MB = buttons;
                MB = buttons;
                return;
            } else {
                TERM_SELECTING = false;
                DIRTY_FULL = true;
            }
        }

        let terminal_under = match hit_window(mx, my) {
            Some(i) if WINS[i].kind == WinKind::Terminal => true,
            _ => false,
        };

        if right != 0 && prev_right == 0 && terminal_under {
            TERM_MENU_X = mx;
            TERM_MENU_Y = my;
            TERM_MENU = true;
            CTX_MENU = false;
            START_MENU = false;
            DIRTY_FULL = true;
            PREV_MB = buttons;
            MB = buttons;
            return;
        }

        if TERM_MENU && left != 0 && prev_left == 0 {
            if let Some(act) = term_menu_hit(mx, my) {
                match act {
                    0 => term_copy_selection(),
                    1 => term_paste_clipboard(),
                    2 => term_selection_clear(),
                    _ => {}
                }
            } else {
                TERM_MENU = false;
                DIRTY_FULL = true;
            }
            PREV_MB = buttons;
            MB = buttons;
            return;
        }

        if left != 0 && prev_left == 0 && terminal_under {
            if term_selection_begin(mx, my) {
                PREV_MB = buttons;
                MB = buttons;
                return;
            }
        }

        // Integrated terminal keyboard: handle key cells before generic window hit-testing.
        if left != 0 && prev_left == 0 {
            if handle_terminal_keyboard_click(mx, my) {
                PREV_MB = buttons;
                MB = buttons;
                return;
            }
        }

                // Desktop context menu (right-click empty area)
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
            if handle_terminal_keyboard_button(mx, my) {
                PREV_MB = buttons;
                MB = buttons;
                return;
            }
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
                        7 => open_win(12), // Alarm Clock
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
                        7 => {
                            if crate::pe::launch_hello() {
                                open_win(13);
                            }
                        },
                        8 => {
                            if crate::winamp::launch() {
                                open_win(14);
                            }
                        },
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
                    BACKGROUND_DRAWN = false;
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
                    let nh = WINS[idx].h;
                    let body_y = ny + TITLE_H;
                    let lx = nx + 212;
                    let ly = body_y + 76;
                    let lh = nh.saturating_sub(TITLE_H + 122);
                    let by = ly + lh.saturating_sub(38);
                    if mx >= lx && mx < lx + 112 && my >= by && my < by + 28 {
                        if crate::drivers::wifi::found()
                            && crate::drivers::wifi::mmio_ready()
                            && crate::drivers::wifi::alive_seen()
                            && crate::drivers::wifi::command_queue_ready()
                        {
                            WIFI_UI_SELECTED = -1;
                            WIFI_UI_SCAN_REQUESTED = true;
                            WIFI_UI_STATUS = 1;
                            DIRTY_WINDOW = idx as i16;
                        } else {
                            WIFI_UI_STATUS = 3;
                            DIRTY_WINDOW = idx as i16;
                        }
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
                                    CURSOR_PENDING = CURSOR_ID;
                                }
                                DIRTY_WINDOW = idx as i16;
                                break;
                            }
                            i += 1;
                        }
                    } else if SETTINGS_VIEW == 1 {
                        let mut i = 0usize;
                        while i < crate::cursor_builtin::COUNT {
                            let col = i % 6; let row = i / 6;
                            let bx = sx + 18 + (col as i32) * 78; let by = sy + 96 + (row as i32) * 40;
                            if mx >= bx && mx < bx + 70 && my >= by && my < by + 34 {
                                CURSOR_PENDING = i as u8; DIRTY_WINDOW = idx as i16; break;
                            }
                            i += 1;
                        }
                        let okx = sx + sw - 184;
                        let cancelx = sx + sw - 94;
                        let by = sy + sh - 42;
                        if my >= by && my < by + 24 {
                            if mx >= okx && mx < okx + 78 {
                                CURSOR_ID = CURSOR_PENDING;
                                SETTINGS_VIEW = 0;
                                DIRTY_WINDOW = idx as i16;
                            } else if mx >= cancelx && mx < cancelx + 78 {
                                SETTINGS_VIEW = 0;
                                CURSOR_PENDING = 0;
                                DIRTY_WINDOW = idx as i16;
                            }
                        }
                    } else {
                        // Future-category placeholder: Back returns to Settings home.
                        let bx = sx + sw - 94;
                        let by = sy + sh - 42;
                        if mx >= bx && mx < bx + 78 && my >= by && my < by + 24 {
                            SETTINGS_VIEW = 0;
                            DIRTY_WINDOW = idx as i16;
                        }
                    }
                } else if WINS[idx].kind == WinKind::Video
                    && my >= WINS[idx].y + TITLE_H
                {
                    let vx = WINS[idx].x;
                    let vy = WINS[idx].y;
                    let vw = WINS[idx].w;
                    let vh = WINS[idx].h;

                    // Resolution rows.
                    let mut i = 0usize;
                    while i < 2 {
                        let row_y = vy + 76 + (i as i32) * 32;
                        if mx >= vx + 18 && mx < vx + vw - 18
                            && my >= row_y && my < row_y + 26
                        {
                            if i == 0 {
                                VIDEO_PENDING_W = 800;
                                VIDEO_PENDING_H = 600;
                            } else {
                                VIDEO_PENDING_W = 1366;
                                VIDEO_PENDING_H = 768;
                            }
                            VIDEO_STATUS = 0;
                            DIRTY_WINDOW = idx as i16;
                            break;
                        }
                        i += 1;
                    }

                    let by = vy + vh - 42;
                    let apply_x = vx + vw - 184;
                    let cancel_x = vx + vw - 94;
                    if my >= by && my < by + 24 {
                        if mx >= apply_x && mx < apply_x + 78 {
                            let pw = VIDEO_PENDING_W;
                            let ph = VIDEO_PENDING_H;
                            // Use the existing guarded Sandy Bridge mode path.
                            // No new register programming is introduced here.
                            let ok = crate::drivers::video::modeset_to(pw, ph);
                            VIDEO_STATUS = if ok { 1 } else { 2 };
                            if ok {
                                VIDEO_PENDING_W = graphics::width() as u16;
                                VIDEO_PENDING_H = graphics::height() as u16;
                                if graphics::enable_backbuffer() {
                                    serial::write_str("[FB] backbuffer recreated after modeset\n");
                                } else {
                                    serial::write_str("[FB] backbuffer unavailable after modeset; direct framebuffer\n");
                                }
                                handle_resolution_changed();
                                ps2::clamp_to_screen();
                            }
                            DIRTY_FULL = true;
                        } else if mx >= cancel_x && mx < cancel_x + 78 {
                            VIDEO_PENDING_W = graphics::width() as u16;
                            VIDEO_PENDING_H = graphics::height() as u16;
                            VIDEO_STATUS = 0;
                            DIRTY_WINDOW = idx as i16;
                        }
                    }
                } else if WINS[idx].kind == WinKind::Alarm
                    && my >= WINS[idx].y + TITLE_H
                {
                    let changed = crate::alarm::alarm_click(
                        mx - WINS[idx].x - 3,
                        my - WINS[idx].y - TITLE_H,
                    );
                    if changed { DIRTY_WINDOW = idx as i16; }
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
                            DIRTY_WINDOW=idx as i16;
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
                        DIRTY_WINDOW=idx as i16;
                    } else if my >= cy+126 && my < cy+142 {
                        let barw=(WINS[idx].w-36).max(1) as i32;
                        let rel=(mx-(bx+18)).max(0).min(barw) as u16;
                        let perm=((rel as u32)*1000/(barw as u32)) as u16;
                        crate::media_player::seek_permille(perm);
                        DIRTY_WINDOW=idx as i16;
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
                    DIRTY_WINDOW=idx as i16;
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
    crate::cursor_builtin::draw(unsafe { CURSOR_ID as usize }, x, y);
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
                WinKind::Alarm => IconId::Settings,
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
                let body_y = wy + TITLE_H as usize;
                let body_h = wh.saturating_sub(TITLE_H as usize + 3);
                graphics::fill_rect(wx + 3, body_y, ww - 6, body_h, 0x0013161B);
                // Modern terminal tab strip: one real shell session today,
                // with room reserved for future tabs without pretending they exist.
                graphics::fill_rect(wx + 4, body_y + 2, ww - 8, 25, 0x001D2229);
                graphics::fill_rect(wx + 5, body_y + 3, 142, 22, 0x00282F38);
                graphics::fill_rect(wx + 5, body_y + 24, 142, 1, 0x0000B7C3);
                graphics::draw_str(wx + 14, body_y + 10, "Aether Shell", 0x00F2F2F2);
                graphics::draw_str(wx + 158, body_y + 10, "+", 0x00A0A8B0);
                let kb_x = wx + ww.saturating_sub(84);
                graphics::fill_rect(kb_x, body_y + 4, 30, 20, if KEYBOARD_OPEN { 0x003E9CCB } else { 0x00282F38 });
                graphics::border_rect(kb_x, body_y + 4, 30, 20, 0x006A747C);
                graphics::draw_str(kb_x + 7, body_y + 10, "KB", 0x00F2F2F2);
                graphics::draw_str(wx + ww.saturating_sub(48), body_y + 10, "LOCAL", 0x007D8791);
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
                        let y = wy + TITLE_H as usize + 34 + r * 10;
                        if y + 8 < wy + wh {
                            let len = TERM_LEN[idx];
                            let mut k = 0usize;
                            while k < len {
                                if term_selection_contains(idx, k) {
                                    graphics::fill_rect(wx + 10 + k * 8, y, 8, 10, 0x0000B7C3);
                                    graphics::draw_char(wx + 10 + k * 8, y, TERM_LINES[idx][k], 0x00FFFFFF);
                                } else if term_search_contains(idx, k) {
                                    graphics::fill_rect(wx + 10 + k * 8, y, 8, 10, 0x00FFD966);
                                    graphics::draw_char(wx + 10 + k * 8, y, TERM_LINES[idx][k], 0x00000000);
                                } else {
                                    graphics::draw_char(wx + 10 + k * 8, y, TERM_LINES[idx][k], term_ansi_color(TERM_COLOR[idx][k]));
                                }
                                k += 1;
                            }
                        }
                    }
                    r += 1;
                }
                // Visible terminal scrollbar: up/down buttons + proportional thumb.
                let bar_x = wx + ww - 18;
                let bar_top = wy + TITLE_H as usize + 31;
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
                let y = wy + TITLE_H as usize + 34 + (view_rows.saturating_sub(1)) * 10;
                if y + 8 < wy + wh && focused && !TERM_PAGE_MODE {
                    graphics::draw_str(wx + 10, y, "aether> ", COL_TERM_PROMPT);
                    let mut k = 0usize;
                    while k < INPUT_LEN {
                        graphics::draw_char(wx + 10 + 64 + k * 8, y, INPUT[k], COL_TERM_FG);
                        k += 1;
                    }
                    graphics::fill_rect(wx + 10 + 64 + INPUT_CURSOR * 8, y, 6, 8, 0x0000D7FF);
                }
                let footer_y = wy + wh - 22;
                graphics::fill_rect(wx + 4, footer_y, ww - 8, 18, 0x001D2229);
                graphics::border_rect(wx + 4, footer_y, ww - 8, 18, 0x00505050);
                graphics::draw_str(wx + 10, footer_y + 5,
                    if TERM_PAGE_MODE { "PAGE MODE  SPACE: NEXT  BACKSPACE: PREVIOUS" } else { "ASCII | UP/DOWN: HISTORY | DRAG: SELECT | RIGHT CLICK: MENU" },
                    if TERM_PAGE_MODE { 0x00FFD24A } else { 0x007D8791 });
                draw_terminal_keyboard(idx);
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
            WinKind::AIChat => {
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6,
                    wh - TITLE_H as usize - 3, 0x0013161B);
                graphics::fill_rect(wx + 5, wy + TITLE_H as usize + 4, ww - 10, 24, 0x001D2229);
                graphics::draw_str(wx + 16, wy + TITLE_H as usize + 12, "Aether AI / TEXT BRIDGE", 0x00F2F2F2);
                let ai_kb_x = wx + ww.saturating_sub(84);
                graphics::fill_rect(ai_kb_x, wy + TITLE_H as usize + 4, 30, 20,
                    if unsafe { KEYBOARD_OPEN && KEYBOARD_TARGET == idx } { 0x003E9CCB } else { 0x00282F38 });
                graphics::border_rect(ai_kb_x, wy + TITLE_H as usize + 4, 30, 20, 0x006A747C);
                graphics::draw_str(ai_kb_x + 7, wy + TITLE_H as usize + 10, "KB", 0x00F2F2F2);
                graphics::draw_str(wx + ww.saturating_sub(170), wy + TITLE_H as usize + 12,
                    if unsafe { AI_BRIDGE_ACTIVE } { "MODEL BRIDGE: ACTIVE" } else { "MODEL BRIDGE: WAITING" }, 0x0066D966);
                let top = wy + TITLE_H as usize + 38;
                let bottom = wy + wh.saturating_sub(48);
                let visible = if bottom > top { (bottom - top) / 14 } else { 0 };
                unsafe {
                    let start = if AI_HISTORY_COUNT > visible { AI_HISTORY_COUNT - visible } else { 0 };
                    let mut r = start;
                    let mut row = 0usize;
                    while r < AI_HISTORY_COUNT && row < visible {
                        let y = top + row * 14;
                        let len = AI_HISTORY_LEN[r];
                        let mut k = 0usize;
                        while k < len && wx + 12 + k * 8 < wx + ww - 22 {
                            let c = AI_HISTORY[r][k];
                            let col = if k < 4 && c == b'Y' { 0x00FFD966 } else { 0x00C0C0C0 };
                            graphics::draw_char(wx + 12 + k * 8, y, c, col);
                            k += 1;
                        }
                        r += 1; row += 1;
                    }
                }
                let keyboard_for_ai = unsafe { KEYBOARD_OPEN && KEYBOARD_TARGET == idx };
                let keyboard_h = if keyboard_for_ai { terminal_keyboard_height(w.w) } else { 0 };
                let iy = wy + wh.saturating_sub(44 + keyboard_h);
                graphics::fill_rect(wx + 8, iy, ww - 16, 28, 0x000B0D10);
                graphics::border_rect(wx + 8, iy, ww - 16, 28, 0x00505050);
                graphics::draw_str(wx + 16, iy + 10, "YOU>", 0x0066D966);
                unsafe {
                    let mut k = 0usize;
                    while k < AI_INPUT_LEN && wx + 56 + k * 8 < wx + ww - 22 {
                        graphics::draw_char(wx + 56 + k * 8, iy + 10, AI_INPUT[k], 0x00E0E0E0);
                        k += 1;
                    }
                    if focused {
                        graphics::fill_rect(wx + 56 + AI_INPUT_CURSOR * 8, iy + 9, 6, 9, 0x0000D7FF);
                    }
                }
                if !keyboard_for_ai {
                    graphics::draw_str(wx + 12, wy + wh - 12,
                        "ENTER: send | ESC: menus | MODEL BRIDGE: external", 0x007D8791);
                } else {
                    graphics::draw_str(wx + 12, iy + 36,
                        "Keyboard: on-screen input", 0x007D8791);
                    draw_terminal_keyboard(idx);
                }
            }
            WinKind::Alarm => {
                crate::alarm::alarm_draw(wx as i32 + 3, wy as i32 + TITLE_H);
            }
            WinKind::Keyboard => {
                let ky=wy+TITLE_H as usize+30;let kx=wx+5;
                graphics::fill_rect(wx+3,wy+TITLE_H as usize,ww-6,wh-TITLE_H as usize-3,0x00ECE9D8);
                graphics::draw_str(wx+8,wy+25,"Terminal keyboard",COL_TEXT_DIM);
                let rows:[&[&str];5]=[
                    &["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12"],
                    &["1","2","3","4","5","6","7","8","9","0","-","=","BACK"],
                    &["TAB","q","w","e","r","t","y","u","i","o","p","[","]"],
                    &["CAPS","a","s","d","f","g","h","j","k","l",";","'","ENTER"],
                    &["SHIFT","z","x","c","v","b","n","m",",",".","/","UP","DOWN"]
                ];
                let widths:[i32;13]=[15,15,15,15,15,15,15,15,15,15,15,15,28];
                let mut r=0usize;
                while r<rows.len(){let mut x=kx;let y=ky+r*17;let mut c=0usize;
                    while c<rows[r].len(){let label=rows[r][c];let w=widths[c] as usize;
                        let active=unsafe{(label=="SHIFT"&&KEYBOARD_SHIFT)||(label=="CAPS"&&KEYBOARD_CAPS)};
                        graphics::fill_rect(x as usize,y,w,14,if active{0x00B8D4FF}else{COL_BTN_FACE});
                        graphics::border_rect(x as usize,y,w,14,0x00606060);
                        let short=match label {
                            "BACK"=>"BK","ENTER"=>"EN","SHIFT"=>"SH","CAPS"=>"CA","TAB"=>"T","ESC"=>"E",
                            "F10"=>"10","F11"=>"11","F12"=>"12","UP"=>"^","DOWN"=>"v",_=>label
                        };
                        let tw=short.len()*8;
                        if tw<=w { graphics::draw_str(x as usize+(w-tw)/2,y+3,short,COL_TEXT); }
                        else if short.len()==1 { graphics::draw_char(x as usize+3,y+3,short.as_bytes()[0],COL_TEXT); }
                        x+=(w)+2;c+=1;}
                    r+=1;
                }
                let y=ky+85;
                let bottom:[(&str,i32);8]=[("CTRL",24),("ALT",24),("SPACE",62),("LEFT",20),("RIGHT",20),("HOME",20),("END",20),("DEL",20)];
                let mut x=kx;let mut i=0usize;
                while i<bottom.len(){let(label,w)=bottom[i];
                    let active=unsafe{(label=="CTRL"&&KEYBOARD_CTRL)||(label=="ALT"&&KEYBOARD_ALT)};
                    graphics::fill_rect(x as usize,y,w as usize,14,if active{0x00B8D4FF}else{COL_BTN_FACE});
                    graphics::border_rect(x as usize,y,w as usize,14,0x00606060);
                    let short=match label {"CTRL"=>"C","ALT"=>"A","SPACE"=>"_","LEFT"=>"<","RIGHT"=>">","HOME"=>"H","END"=>"E","DEL"=>"D",_=>label};
                    let tw=short.len()*8;
                    if tw<=w as usize { graphics::draw_str(x as usize+((w as usize-tw)/2),y+3,short,COL_TEXT); }
                    x+=(w as usize)+2;i+=1;}
            }
            WinKind::HelloExe => {
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6,
                    wh - TITLE_H as usize - 3, 0x00F4F4F4);
                graphics::draw_str(wx + 24, wy + 58, "Hello from Windows EXE", 0x00000000);
                graphics::draw_str(wx + 24, wy + 84, "Aether PE64 compatibility", 0x00404040);
                graphics::draw_str(wx + 24, wy + 108, "MZ + PE64 + AMD64: VALID", 0x00008000);
                graphics::draw_str(wx + 24, wy + 132, "Entry RVA: 0x1000", 0x00404040);
                graphics::draw_str(wx + 24, wy + 156, "This is the first safe PE launch.", 0x00404040);
                graphics::draw_str(wx + 24, wy + 180, "Native code execution comes next.", 0x00800000);
            }
            WinKind::WinampExe => {
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6,
                    wh - TITLE_H as usize - 3, 0x00F4F4F4);
                graphics::draw_str(wx + 24, wy + 58, "Winamp 5.9.2", 0x00000000);
                graphics::draw_str(wx + 24, wy + 84, "Real downloaded Winamp executable", 0x00404040);
                graphics::draw_str(wx + 24, wy + 110, "PE32 / x86: DETECTED", 0x00008000);
                graphics::draw_str(wx + 24, wy + 136, "Third-party binary: NOT recreated", 0x00404040);
                graphics::draw_str(wx + 24, wy + 162, "x86 Win32 execution layer is next.", 0x00800000);
                graphics::draw_str(wx + 24, wy + 188, "No emulator or Windows OS is used.", 0x00404040);
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
                // Native Aether Wi-Fi surface. Values come from the driver;
                // no SSID, signal or connection state is fabricated.
                graphics::fill_rect(wx + 3, wy + TITLE_H as usize, ww - 6,
                    wh - TITLE_H as usize - 3, 0x00F4F7FB);
                let body_y = wy + TITLE_H as usize;
                graphics::fill_rect(wx + 3, body_y, ww - 6, 62, 0x00EAF2FB);
                graphics::fill_rect(wx + 3, body_y + 61, ww - 6, 1, 0x00B7C9DE);
                graphics::draw_str(wx + 18, body_y + 14, "Wi-Fi", 0x001F4E79);
                graphics::draw_str(wx + 18, body_y + 34,
                    "Intel Centrino Wireless-N 2230", 0x00444F5C);

                let card_x = wx + 14;
                let card_y = body_y + 76;
                let card_w = 184usize;
                let card_h = wh.saturating_sub(TITLE_H as usize + 122);
                graphics::fill_rect(card_x, card_y, card_w, card_h, 0x00E8EEF5);
                graphics::border_rect(card_x, card_y, card_w, card_h, 0x00C7D2DF);
                graphics::draw_str(card_x + 14, card_y + 16, "ADAPTER", 0x00606A73);

                let found = crate::drivers::wifi::found();
                let alive = crate::drivers::wifi::alive_seen();
                let mmio = crate::drivers::wifi::mmio_ready();
                let cmdq = crate::drivers::wifi::command_queue_ready();
                let scan_seen = crate::drivers::wifi::ui_scan_notification_seen();
                let status = unsafe { WIFI_UI_STATUS };
                let state_text = if !found { "Not detected" }
                    else if !mmio { "PCI found / MMIO offline" }
                    else if !alive { "Firmware not alive" }
                    else if !cmdq { "Firmware alive / command queue offline" }
                    else if status == 1 { "Scanning..." }
                    else if status == 3 { "Scan request failed" }
                    else if scan_seen { "Scan notifications received" }
                    else { "Ready" };
                let state_col = if !found || !mmio { 0x00800000 }
                    else if !alive || !cmdq { 0x00B06000 }
                    else if status == 1 { 0x001E5AA8 }
                    else { 0x00008000 };
                graphics::draw_str(card_x + 14, card_y + 40, state_text, state_col);
                graphics::draw_str(card_x + 14, card_y + 70, "PCI", 0x00717D89);
                graphics::draw_str(card_x + 58, card_y + 70, if found { "8:0.0" } else { "--" }, 0x00333333);
                graphics::draw_str(card_x + 14, card_y + 90, "MMIO", 0x00717D89);
                graphics::draw_str(card_x + 58, card_y + 90, if mmio { "READY" } else { "OFFLINE" }, 0x00333333);
                graphics::draw_str(card_x + 14, card_y + 110, "CMDQ", 0x00717D89);
                graphics::draw_str(card_x + 58, card_y + 110, if cmdq { "READY" } else { "OFFLINE" }, 0x00333333);
                graphics::draw_str(card_x + 14, card_y + 130, "RF", 0x00717D89);
                graphics::draw_str(card_x + 58, card_y + 130, if alive { "ALIVE" } else { "OFFLINE" }, 0x00333333);

                let lx = wx + 212;
                let ly = body_y + 76;
                let lw = ww.saturating_sub(226);
                let lh = wh.saturating_sub(TITLE_H as usize + 122);
                graphics::fill_rect(lx, ly, lw, lh, 0x00FFFFFF);
                graphics::border_rect(lx, ly, lw, lh, 0x00AAB7C5);
                graphics::draw_str(lx + 14, ly + 16, "Wireless scan", 0x001F4E79);

                let starts = crate::drivers::wifi::ui_scan_start_count();
                let results = crate::drivers::wifi::ui_scan_results_count();
                let completes = crate::drivers::wifi::ui_scan_complete_count();
                let channels = crate::drivers::wifi::ui_scan_complete_channels();
                let scan_status = crate::drivers::wifi::ui_scan_complete_status();
                if status == 1 {
                    graphics::draw_str(lx + 14, ly + 44, "Scanning 2.4 GHz channels 1-11...", 0x001E5AA8);
                } else if status == 3 {
                    graphics::draw_str(lx + 14, ly + 44, "The native scan did not complete.", 0x00800000);
                } else if completes > 0 {
                    graphics::draw_str(lx + 14, ly + 44, "Scan completed.", 0x00008000);
                } else {
                    graphics::draw_str(lx + 14, ly + 44, "No scan has been completed yet.", 0x00606A73);
                }
                graphics::draw_str(lx + 14, ly + 72, "Scan starts", 0x00717D89);
                draw_u32(lx + 110, ly + 72, starts, 0x00333333);
                graphics::draw_str(lx + 14, ly + 92, "Result notifications", 0x00717D89);
                draw_u32(lx + 154, ly + 92, results, 0x00333333);
                graphics::draw_str(lx + 14, ly + 112, "Complete notifications", 0x00717D89);
                draw_u32(lx + 154, ly + 112, completes, 0x00333333);
                if completes > 0 {
                    graphics::draw_str(lx + 14, ly + 140, "Channels reported", 0x00717D89);
                    draw_u32(lx + 154, ly + 140, channels as u32, 0x00333333);
                    graphics::draw_str(lx + 14, ly + 160, "Last status", 0x00717D89);
                    draw_u32(lx + 154, ly + 160, scan_status as u32, 0x00333333);
                } else {
                    graphics::draw_str(lx + 14, ly + 140, "SSID list", 0x00717D89);
                    graphics::draw_str(lx + 14, ly + 158,
                        "Waiting for decoded 802.11 RX frames.", 0x00717D89);
                }

                let by = ly + lh.saturating_sub(38);
                let enabled = found && mmio && alive && cmdq;
                graphics::fill_rect(lx, by, 112, 28, if enabled { 0x00E8F2FA } else { 0x00E0E0E0 });
                graphics::border_rect(lx, by, 112, 28, if enabled { 0x00316AC5 } else { 0x008A8A8A });
                graphics::draw_str(lx + 28, by + 10, "Scan now",
                    if enabled { 0x001E5AA8 } else { 0x00707070 });
                graphics::draw_str(lx + 128, by + 10,
                    if enabled { "Native driver" } else { "Driver not ready" }, 0x00717D89);

                let sy = wy + wh - 34;
                graphics::fill_rect(wx + 3, sy, ww - 6, 30, 0x00E8EDF3);
                graphics::fill_rect(wx + 3, sy, ww - 6, 1, 0x00C0CBD7);
                graphics::draw_str(wx + 18, sy + 10,
                    if alive && cmdq { "Firmware + command transport active" }
                    else if found { "Adapter detected; transport not ready" }
                    else { "No Intel 2230 adapter detected" }, 0x005A6673);
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

                graphics::draw_str(wx + 18, wy + 42, "AETHER GRAPHICS", COL_TEXT);
                graphics::draw_str(wx + 18, wy + 60, "Screen mode", COL_TEXT_DIM);

                // The current Gen6 path has only been exercised for these two
                // AH532 target modes. Do not advertise arbitrary EDID modes yet.
                let modes: [(u16, u16, &str); 2] = [
                    (800, 600, "800 x 600"),
                    (1366, 768, "1366 x 768"),
                ];
                let mut i = 0usize;
                while i < modes.len() {
                    let (mw, mh, label) = modes[i];
                    let row_y = wy + 76 + i * 32;
                    let selected = unsafe { VIDEO_PENDING_W == mw && VIDEO_PENDING_H == mh };
                    graphics::fill_rect(
                        wx + 18, row_y, ww - 36, 26,
                        if selected { 0x00DCEBFA } else { 0x00FFFFFF }
                    );
                    graphics::border_rect(
                        wx + 18, row_y, ww - 36, 26,
                        if selected { COL_ACCENT } else { 0x00808080 }
                    );
                    graphics::draw_str(wx + 30, row_y + 9, if selected { "o" } else { " " }, COL_TEXT);
                    graphics::draw_str(wx + 52, row_y + 9, label, COL_TEXT);
                    i += 1;
                }

                graphics::draw_str(wx + 18, wy + 150, "Graphics adapter:", COL_TEXT_DIM);
                graphics::draw_str(wx + 150, wy + 150, "Intel HD Graphics 3000", COL_TEXT);
                graphics::draw_str(wx + 18, wy + 168, "Generation:", COL_TEXT_DIM);
                graphics::draw_str(wx + 150, wy + 168, "6", COL_TEXT);
                graphics::draw_str(wx + 18, wy + 186, "Current:", COL_TEXT_DIM);
                draw_u32(wx + 150, wy + 186, graphics::width() as u32, COL_TEXT);
                graphics::draw_str(wx + 190, wy + 186, "x", COL_TEXT);
                draw_u32(wx + 202, wy + 186, graphics::height() as u32, COL_TEXT);

                let status = unsafe { VIDEO_STATUS };
                if status == 1 {
                    graphics::draw_str(wx + 18, wy + 208, "Mode applied.", 0x00008000);
                } else if status == 2 {
                    graphics::draw_str(wx + 18, wy + 208, "Mode change failed; current mode kept.", 0x00800000);
                } else {
                    graphics::draw_str(wx + 18, wy + 208, "Select a mode, then press Apply.", COL_TEXT_DIM);
                }

                let by = wy + wh - 42;
                let apply_x = wx + ww - 184;
                let cancel_x = wx + ww - 94;
                graphics::fill_rect(apply_x, by, 78, 24, COL_BTN_FACE);
                graphics::border_rect(apply_x, by, 78, 24, 0x00404040);
                graphics::draw_str(apply_x + 22, by + 8, "Apply", COL_TEXT);
                graphics::fill_rect(cancel_x, by, 78, 24, COL_BTN_FACE);
                graphics::border_rect(cancel_x, by, 78, 24, 0x00404040);
                graphics::draw_str(cancel_x + 20, by + 8, "Cancel", COL_TEXT);

                if !crate::drivers::video::hardware_ready() {
                    graphics::draw_str(wx + 18, wy + 226, "Hardware display path: not ready", 0x00800000);
                }
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
            graphics::draw_str(wx + 18, wy + 62, "Cursor", COL_TEXT_DIM);
            graphics::border_rect(wx + 14, wy + 76, ww - 28, 220, 0x00808080);

            // 24 native cursor variants: 4 shapes x 6 colors.
            // The click geometry is shared with handle_mouse_buttons().
            let shapes: [&str; 4] = ["Arrow", "Pointer", "Cross", "Text"];
            let colors: [&str; 6] = ["Red", "Blue", "Green", "Gold", "Purple", "Cyan"];

            let mut i = 0usize;
            while i < crate::cursor_builtin::COUNT {
                let col = i % 6;
                let row = i / 6;
                let bx = wx + 18 + col * 78;
                let by = wy + 96 + row * 40;
                let selected = CURSOR_PENDING == i as u8;

                graphics::fill_rect(
                    bx as usize,
                    by as usize,
                    70,
                    34,
                    if selected { 0x00DCEBFA } else { 0x00FFFFFF },
                );
                graphics::border_rect(
                    bx as usize,
                    by as usize,
                    70,
                    46,
                    if selected { 0x00316AC5 } else { 0x00808080 },
                );

                // Crisp 16x16 preview from the exact bundled cursor asset.
                crate::cursor_builtin::draw(
                    i,
                    (bx + 5) as usize,
                    (by + 3) as usize,
                );

                let shape = shapes[row];
                let color = colors[col];
                graphics::draw_str(bx as usize + 25, by as usize + 5, shape, COL_TEXT);
                graphics::draw_str(bx as usize + 25, by as usize + 19, color, COL_TEXT_DIM);
                i += 1;
            }

            graphics::draw_str(wx + 18, wy + 274, "Selected:", COL_TEXT_DIM);
            let selected_name = crate::cursor_builtin::name(CURSOR_PENDING as usize);
            graphics::draw_str(wx + 82, wy + 274, selected_name, COL_TEXT);

            let okx = wx + ww - 184;
            let cancelx = wx + ww - 94;
            let by = wy + wh - 42;

            graphics::fill_rect(okx, by, 78, 24, COL_BTN_FACE);
            graphics::border_rect(okx, by, 78, 24, 0x00404040);
            graphics::draw_str(okx + 22, by + 8, "Apply", COL_TEXT);

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
            graphics::draw_str(wx + 212, wy + 116, "Cursor selection is available.", COL_TEXT_DIM);
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
    // Network icon reflects the real Intel 2230 state; never fake "connected".
    let nx = tray_x + 2;
    let wifi_found = crate::drivers::wifi::found();
    let wifi_alive = crate::drivers::wifi::alive_seen();
    let wifi_scan = unsafe { WIFI_UI_STATUS == 1 };
    let wifi_col = if wifi_scan { 0x00FFD966 }
        else if wifi_alive { 0x0047D147 }
        else if wifi_found { 0x00D0D0D0 }
        else { 0x00808080 };
    graphics::fill_rect(nx + 3, cy + 8, 2, 3, wifi_col);
    graphics::fill_rect(nx + 6, cy + 6, 2, 5, wifi_col);
    graphics::fill_rect(nx + 9, cy + 4, 2, 7, wifi_col);
    graphics::fill_rect(nx + 12, cy + 2, 2, 9, wifi_col);
    graphics::fill_rect(nx + 1, cy + 11, 14, 1, 0x00505050);
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
        if !wallpaper::draw_region(ox, oy, ow, oh) {
            graphics::fill_rect(ox, oy, ow, oh, COL_BG);
        }
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
        graphics::present();
    }
}




fn redraw_terminal_input(idx: usize) {
    unsafe {
        if idx >= MAX_WIN || !WINS[idx].visible || WINS[idx].minimized ||
           WINS[idx].kind != WinKind::Terminal || FOCUS != idx {
            return;
        }
        // Keyboard input changes only the live input row. Redrawing the whole
        // terminal window still looks like a flash on the real LFB, even
        // though it no longer reaches DIRTY_FULL.
        cursor_restore();
        let w = &WINS[idx];
        let wx = w.x as usize;
        let wy = w.y as usize;
        let ww = w.w as usize;
        let wh = w.h as usize;
        let view_rows = term_visible_rows();
        let y = wy + TITLE_H as usize + 34 + (view_rows.saturating_sub(1)) * 10;
        if y + 8 < wy + wh && !TERM_PAGE_MODE {
            graphics::fill_rect(wx + 8, y, ww.saturating_sub(28), 10, 0x0013161B);
            graphics::draw_str(wx + 10, y, "aether> ", COL_TERM_PROMPT);
            let mut k = 0usize;
            while k < INPUT_LEN {
                graphics::draw_char(wx + 10 + 64 + k * 8, y, INPUT[k], COL_TERM_FG);
                k += 1;
            }
            graphics::fill_rect(wx + 10 + 64 + INPUT_CURSOR * 8, y, 6, 8, 0x0000D7FF);
        }
        CURSOR_SAVED = false;
        cursor_save_and_draw(MX, MY);
    }
}

fn redraw_single_window(idx: usize) {
    unsafe {
        if idx >= MAX_WIN || !WINS[idx].visible || WINS[idx].minimized { return; }
        if WINS[idx].kind == WinKind::Terminal {
            redraw_terminal_input(idx);
            return;
        }
        // Non-terminal windows keep the existing local redraw path.
        cursor_restore();
        draw_window(idx);
        CURSOR_SAVED = false;
        cursor_save_and_draw(MX, MY);
        graphics::present();
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
    let menu_h = 364usize;
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
        (IconId::Settings, "Alarm Clock", 12),
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
    let menu_h = 364i32;
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

static mut BACKGROUND_DRAWN: bool = false;
static mut DIRTY_WINDOW: i16 = -1;

/// Rebuild compositor geometry after a successful hardware mode change.
/// Maximized windows follow the new work area; other windows are clamped to it.
/// The next full render repaints the new backbuffer from scratch.
fn handle_resolution_changed() {
    let sw = graphics::width() as i32;
    let sh = graphics::height() as i32;
    if sw <= 0 || sh <= 70 {
        return;
    }
    unsafe {
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].maximized {
                WINS[i].x = 0;
                WINS[i].y = 30;
                WINS[i].w = sw;
                WINS[i].h = sh - 70;
            } else {
                clamp_win(i);
            }
            i += 1;
        }
        MX = MX.clamp(0, sw - 1);
        MY = MY.clamp(0, sh - 1);
        CURSOR_SAVED = false;
        CURSOR_OX = -1;
        CURSOR_OY = -1;
        BACKGROUND_DRAWN = false;
        DIRTY_FULL = true;
        DIRTY_WINDOW = -1;
        DIRTY_CURSOR = false;
    }
    serial::write_str("[DESKTOP] geometry recalculated for ");
    serial::write_usize(sw as usize);
    serial::write_str("x");
    serial::write_usize(sh as usize);
    serial::write_str("\n");
}

fn render() {
    let w = graphics::width();
    let h = graphics::height();
    if w == 0 || h == 0 {
        return;
    }
    // The framebuffer already contains the desktop background after the
    // initial render. Redrawing the full wallpaper on every ordinary mouse
    // click causes a visible flash on the real LFB. Keep it stable and only
    // paint it once here; localized redraw paths handle moving/closing UI.
    unsafe {
        if !BACKGROUND_DRAWN {
            if !wallpaper::draw(w, h) {
                draw_xp_wallpaper(w, h);
            }
            BACKGROUND_DRAWN = true;
        }
    }
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
        if TERM_MENU {
            draw_term_menu();
        }
        CURSOR_SAVED = false;
        cursor_save_and_draw(MX, MY);
        graphics::present();
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
        } else if WINS[FOCUS].kind == WinKind::Alarm {
            match hid_code {
                0x52 => crate::alarm::alarm_key(crate::alarm::KEY_UP),
                0x51 => crate::alarm::alarm_key(crate::alarm::KEY_DOWN),
                _ => false,
            }
        } else if WINS[FOCUS].kind == WinKind::AIChat {
            match hid_code {
                0x50 => { if AI_INPUT_CURSOR > 0 { AI_INPUT_CURSOR -= 1; DIRTY_WINDOW = FOCUS as i16; } true }
                0x4F => { if AI_INPUT_CURSOR < AI_INPUT_LEN { AI_INPUT_CURSOR += 1; DIRTY_WINDOW = FOCUS as i16; } true }
                0x4A => { AI_INPUT_CURSOR = 0; DIRTY_WINDOW = FOCUS as i16; true }
                0x4D => { AI_INPUT_CURSOR = AI_INPUT_LEN; DIRTY_WINDOW = FOCUS as i16; true }
                0x4C => {
                    if AI_INPUT_CURSOR < AI_INPUT_LEN {
                        let mut i = AI_INPUT_CURSOR;
                        while i + 1 < AI_INPUT_LEN { AI_INPUT[i] = AI_INPUT[i + 1]; i += 1; }
                        AI_INPUT_LEN -= 1; AI_INPUT[AI_INPUT_LEN] = 0;
                    }
                    DIRTY_WINDOW = FOCUS as i16; true
                }
                _ => false,
            }
        } else if WINS[FOCUS].kind == WinKind::Terminal {
            match hid_code {
                0x52 => { term_history_prev(); true }
                0x51 => { term_history_next(); true }
                0x50 => {
                    if INPUT_CURSOR > 0 { INPUT_CURSOR -= 1; DIRTY_WINDOW = FOCUS as i16; }
                    true
                }
                0x4F => {
                    if INPUT_CURSOR < INPUT_LEN { INPUT_CURSOR += 1; DIRTY_WINDOW = FOCUS as i16; }
                    true
                }
                0x4A => { INPUT_CURSOR = 0; DIRTY_WINDOW = FOCUS as i16; true }
                0x4D => { INPUT_CURSOR = INPUT_LEN; DIRTY_WINDOW = FOCUS as i16; true }
                0x4C => {
                    if INPUT_CURSOR < INPUT_LEN {
                        let mut i = INPUT_CURSOR;
                        while i + 1 < INPUT_LEN {
                            INPUT[i] = INPUT[i + 1];
                            i += 1;
                        }
                        INPUT_LEN -= 1;
                        INPUT[INPUT_LEN] = 0;
                        DIRTY_WINDOW = FOCUS as i16;
                    }
                    true
                }
                0x4B | 0x4E => {
                    if TERM_VIEW > 0 || TERM_PAGE_MODE { term_scroll_up(); }
                    true
                }
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
            TERM_MENU = false;
            TERM_SELECTING = false;
            DIRTY_FULL = true;
            return;
        }
        if FOCUS >= MAX_WIN || !WINS[FOCUS].visible {
            return;
        }
        if WINS[FOCUS].kind == WinKind::Alarm {
            if crate::alarm::alarm_key(ch) {
                DIRTY_FULL = true;
            }
            return;
        }
        if WINS[FOCUS].kind == WinKind::AIChat {
            if ch == b'\n' {
                ai_submit();
                DIRTY_FULL = true;
            } else if ch == 0x08 {
                if AI_INPUT_CURSOR > 0 {
                    let remove_at = AI_INPUT_CURSOR - 1;
                    let mut i = remove_at;
                    while i + 1 < AI_INPUT_LEN { AI_INPUT[i] = AI_INPUT[i + 1]; i += 1; }
                    AI_INPUT_LEN -= 1; AI_INPUT_CURSOR -= 1; AI_INPUT[AI_INPUT_LEN] = 0;
                    DIRTY_WINDOW = FOCUS as i16;
                }
            } else if ch >= 32 && ch < 127 && AI_INPUT_LEN < 90 {
                let mut i = AI_INPUT_LEN;
                while i > AI_INPUT_CURSOR { AI_INPUT[i] = AI_INPUT[i - 1]; i -= 1; }
                AI_INPUT[AI_INPUT_CURSOR] = ch;
                AI_INPUT_LEN += 1; AI_INPUT_CURSOR += 1;
                DIRTY_WINDOW = FOCUS as i16;
            }
            return;
        }
        if WINS[FOCUS].kind != WinKind::Terminal {
            return;
        }
        if ch == b'\n' {
            // Echo the exact byte buffer before dispatch so GUI command routing
            // is directly observable during hardware diagnostics.
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            terminal_write("\x1b[32maether>\x1b[0m CMD-IN=[");
            let mut k = 0usize;
            while k < INPUT_LEN {
                term_putc(INPUT[k]);
                k += 1;
            }
            terminal_write("]\n");
            let output_start = TERM_ROW;
            term_history_save();
            if ai_terminal_command(&INPUT[..INPUT_LEN]) {
                if INPUT_LEN == 4 {
                    terminal_write("VIRT: usage: VIRT <question>\n");
                } else {
                    ai_terminal_request(&INPUT[5..INPUT_LEN]);
                }
                INPUT_LEN = 0;
                INPUT_CURSOR = 0;
                DIRTY_FULL = true;
            } else {
                crate::shell::run_command_from_gui(&INPUT, INPUT_LEN);
                INPUT_LEN = 0;
                INPUT_CURSOR = 0;
                term_page_begin(output_start);
                DIRTY_FULL = true;
            }
        } else if ch == b' ' && INPUT_LEN == 0 && (TERM_PAGE_MODE || TERM_VIEW > 0) {
            term_page_next();
        } else if ch == 0x08 && INPUT_LEN == 0 && (TERM_PAGE_MODE || TERM_VIEW > 0) {
            term_page_prev();
        } else if ch == 0x08 {
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            if INPUT_CURSOR > 0 {
                let remove_at = INPUT_CURSOR - 1;
                let mut i = remove_at;
                while i + 1 < INPUT_LEN {
                    INPUT[i] = INPUT[i + 1];
                    i += 1;
                }
                INPUT_LEN -= 1;
                INPUT_CURSOR -= 1;
                INPUT[INPUT_LEN] = 0;
                DIRTY_WINDOW = FOCUS as i16;
            }
        } else if ch >= 32 && ch < 127 && INPUT_LEN < 63 {
            TERM_PAGE_MODE = false;
            TERM_VIEW = 0;
            let mut i = INPUT_LEN;
            while i > INPUT_CURSOR {
                INPUT[i] = INPUT[i - 1];
                i -= 1;
            }
            INPUT[INPUT_CURSOR] = ch;
            INPUT_LEN += 1;
            INPUT_CURSOR += 1;
            DIRTY_WINDOW = FOCUS as i16;
        }
    }
}

pub fn run() -> ! {
    // Stage 4 keeps display timings and the firmware-selected scanout mode
    // owned by firmware. Do not call the legacy video::modeset_to() here:
    // that path still programs Pipe A directly and bypasses the rollback-safe
    // intel_kms path. The framebuffer discovered from Multiboot is already the
    // working scanout surface, so start the desktop on that mode unchanged.
    serial::write_str("[DESKTOP] startup uses firmware-selected framebuffer ");
    serial::write_usize(graphics::width());
    serial::write_str("x");
    serial::write_usize(graphics::height());
    serial::write_str("\n");
    if graphics::enable_backbuffer() {
        serial::write_str("[DESKTOP] RAM backbuffer enabled\n");
    } else {
        serial::write_str("[DESKTOP] RAM backbuffer unavailable; direct framebuffer\n");
    }
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
    crate::alarm::alarm_init();
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

    // Automatic Winamp diagnostic: runs once at desktop startup so the user
    // can see exactly how far the real PE32 target gets without entering a command.
    terminal_write("\n======== WINAMP AUTO DIAGNOSTIC ========\n");
    let wd = crate::winamp::diagnose();
    terminal_write(if wd.file_found { "[WINAMP] FILE=FOUND\n" } else { "[WINAMP] FILE=NOT_FOUND\n" });
    terminal_write(if wd.mz_valid { "[WINAMP] MZ=VALID\n" } else { "[WINAMP] MZ=INVALID\n" });
    terminal_write("[WINAMP] HEADER_BYTES=");
    terminal_write_usize(wd.header_bytes);
    terminal_write("\n");
    terminal_write(if wd.pe_valid { "[WINAMP] PE=VALID\n" } else { "[WINAMP] PE=INVALID\n" });
    if wd.pe_valid {
        terminal_write("[WINAMP] PE_OFFSET=0x");
        term_write_hex(wd.pe_offset as usize);
        terminal_write("\n[WINAMP] MACHINE=0x");
        term_write_hex(wd.machine as usize);
        terminal_write("\n[WINAMP] SECTIONS=");
        terminal_write_usize(wd.sections as usize);
        terminal_write("\n[WINAMP] OPTIONAL_SIZE=0x");
        term_write_hex(wd.optional_size as usize);
        terminal_write("\n[WINAMP] OPTIONAL_MAGIC=0x");
        term_write_hex(wd.optional_magic as usize);
        terminal_write("\n");
    }
    terminal_write(if wd.pe32_x86 {
        "[WINAMP] FORMAT=PE32/x86\n[WINAMP] EXECUTION=NOT_ATTEMPTED\n[WINAMP] RESULT=SAFE_DIAGNOSTIC_ONLY\n"
    } else {
        "[WINAMP] FORMAT=UNKNOWN_OR_UNSUPPORTED\n[WINAMP] EXECUTION=NOT_ATTEMPTED\n[WINAMP] RESULT=VALIDATION_STOP\n"
    });
    terminal_write("========================================\n");
    terminal_write("AI: TEXT INTERFACE AUTOSTARTED; TYPE IN AETHER AI WINDOW\n");
    terminal_write("AI: MODEL BRIDGE=PENDING (NO LOCAL MODEL CLAIM)\n");
    terminal_write("\x1b[32maether>\x1b[0m ");
    ai_init();
    serial::write_str("AI_STATUS:READY\n");
    unsafe {
        DIRTY_FULL = true;
        FOCUS = 15;
        MX = (graphics::width() / 2) as i32;
        MY = (graphics::height() / 2) as i32;
    }
    render();

    let mut ai_handshake_ticks = 0u32;
    loop {
        ai_handshake_ticks = ai_handshake_ticks.wrapping_add(1);
        if ai_handshake_ticks >= 1000 {
            ai_handshake_ticks = 0;
            // Repeat READY periodically so a late TCP/serial bridge connection
            // cannot miss the one-shot startup marker.
            serial::write_str("AI_STATUS:READY\n");
        }
        ai_transport_poll();
        crate::virt_runtime::tick();
        if crate::virt_runtime::system_state_health() == crate::virt_runtime::SYSTEM_HEALTH_FAILED {
            unsafe {
                if !VIRT_CRITICAL_ALERTED {
                    VIRT_CRITICAL_ALERTED = true;
                    WINS[0].visible = true;
                    WINS[0].minimized = false;
                    bring_to_front(0);
                    terminal_write("!!! VIRT CRITICAL WARNING !!!\\n");
                    terminal_write("System health is FAILED. Immediate recovery required.\\n");
                    terminal_write("Virt has opened the terminal for operator attention.\\n");
                    DIRTY_FULL = true;
                }
            }
        } else {
            unsafe { VIRT_CRITICAL_ALERTED = false; }
        }
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
                    if FOCUS < MAX_WIN && WINS[FOCUS].visible && WINS[FOCUS].kind == WinKind::Alarm {
                        let _ = crate::alarm::alarm_mouse_move(
                            MX - WINS[FOCUS].x - 3,
                            MY - WINS[FOCUS].y - TITLE_H,
                        );
                    }
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
                let terminal_focused = unsafe {
                    FOCUS < MAX_WIN && WINS[FOCUS].visible && WINS[FOCUS].kind == WinKind::Terminal
                };
                if files_focused && (sc == 0x48 || sc == 0x50) {
                    crate::files_mgr::on_nav_key(sc);
                } else if terminal_focused {
                    match sc {
                        0x48 => { handle_special_key(0x52); }
                        0x50 => { handle_special_key(0x51); }
                        0x4B => { handle_special_key(0x50); }
                        0x4D => { handle_special_key(0x4F); }
                        _ => {
                            match sc {
                                0x49 => { term_scroll_up(); }
                                0x51 => { term_scroll_down(); }
                                _ => {
                                    ps2::scancode_to_ascii(0xE0);
                                    if let Some(ch) = ps2::scancode_to_ascii(sc) {
                                        handle_key(ch);
                                    }
                                }
                            }
                        }
                    }
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
        if crate::alarm::alarm_tick(crate::time::uptime()) {
            unsafe { DIRTY_FULL = true; }
        }

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
                    graphics::present();
                }
            }
            if DIRTY_FULL {
                CURSOR_SAVED = false;
                render();
                DIRTY_FULL = false;
                DIRTY_WINDOW = -1;
                DIRTY_CURSOR = false;
            } else if DIRTY_WINDOW >= 0 {
                let idx = DIRTY_WINDOW as usize;
                DIRTY_WINDOW = -1;
                redraw_single_window(idx);
                DIRTY_CURSOR = false;
            } else if DIRTY_CURSOR {
                cursor_save_and_draw(MX, MY);
                graphics::present();
                DIRTY_CURSOR = false;
            }
        }
        let mut d = 0u32;
        while d < 200 {
            d += 1;
        }
    }
}
