//! Native Aether media player core.
//! Streaming VFS reader + bounded PCM buffer. The decoder boundary is format
//! aware; WAV/PCM is the first native decoder and feeds the future HDA backend.

use crate::fs;
use crate::serial;
use crate::media_builtin;

const MAX_PATH: usize = 96;
const HEADER_BUF: usize = 4096;
const PCM_BUF: usize = 32768;
const MAX_PLAYLIST: usize = 16;

#[derive(Clone, Copy, PartialEq)]
pub enum State { Empty, Stopped, Playing, Paused, Error }

#[derive(Clone, Copy, PartialEq)]
pub enum Format { Unknown, WavPcm }

static mut STATE: State = State::Empty;
static mut FORMAT: Format = Format::Unknown;
static mut PATH: [u8; MAX_PATH] = [0; MAX_PATH];
static mut PATH_LEN: usize = 0;
static mut TITLE: [u8; MAX_PATH] = [0; MAX_PATH];
static mut TITLE_LEN: usize = 0;
static mut FILE_SIZE: usize = 0;
static mut SAMPLE_RATE: u32 = 0;
static mut CHANNELS: u16 = 0;
static mut BITS: u16 = 0;
static mut DATA_OFF: usize = 0;
static mut DATA_LEN: usize = 0;
static mut PCM_FILE_POS: usize = 0;
static mut PCM_READY: usize = 0;
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
static mut BUILTIN_ACTIVE: u8 = 0;

fn le16(b: &[u8], p: usize) -> u16 { (b[p] as u16) | ((b[p+1] as u16) << 8) }
fn le32(b: &[u8], p: usize) -> u32 {
    (b[p] as u32) | ((b[p+1] as u32) << 8) | ((b[p+2] as u32) << 16) | ((b[p+3] as u32) << 24)
}
fn copy_bytes(dst: &mut [u8], src: &[u8]) -> usize {
    let n=dst.len().min(src.len()); let mut i=0; while i<n { dst[i]=src[i]; i+=1; } n
}
fn four(b:&[u8],p:usize,s:&[u8;4])->bool { p+4<=b.len() && b[p..p+4]==s[..] }

fn parse_wav_bytes(b:&[u8])->Option<(usize,u32,u16,u16,usize,usize)> {
    if b.len()<12 || !four(b,0,b"RIFF") || !four(b,8,b"WAVE") { return None; }
    let mut p=12usize; let mut rate=0u32; let mut ch=0u16; let mut bits=0u16;
    let mut data_off=0usize; let mut data_len=0usize; let mut fmt=false;
    while p+8<=b.len() {
        let sz=le32(b,p+4) as usize; let body=p+8;
        let end=body.checked_add(sz)?;
        if end>b.len() { return None; }
        if four(b,p,b"fmt ") && sz>=16 {
            if le16(b,body)!=1 { return None; }
            ch=le16(b,body+2); rate=le32(b,body+4); bits=le16(b,body+14);
            if rate==0 || (ch!=1 && ch!=2) || (bits!=8 && bits!=16) { return None; }
            fmt=true;
        } else if four(b,p,b"data") {
            data_off=body; data_len=sz.min(b.len().saturating_sub(body));
            let frame=((ch as usize)*(bits as usize))/8;
            if frame==0 { return None; }
            data_len-=data_len%frame;
            return if fmt { Some((b.len(),rate,ch,bits,data_off,data_len)) } else { None };
        }
        p=end+(sz&1);
    }
    None
}

pub fn builtin_count()->usize { 3 }
pub fn builtin_name(i:usize)->&'static str {
    match i { 0=>"TEST.WAV", 1=>"TEST.MP3", 2=>"TEST.OGG", _=>"" }
}
pub fn builtin_kind(i:usize)->&'static str {
    match i { 0=>"WAV PCM", 1=>"MP3", 2=>"OGG Vorbis", _=>"" }
}
pub fn builtin_bytes(i:usize)->&'static [u8] {
    match i { 0=>media_builtin::TEST_WAV, 1=>media_builtin::TEST_MP3, 2=>media_builtin::TEST_OGG, _=>&[] }
}
pub fn builtin_path(i:usize)->&'static str {
    match i { 0=>"/MEDIA/TEST.WAV", 1=>"/MEDIA/TEST.MP3", 2=>"/MEDIA/TEST.OGG", _=>"" }
}

