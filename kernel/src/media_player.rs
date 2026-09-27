//! Native Aether media-player state machine.
//! The UI and decoder boundary are intentionally independent from HDA.
//! Current decoder: bounded RIFF/WAVE PCM (8/16-bit, mono/stereo).
//! Compressed decoders plug into the same PCM producer interface.

use crate::fs;
use crate::serial;

const MAX_PATH: usize = 96;
const PCM_BUF: usize = 32768;
const MAX_PLAYLIST: usize = 16;

#[derive(Clone, Copy, PartialEq)]
pub enum State { Empty, Stopped, Playing, Paused, Error }

static mut STATE: State = State::Empty;
static mut PATH: [u8; MAX_PATH] = [0; MAX_PATH];
static mut PATH_LEN: usize = 0;
static mut TITLE: [u8; MAX_PATH] = [0; MAX_PATH];
static mut TITLE_LEN: usize = 0;
static mut SAMPLE_RATE: u32 = 0;
static mut CHANNELS: u16 = 0;
static mut BITS: u16 = 0;
static mut DATA_OFF: usize = 0;
static mut DATA_LEN: usize = 0;
static mut PCM_POS: usize = 0;
static mut VOLUME: u8 = 100;
static mut MUTED: bool = false;
static mut REPEAT: bool = false;
static mut SHUFFLE: bool = false;
static mut LAST_ERROR: u8 = 0;
static mut PCM: [u8; PCM_BUF] = [0; PCM_BUF];
static mut PLAYLIST: [[u8; MAX_PATH]; MAX_PLAYLIST] = [[0; MAX_PATH]; MAX_PLAYLIST];
static mut PL_LEN: [usize; MAX_PLAYLIST] = [0; MAX_PLAYLIST];
static mut PL_COUNT: usize = 0;
static mut PL_INDEX: usize = 0;

fn le16(b: &[u8], p: usize) -> u16 { (b[p] as u16) | ((b[p+1] as u16) << 8) }
fn le32(b: &[u8], p: usize) -> u32 {
    (b[p] as u32) | ((b[p+1] as u32) << 8) | ((b[p+2] as u32) << 16) | ((b[p+3] as u32) << 24)
}
fn copy_path(dst: &mut [u8], src: &[u8]) -> usize {
    let n = if src.len() > dst.len() { dst.len() } else { src.len() };
    let mut i=0; while i<n { dst[i]=src[i]; i+=1; } n
}
fn wav_parse(buf: &[u8]) -> Option<(u32,u16,u16,usize,usize)> {
    if buf.len() < 12 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" { return None; }
    let mut p=12usize; let mut rate=0u32; let mut ch=0u16; let mut bits=0u16;
    let mut data=0usize; let mut len=0usize; let mut have_fmt=false; let mut have_data=false;
    while p + 8 <= buf.len() {
        let sz=le32(buf,p+4) as usize; let body=p+8;
        if body > buf.len() { return None; }
        let end=body.saturating_add(sz);
        if end > buf.len() { return None; }
        if &buf[p..p+4] == b"fmt " && sz >= 16 {
            if le16(buf,body) != 1 { return None; }
            ch=le16(buf,body+2); rate=le32(buf,body+4); bits=le16(buf,body+14);
            if (ch != 1 && ch != 2) || (bits != 8 && bits != 16) || rate == 0 { return None; }
            have_fmt=true;
        } else if &buf[p..p+4] == b"data" {
            data=body; len=sz; have_data=true;
        }
        p=end + (sz & 1);
    }
    if have_fmt && have_data { Some((rate,ch,bits,data,len)) } else { None }
}

pub fn init() {
    unsafe {
        STATE=State::Empty; PATH_LEN=0; TITLE_LEN=0; PL_COUNT=0; PL_INDEX=0;
        VOLUME=100; MUTED=false; REPEAT=false; SHUFFLE=false; LAST_ERROR=0;
    }
    serial::write_str("[MEDIA] native player core ready\n");
}

pub fn state() -> State { unsafe { STATE } }
pub fn volume() -> u8 { unsafe { VOLUME } }
pub fn muted() -> bool { unsafe { MUTED } }
pub fn repeat() -> bool { unsafe { REPEAT } }
pub fn shuffle() -> bool { unsafe { SHUFFLE } }
pub fn sample_rate() -> u32 { unsafe { SAMPLE_RATE } }
pub fn channels() -> u16 { unsafe { CHANNELS } }
pub fn bits() -> u16 { unsafe { BITS } }
pub fn position_bytes() -> usize { unsafe { PCM_POS } }
pub fn data_bytes() -> usize { unsafe { DATA_LEN } }
pub fn error() -> u8 { unsafe { LAST_ERROR } }