pub fn open_builtin(i:usize)->bool {
    let p=builtin_path(i);
    if p.is_empty(){return false;}
    add_to_playlist(p);
    if open(p){
        serial::write_str("[MEDIA] opened AetherFS media file\n");
        true
    } else {
        if i==1 || i==2 {
            serial::write_str("[MEDIA] file is real AetherFS media; decoder pending\n");
        }
        false
    }
}

fn parse_wav(path:&str)->Option<(usize,u32,u16,u16,usize,usize)> {
    let size=fs::file_size(path)?;
    if size<12 { return None; }
    let mut h=[0u8;HEADER_BUF];
    let n=fs::read_range(path,0,&mut h)?;
    if n<12 || !four(&h,0,b"RIFF") || !four(&h,8,b"WAVE") { return None; }
    let mut p=12usize; let mut rate=0u32; let mut ch=0u16; let mut bits=0u16;
    let mut data_off=0usize; let mut data_len=0usize; let mut fmt=false;
    while p+8<=n {
        let sz=le32(&h,p+4) as usize; let body=p+8;
        let end=body.checked_add(sz)?;
        if end>size || end>n { 
            // Header chunks must be contained in the bounded header buffer.
            // If the data chunk starts beyond the header window, reject it
            // instead of reading unbounded metadata.
            return None;
        }
        if four(&h,p,b"fmt ") && sz>=16 {
            if le16(&h,body)!=1 { return None; }
            ch=le16(&h,body+2); rate=le32(&h,body+4); bits=le16(&h,body+14);
            if rate==0 || (ch!=1 && ch!=2) || (bits!=8 && bits!=16) { return None; }
            fmt=true;
        } else if four(&h,p,b"data") {
            data_off=body; data_len=sz; 
            if data_off.checked_add(data_len)? > size { return None; }
            data_len=(size-data_off).min(data_len);
            data_len-=data_len%(((ch as usize)*(bits as usize))/8).max(1);
            return if fmt { Some((size,rate,ch,bits,data_off,data_len)) } else { None };
        }
        p=end+(sz&1);
    }
    None
}

pub fn init() {
    unsafe {
        STATE=State::Empty; FORMAT=Format::Unknown; PATH_LEN=0; TITLE_LEN=0;
        FILE_SIZE=0; SAMPLE_RATE=0; CHANNELS=0; BITS=0; DATA_OFF=0; DATA_LEN=0;
        PCM_FILE_POS=0; PCM_READY=0; VOLUME=100; MUTED=false;
        REPEAT=false; SHUFFLE=false; LAST_ERROR=0; PL_COUNT=0; PL_INDEX=0; BUILTIN_ACTIVE=0;
    }
    serial::write_str("[MEDIA] native streaming player core ready\n");
}

pub fn state()->State { unsafe{STATE} }
pub fn format()->Format { unsafe{FORMAT} }
pub fn volume()->u8 { unsafe{VOLUME} }
pub fn muted()->bool { unsafe{MUTED} }
pub fn repeat()->bool { unsafe{REPEAT} }
pub fn shuffle()->bool { unsafe{SHUFFLE} }
pub fn sample_rate()->u32 { unsafe{SAMPLE_RATE} }
pub fn channels()->u16 { unsafe{CHANNELS} }
pub fn bits()->u16 { unsafe{BITS} }
pub fn position_bytes()->usize { unsafe{PCM_FILE_POS} }
pub fn data_bytes()->usize { unsafe{DATA_LEN} }
pub fn file_size()->usize { unsafe{FILE_SIZE} }
pub fn error()->u8 { unsafe{LAST_ERROR} }
pub fn title(out:&mut [u8])->usize { unsafe{copy_bytes(out,&TITLE[..TITLE_LEN])} }
pub fn path(out:&mut [u8])->usize { unsafe{copy_bytes(out,&PATH[..PATH_LEN])} }

pub fn open(path:&str)->bool {
    let (size,rate,ch,bits,data,len)=match parse_wav(path) {
        Some(v)=>v,
        None=>{unsafe{STATE=State::Error;FORMAT=Format::Unknown;LAST_ERROR=2;} serial::write_str("[MEDIA] unsupported or malformed WAV\n"); return false;}
    };
    unsafe {
        PATH_LEN=copy_bytes(&mut PATH,path.as_bytes());
        TITLE_LEN=PATH_LEN.min(TITLE.len());
        let mut i=0; while i<TITLE_LEN { TITLE[i]=PATH[i]; i+=1; }
        FILE_SIZE=size; SAMPLE_RATE=rate; CHANNELS=ch; BITS=bits;
        DATA_OFF=data; DATA_LEN=len; PCM_FILE_POS=0; PCM_READY=0;
        FORMAT=Format::WavPcm; LAST_ERROR=0; STATE=State::Stopped;
    }
    serial::write_str("[MEDIA] WAV PCM opened\n");
    true
}

pub fn refill_pcm()->usize {
    unsafe {
        if FORMAT!=Format::WavPcm || DATA_LEN==0 || PCM_FILE_POS>=DATA_LEN { PCM_READY=0; return 0; }
        let frame=((CHANNELS as usize)*(BITS as usize))/8;
        if frame==0 { return 0; }
        let remain=DATA_LEN-PCM_FILE_POS;
        let n=PCM_BUF.min(remain);
        let n=n-(n%frame);
        if n==0 { PCM_READY=0; return 0; }
        let got = fs::read_range(
            core::str::from_utf8_unchecked(&PATH[..PATH_LEN]),
            DATA_OFF + PCM_FILE_POS,
            &mut PCM[..n]
        );
        match got {
            Some(got) if got>0 => { PCM_READY=got; got },
            _ => { PCM_READY=0; LAST_ERROR=3; STATE=State::Error; 0 }
        }
    }
}

pub fn pcm_buffer(out:&mut [u8])->usize {
    unsafe {
        let n=out.len().min(PCM_READY); let mut i=0; while i<n {out[i]=PCM[i];i+=1;} n
    }
}

/// Called by the PCM sink after it has consumed n bytes.
pub fn consume_pcm(n:usize) {
    unsafe {
        let n=n.min(PCM_READY); PCM_READY-=n; PCM_FILE_POS+=n;
        if PCM_READY==0 && PCM_FILE_POS<DATA_LEN { let _=refill_pcm(); }
        if PCM_FILE_POS>=DATA_LEN {
            if REPEAT { PCM_FILE_POS=0; PCM_READY=0; let _=refill_pcm(); }
            else { STATE=State::Stopped; PCM_READY=0; }
        }
    }
}

pub fn play(){unsafe{if DATA_LEN>0 && (STATE==State::Stopped||STATE==State::Paused){STATE=State::Playing;let _=refill_pcm();}}}
pub fn pause(){unsafe{if STATE==State::Playing{STATE=State::Paused;}}}
pub fn stop(){unsafe{if DATA_LEN>0{PCM_FILE_POS=0;PCM_READY=0;STATE=State::Stopped;}}}
pub fn toggle_play(){unsafe{if STATE==State::Playing{STATE=State::Paused}else if DATA_LEN>0{STATE=State::Playing;let _=refill_pcm();}}}

pub fn seek_permille(v:u16){
    unsafe{
        if DATA_LEN==0{return;}
        let x=(v.min(1000) as usize);
        let frame=((CHANNELS as usize)*(BITS as usize))/8;
        if frame==0{return;}
        PCM_FILE_POS=((DATA_LEN*x)/1000/frame)*frame;
        PCM_READY=0;
        if STATE==State::Playing { let _=refill_pcm(); }
    }
}
pub fn set_volume(v:u8){unsafe{VOLUME=v.min(100);}}
pub fn volume_up(){unsafe{VOLUME=VOLUME.saturating_add(5).min(100);}}
pub fn volume_down(){unsafe{VOLUME=VOLUME.saturating_sub(5);}}
pub fn toggle_mute(){unsafe{MUTED=!MUTED;}}
pub fn toggle_repeat(){unsafe{REPEAT=!REPEAT;}}
pub fn toggle_shuffle(){unsafe{SHUFFLE=!SHUFFLE;}}

pub fn add_to_playlist(path:&str)->bool{
    unsafe{if PL_COUNT>=MAX_PLAYLIST{return false;}PL_LEN[PL_COUNT]=copy_bytes(&mut PLAYLIST[PL_COUNT],path.as_bytes());PL_COUNT+=1;true}
}
pub fn playlist_count()->usize{unsafe{PL_COUNT}}
pub fn next()->bool{
    unsafe{
        if PL_COUNT==0{return false;}
        if SHUFFLE{PL_INDEX=(PL_INDEX.wrapping_mul(7).wrapping_add(3))%PL_COUNT}
        else if PL_INDEX+1<PL_COUNT{PL_INDEX+=1}
        else if REPEAT{PL_INDEX=0}
        else{return false}
        let n=PL_LEN[PL_INDEX]; let mut p=[0u8;MAX_PATH]; let mut i=0; while i<n{p[i]=PLAYLIST[PL_INDEX][i];i+=1;}
        if let Ok(s)=core::str::from_utf8(&p[..n]) { open(s) } else { false }
    }
}