pub fn title(out: &mut [u8]) -> usize { unsafe { copy_path(out,&TITLE[..TITLE_LEN]) } }
pub fn path(out: &mut [u8]) -> usize { unsafe { copy_path(out,&PATH[..PATH_LEN]) } }

pub fn open(path: &str) -> bool {
    let mut buf=[0u8; PCM_BUF];
    let n=match fs::read_large(path,&mut buf) { Some(n)=>n, None=>{unsafe{STATE=State::Error;LAST_ERROR=1;}return false;} };
    let (rate,ch,bits,data,len)=match wav_parse(&buf[..n]) {
        Some(v)=>v, None=>{unsafe{STATE=State::Error;LAST_ERROR=2;}return false;}
    };
    unsafe {
        let mut i=0; while i<n { PCM[i]=buf[i]; i+=1; }
        PATH_LEN=copy_path(&mut PATH,path.as_bytes());
        TITLE_LEN=PATH_LEN; TITLE[..TITLE_LEN].copy_from_slice(&PATH[..PATH_LEN]);
        SAMPLE_RATE=rate; CHANNELS=ch; BITS=bits; DATA_OFF=data; DATA_LEN=len;
        PCM_POS=0; LAST_ERROR=0; STATE=State::Stopped;
    }
    serial::write_str("[MEDIA] WAV opened\n");
    true
}

pub fn play() { unsafe { if DATA_LEN != 0 && (STATE==State::Stopped || STATE==State::Paused) { STATE=State::Playing; } } }
pub fn pause() { unsafe { if STATE==State::Playing { STATE=State::Paused; } } }
pub fn stop() { unsafe { if DATA_LEN != 0 { PCM_POS=0; STATE=State::Stopped; } } }
pub fn toggle_play() { unsafe { if STATE==State::Playing { STATE=State::Paused; } else if DATA_LEN != 0 { STATE=State::Playing; } } }

pub fn seek_permille(v: u16) {
    unsafe {
        if DATA_LEN == 0 { return; }
        let x=if v>1000{1000}else{v as usize};
        let bytes_per_frame=((CHANNELS as usize)*(BITS as usize))/8;
        if bytes_per_frame==0 { return; }
        PCM_POS=((DATA_LEN*x)/1000 / bytes_per_frame)*bytes_per_frame;
    }
}
pub fn set_volume(v: u8) { unsafe { VOLUME=v.min(100); } }
pub fn volume_up() { unsafe { VOLUME=VOLUME.saturating_add(5).min(100); } }
pub fn volume_down() { unsafe { VOLUME=VOLUME.saturating_sub(5); } }
pub fn toggle_mute() { unsafe { MUTED=!MUTED; } }
pub fn toggle_repeat() { unsafe { REPEAT=!REPEAT; } }
pub fn toggle_shuffle() { unsafe { SHUFFLE=!SHUFFLE; } }

pub fn add_to_playlist(path: &str) -> bool {
    unsafe {
        if PL_COUNT >= MAX_PLAYLIST { return false; }
        PL_LEN[PL_COUNT]=copy_path(&mut PLAYLIST[PL_COUNT],path.as_bytes());
        PL_COUNT+=1; true
    }
}
pub fn playlist_count() -> usize { unsafe { PL_COUNT } }
pub fn next() -> bool {
    unsafe {
        if PL_COUNT==0 { return false; }
        if SHUFFLE { PL_INDEX=(PL_INDEX+3)%PL_COUNT; }
        else if PL_INDEX+1<PL_COUNT { PL_INDEX+=1; }
        else if REPEAT { PL_INDEX=0; } else { stop(); return false; }
        let n=PL_LEN[PL_INDEX]; let mut p=[0u8;MAX_PATH]; let mut i=0; while i<n {p[i]=PLAYLIST[PL_INDEX][i];i+=1;}
        // ASCII/UTF-8 path is preserved; current shell/VFS API is UTF-8.
        false
    }
}

pub fn pcm_slice() -> (&'static [u8], usize) {
    unsafe {
        if DATA_OFF+PCM_POS >= PCM_BUF { return (&PCM,0); }
        let remain=DATA_LEN.saturating_sub(PCM_POS);
        let n=if remain > PCM_BUF-DATA_OFF-PCM_POS { PCM_BUF-DATA_OFF-PCM_POS } else { remain };
        (&PCM[DATA_OFF+PCM_POS..DATA_OFF+PCM_POS+n],n)
    }
}
