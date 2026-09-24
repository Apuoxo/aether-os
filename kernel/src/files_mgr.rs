//! Aether Explorer — native Windows 7 inspired file manager.
//! This replaces the old XP-style file window.  It is an original Aether UI,
//! not a copy of Microsoft code/assets.

use crate::fs;
use crate::graphics;
use crate::gui::{font, icon, theme};
use crate::serial;
use crate::part;
use crate::{fs_fat, fs_ntfs};

const WHITE:u32=0x00FFFFFF;
const TEXT:u32=0x001F1F1F;
const DIM:u32=0x005A5A5A;
const BORDER:u32=0x00B7B7B7;
const TOOL:u32=0x00F6F9FC;
const TOOL2:u32=0x00E8EEF5;
const GLASS:u32=0x00EEF5FC;
const BLUE:u32=0x003A72B8;
const BLUE2:u32=0x00DCEEFF;
const SELECT:u32=0x00CDE7FA;
const SIDE:u32=0x00F7F7F7;
const GREEN:u32=0x003B8D3B;
const RED:u32=0x00B03030;

pub const VIEW_COMPUTER:u8=0;
pub const VIEW_ROOT:u8=1;
pub const VIEW_PROPS:u8=2;
pub const VIEW_TEXT:u8=3;
pub const VIEW_DISK:u8=4;
pub const VIEW_FAT:u8=5;
pub const VIEW_NTFS:u8=6;

static mut VIEW:u8=VIEW_COMPUTER;
static mut SEL:i32=-1;
static mut HOVER:i32=-1;
static mut FOCUS_ADDR:bool=false;
static mut CTX:bool=false;
static mut CTX_X:i32=0;
static mut CTX_Y:i32=0;
static mut CONFIRM_DEL:bool=false;
static mut HIST:[[u8;32];8]=[[0;32];8];
static mut HIST_LEN:[usize;8]=[0;8];
static mut HIST_N:usize=0;
static mut HIST_I:usize=0;
static mut CWD:[u8;32]=[b'/',0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
static mut CWD_LEN:usize=1;
static mut NTFS_CWD_REF:u32=5;
static mut NTFS_PARENT:[u32;8]=[0;8];
static mut NTFS_DEPTH:usize=0;
static mut PREVIEW:[u8;512]=[0;512];
static mut PREVIEW_LEN:usize=0;
static mut STATUS:[u8;64]=[0;64];
static mut STATUS_LEN:usize=0;

fn log(s:&[u8]){serial::write_str("[EXPLORER] ");if let Ok(v)=core::str::from_utf8(s){serial::write_str(v);}serial::write_str("\n");}
fn status(s:&[u8]){log(s);unsafe{let mut i=0;while i<64{STATUS[i]=0;i+=1;}i=0;while i<s.len()&&i<63{STATUS[i]=s[i];i+=1;}STATUS_LEN=i;}}
fn set_root(){unsafe{CWD[0]=b'/';CWD_LEN=1;VIEW=VIEW_ROOT;SEL=-1;}}
fn push_hist(){unsafe{if HIST_N<8{let mut i=0;while i<32{HIST[HIST_N][i]=CWD[i];i+=1;}HIST_LEN[HIST_N]=CWD_LEN;HIST_N+=1;HIST_I=HIST_N;}}}
fn draw_num(x:usize,y:usize,mut n:u32){if n==0{graphics::draw_char(x,y,b'0',TEXT);return;}let mut d=[0u8;10];let mut c=0;while n>0{d[c]=(n%10)as u8+b'0';n/=10;c+=1;}while c>0{c-=1;graphics::draw_char(x+(d.len()-1-c)*0,y,d[c],TEXT);}}
fn btn(x:usize,y:usize,w:usize,label:&str,hot:bool){graphics::fill_rect(x,y,w,24,if hot{BLUE2}else{TOOL});graphics::border_rect(x,y,w,24,BORDER);graphics::draw_str(x+8,y+8,label,if hot{BLUE}else{TEXT});}

pub fn reset(){unsafe{VIEW=VIEW_COMPUTER;SEL=-1;HOVER=-1;FOCUS_ADDR=false;CTX=false;CONFIRM_DEL=false;HIST_N=0;HIST_I=0;CWD[0]=b'/';CWD_LEN=1;NTFS_CWD_REF=5;NTFS_DEPTH=0;PREVIEW_LEN=0;}status(b"Ready");}

fn draw_header(wx:usize,wy:usize,ww:usize,body_y:usize){
    // Native Aether rendering of a Windows 7-style Explorer chrome:
    // command bar, navigation buttons, breadcrumb address bar and search box.
    graphics::fill_rect(wx+3,body_y,ww-6,36,TOOL);
    graphics::fill_rect(wx+3,body_y,ww-6,2,BLUE);
    btn(wx+8,body_y+6,30,"<",false);
    btn(wx+40,body_y+6,30,">",false);
    btn(wx+72,body_y+6,30,"Up",false);
    graphics::fill_rect(wx+108,body_y+6,1,24,BORDER);
    btn(wx+116,body_y+6,62,"Organize",false);
    btn(wx+182,body_y+6,54,"Views",false);
    btn(wx+240,body_y+6,54,"Share",false);

    let ax=wx+300;
    let search_w=112usize.min(ww.saturating_sub(320));
    let aw=ww.saturating_sub(306+search_w);
    graphics::fill_rect(ax,body_y+6,aw,24,WHITE);
    graphics::border_rect(ax,body_y+6,aw,24,BORDER);
    graphics::draw_str(ax+7,body_y+14,"Computer",DIM);
    unsafe{
        if VIEW==VIEW_COMPUTER {
            graphics::draw_str(ax+76,body_y+14,">",DIM);
        } else if VIEW==VIEW_NTFS {
            graphics::draw_str(ax+76,body_y+14,">",DIM);
            graphics::draw_str(ax+88,body_y+14,"Local Disk (C:)",TEXT);
        } else if VIEW==VIEW_ROOT {
            graphics::draw_str(ax+76,body_y+14,">",DIM);
            graphics::draw_str(ax+88,body_y+14,"AetherFS (A:)",TEXT);
        } else if VIEW==VIEW_FAT {
            graphics::draw_str(ax+76,body_y+14,">",DIM);
            graphics::draw_str(ax+88,body_y+14,"Removable Disk",TEXT);
        } else {
            graphics::draw_str(ax+76,body_y+14,">",DIM);
            graphics::draw_str(ax+88,body_y+14,"Computer",TEXT);
        }
    }
    let sx=wx+ww.saturating_sub(search_w+6);
    graphics::fill_rect(sx,body_y+6,search_w,24,WHITE);
    graphics::border_rect(sx,body_y+6,search_w,24,BORDER);
    graphics::draw_str(sx+8,body_y+14,"Search",DIM);
    if search_w>70{graphics::draw_str(sx+search_w-18,body_y+14,"?",DIM);}
    let _=wy;
}

fn draw_sidebar(wx:usize,y:usize,w:usize,h:usize){
    // Explorer navigation pane: compact tree with section headers and volume icons.
    graphics::fill_rect(wx+3,y,w,h,GLASS);
    graphics::border_rect(wx+3,y,w,h,BORDER);
    graphics::draw_str(wx+14,y+14,"Favorites",BLUE);
    icon::blit(icon::IconId::Folder,wx+10,y+27,false);
    graphics::draw_str(wx+32,y+35,"Desktop",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+45,false);
    graphics::draw_str(wx+32,y+53,"Downloads",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+63,false);
    graphics::draw_str(wx+32,y+71,"Documents",TEXT);

    graphics::draw_str(wx+14,y+100,"Libraries",BLUE);
    icon::blit(icon::IconId::Folder,wx+10,y+113,false);
    graphics::draw_str(wx+32,y+121,"Documents",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+131,false);
    graphics::draw_str(wx+32,y+139,"Pictures",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+149,false);
    graphics::draw_str(wx+32,y+157,"Music",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+167,false);
    graphics::draw_str(wx+32,y+175,"Videos",TEXT);

    graphics::draw_str(wx+14,y+204,"Computer",BLUE);
    icon::blit(icon::IconId::MyComputer,wx+10,y+217,false);
    graphics::draw_str(wx+32,y+225,"Local Disk (C:)",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+235,false);
    graphics::draw_str(wx+32,y+243,"AetherFS (A:)",TEXT);

    graphics::draw_str(wx+14,y+272,"Network",BLUE);
    icon::blit(icon::IconId::MyComputer,wx+10,y+285,false);
    graphics::draw_str(wx+32,y+293,"Network",TEXT);
    let _=h;
}

fn draw_drive(wx:usize,y:usize,name:&str,sub:&str,sel:bool){
    if sel{graphics::fill_rect(wx,y,250,62,SELECT);}
    icon::blit(icon::IconId::MyComputer,wx+10,y+8,false);
    graphics::draw_str(wx+52,y+11,name,TEXT);
    graphics::draw_str(wx+52,y+28,sub,DIM);
    graphics::fill_rect(wx+52,y+45,178,8,TOOL2);
    graphics::fill_rect(wx+52,y+45,96,8,BLUE);
    graphics::border_rect(wx+52,y+45,178,8,BORDER);
}

fn draw_computer(wx:usize,y:usize,ww:usize,h:usize){
    graphics::draw_str(wx,y+8,"Computer",TEXT);
    graphics::draw_str(wx,y+30,"Hard Disk Drives",BLUE);
    draw_drive(wx,y+44,"Local Disk (C:)","NTFS — read-only",false);
    draw_drive(wx,y+112,"AetherFS (A:)","Aether RAM — read/write",false);
    graphics::draw_str(wx,y+190,"Devices with Removable Storage",BLUE);
    graphics::draw_str(wx+12,y+214,"No removable volumes mounted.",DIM);
    graphics::fill_rect(wx,y+h.saturating_sub(38),ww,1,BORDER);
    graphics::draw_str(wx,y+h.saturating_sub(26),"Select an item to see its details.",DIM);
}

fn draw_disk(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::draw_str(wx+8,y+8,"Local Disk (C:) — Partitions",TEXT);
    let n=part::count();
    if n==0{
        graphics::draw_str(wx+10,y+34,"No partitions detected.",RED);
        graphics::draw_str(wx+10,y+52,"AHCI/partition scan did not return volumes.",DIM);
        return;
    }
    graphics::fill_rect(wx,y+24,ww,22,TOOL2);
    graphics::draw_str(wx+8,y+31,"Volume",TEXT);
    graphics::draw_str(wx+190,y+31,"Filesystem",TEXT);
    graphics::draw_str(wx+300,y+31,"Size (MB)",TEXT);
    unsafe{
        let mut row=0usize;
        let mut i=0usize;
        while i<n&&row<16{
            if let Some(p)=part::get(i){
                let ry=y+48+row*24;
                if SEL==row as i32{graphics::fill_rect(wx,ry,ww,24,SELECT);}
                icon::blit(icon::IconId::MyComputer,wx+4,ry-2,false);
                graphics::draw_str(wx+34,ry+7,"Partition",TEXT);
                graphics::draw_str(wx+190,ry+7,part::type_name(p.ptype),DIM);
                draw_num(wx+300,ry+7,p.sectors/2048);
                row+=1;
            }
            i+=1;
        }
        if SEL>=0{
            let s=SEL as usize;
            if let Some(p)=part::get(s){
                graphics::draw_str(wx+8,y+h.saturating_sub(38),"Read-only: ",DIM);
                graphics::draw_str(wx+76,y+h.saturating_sub(38),part::type_name(p.ptype),TEXT);
            }
        }
    }
}
fn draw_fat(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::draw_str(wx+8,y+8,"FAT volume — read-only",TEXT);
    let n=fs_fat::entry_count();
    if n==0{graphics::draw_str(wx+10,y+34,"This volume is empty or unreadable.",DIM);return;}
    let mut i=0usize;
    while i<n&&i<32{
        if let Some(e)=fs_fat::entry(i){
            let ry=y+28+i*20;
            unsafe{if SEL==i as i32{graphics::fill_rect(wx,ry,ww,20,SELECT);}}
            if e.is_dir{icon::blit(icon::IconId::Folder,wx+4,ry-6,false);}else{icon::blit(icon::IconId::File,wx+4,ry-6,false);}
            let mut k=0;while k<e.name_len&&k<13{graphics::draw_char(wx+38+k*8,ry+6,e.name[k],TEXT);k+=1;}
            if !e.is_dir{draw_num(wx+210,ry+6,e.size);}
        }
        i+=1;
    }
}
fn draw_ntfs(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::draw_str(wx+8,y+8,"NTFS volume — read-only",TEXT);
    let n=fs_ntfs::entry_count();
    if n==0{graphics::draw_str(wx+10,y+34,"Root directory is empty or unavailable.",DIM);return;}
    let mut i=0usize;
    while i<n&&i<32{
        if let Some(e)=fs_ntfs::entry(i){
            let ry=y+28+i*20;
            unsafe{if SEL==i as i32{graphics::fill_rect(wx,ry,ww,20,SELECT);}}
            if e.is_dir{icon::blit(icon::IconId::Folder,wx+4,ry-6,false);}else{icon::blit(icon::IconId::File,wx+4,ry-6,false);}
            let mut k=0;while k<e.name_len&&k<28{graphics::draw_char(wx+38+k*8,ry+6,e.name[k],TEXT);k+=1;}
            if !e.is_dir{draw_num(wx+300,ry+6,if e.size>0xFFFF_FFFF{0xFFFF_FFFF}else{e.size as u32});}
        }
        i+=1;
    }
}
fn draw_list(wx:usize,y:usize,ww:usize,h:usize){
    if !fs::is_mounted(){graphics::draw_str(wx+10,y+10,"AetherFS is not mounted.",RED);return;}
    let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
    let n=unsafe{fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a)};
    graphics::fill_rect(wx,y,ww,h,WHITE);
    // Details header.
    graphics::fill_rect(wx,y,ww,22,TOOL2);
    graphics::draw_str(wx+8,y+7,"Name",TEXT);
    graphics::draw_str(wx+210,y+7,"Type",TEXT);
    graphics::draw_str(wx+300,y+7,"Size",TEXT);
    if n==0{graphics::draw_str(wx+10,y+34,"This folder is empty.",DIM);return;}
    unsafe{
        let mut i=0;
        while i<n{
            let ry=y+24+i*20;
            if SEL==i as i32{graphics::fill_rect(wx,ry,ww,20,SELECT);}
            if HOVER==i as i32{graphics::border_rect(wx,ry,ww,20,BLUE);}
            if a[i].is_dir{icon::blit(icon::IconId::Folder,wx+4,ry-6,false);}else{icon::blit(icon::IconId::File,wx+4,ry-6,false);}
            let col=if SEL==i as i32{BLUE}else{TEXT};
            let mut k=0;while k<a[i].name_len&&k<22{graphics::draw_char(wx+40+k*8,ry+6,a[i].name[k],col);k+=1;}
            graphics::draw_str(wx+210,ry+6,if a[i].is_dir{"Folder"}else{"File"},DIM);
            if !a[i].is_dir{draw_num(wx+300,ry+6,a[i].size);}
            i+=1;
        }
    }
}

fn draw_text(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::draw_str(wx+10,y+10,"Preview",BLUE);
    unsafe{let mut i=0;let mut col=0;let mut row=0;while i<PREVIEW_LEN{let c=PREVIEW[i];if c==b'\n'||col>70{row+=1;col=0;if c==b'\n'{i+=1;continue;}}if y+30+row*10<h+y{graphics::draw_char(wx+10+col*8,y+30+row*10,c,TEXT);}col+=1;i+=1;}}
}

fn draw_props(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);graphics::draw_str(wx+14,y+14,"Properties",BLUE);
    unsafe{if SEL<0{graphics::draw_str(wx+14,y+40,"No item selected.",DIM);return;}let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);let s=SEL as usize;if s>=n{return;}graphics::draw_str(wx+14,y+46,"Name:",DIM);let mut k=0;while k<a[s].name_len{graphics::draw_char(wx+66+k*8,y+46,a[s].name[k],TEXT);k+=1;}graphics::draw_str(wx+14,y+66,if a[s].is_dir{"Type: Folder"}else{"Type: File"},TEXT);if !a[s].is_dir{graphics::draw_str(wx+14,y+86,"Size:",DIM);draw_num(wx+56,y+86,a[s].size);}graphics::draw_str(wx+14,y+106,"Location: AetherFS (A:)",DIM);}
}

// Recreated Explorer shell: keep the visual layer self-contained and native to Aether.\nfn draw_context(){
    unsafe{
        // Windows 7-style light context menu with separators and disabled
        // read-only operations. The menu remains native Aether UI.
        let x=CTX_X as usize;
        let y=CTX_Y as usize;
        let w=218usize;
        let h=224usize;
        graphics::fill_rect(x+3,y+3,w,h,0x00202020);
        graphics::fill_rect(x,y,w,h,WHITE);
        graphics::border_rect(x,y,w,h,BORDER);
        graphics::fill_rect(x+1,y+1,3,h-2,BLUE2);
        graphics::draw_str(x+16,y+11,"Open",TEXT);
        graphics::draw_str(x+16,y+31,"Open with",TEXT);
        graphics::fill_rect(x+10,y+50,w-20,1,BORDER);
        graphics::draw_str(x+16,y+61,"Copy",TEXT);
        graphics::draw_str(x+16,y+81,"Cut",DIM);
        graphics::draw_str(x+16,y+101,"Rename",DIM);
        graphics::draw_str(x+16,y+121,"Delete",DIM);
        graphics::fill_rect(x+10,y+140,w-20,1,BORDER);
        graphics::draw_str(x+16,y+151,"Properties",TEXT);
        graphics::draw_str(x+16,y+171,"Refresh",TEXT);
        graphics::fill_rect(x+10,y+190,w-20,1,BORDER);
        graphics::draw_str(x+16,y+201,"New folder",DIM);
        graphics::draw_str(x+16,y+221,"New text file",DIM);
    }
}

pub fn draw(wx:usize,wy:usize,ww:usize,wh:usize,title_h:usize){
    let by=wy+title_h;
    graphics::fill_rect(wx+3,by,ww-6,wh.saturating_sub(title_h+3),WHITE);
    draw_header(wx,wy,ww,by);
    let sy=by+40;let sh=wh.saturating_sub(title_h+62);
    let side=150usize.min(ww/3);
    draw_sidebar(wx,sy,side,sh);
    let cx=wx+side+8;let cw=ww.saturating_sub(side+16);
    match unsafe{VIEW}{VIEW_COMPUTER=>draw_computer(cx,sy,cw,sh),VIEW_ROOT=>draw_list(cx,sy,cw,sh),VIEW_DISK=>draw_disk(cx,sy,cw,sh),VIEW_FAT=>draw_fat(cx,sy,cw,sh),VIEW_NTFS=>draw_ntfs(cx,sy,cw,sh),VIEW_TEXT=>draw_text(cx,sy,cw,sh),VIEW_PROPS=>draw_props(cx,sy,cw,sh),_=>{}}
    if unsafe{CTX}{draw_context();}
    if unsafe{CONFIRM_DEL}{draw_confirm(wx,wy,ww,wh);}
    let status_y=wy+wh-18;graphics::fill_rect(wx+3,status_y,ww-6,16,TOOL);graphics::border_rect(wx+3,status_y,ww-6,16,BORDER);unsafe{let mut i=0;while i<STATUS_LEN{graphics::draw_char(wx+9+i*8,status_y+4,STATUS[i],DIM);i+=1;}}
}


fn open_selected(){
    unsafe{
        if SEL<0{return;}
        let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);let s=SEL as usize;if s>=n{return;}
        if a[s].is_dir{let mut j=0;while j<a[s].name_len&&CWD_LEN<32{if CWD_LEN>1{CWD[CWD_LEN]=b'/';CWD_LEN+=1;}CWD[CWD_LEN]=a[s].name[j];CWD_LEN+=1;j+=1;}SEL=-1;VIEW=VIEW_ROOT;status(b"folder opened");return;}
        let mut buf=[0u8;512];if let Some(r)=fs::read_name(&a[s].name,a[s].name_len,&mut buf){let mut i=0;while i<r&&i<512{PREVIEW[i]=buf[i];i+=1;}PREVIEW_LEN=r;VIEW=VIEW_TEXT;status(b"Preview opened");}else{status(b"Cannot open file");}
    }
}
fn delete_selected(){unsafe{if SEL<0{return;}let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];let n=fs::list_ex(&mut a);let s=SEL as usize;if s<n&&fs::delete_name(&a[s].name,a[s].name_len){SEL=-1;status(b"Item deleted");}else{status(b"Delete failed");}}}
fn new_folder(){if fs::mkdir("/New Folder"){status(b"New folder created");}else{status(b"Cannot create folder");}}
fn new_file(){if fs::create("/New Text Document.txt"){let _=fs::write("/New Text Document.txt",b"");status(b"Text document created");}else{status(b"Cannot create file");}}
fn rename_selected(){unsafe{if SEL<0{return;}let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];let n=fs::list_ex(&mut a);let s=SEL as usize;if s<n&&fs::rename(core::str::from_utf8_unchecked(core::slice::from_raw_parts(a[s].name.as_ptr(),a[s].name_len)),"Renamed Item"){SEL=-1;status(b"Renamed to Renamed Item");}else{status(b"Rename failed");}}}

pub fn on_click(wx:i32,wy:i32,ww:i32,wh:i32,title_h:i32,mx:i32,my:i32,right:bool)->bool{
    let by=wy+title_h;let relx=mx-wx;let rely=my-by;
    unsafe{
        if CONFIRM_DEL{let cx=ww/2;let cy=wh/2;if my>wy+cy+20&&my<wy+cy+60{if mx<wx+cx{delete_selected();}CONFIRM_DEL=false;}return true;}
        if CTX{
            if mx>=CTX_X&&mx<CTX_X+218&&my>=CTX_Y&&my<CTX_Y+224{
                let r=(my-CTX_Y)/20;
                match r{
                    0=>open_selected(),
                    1=>status(b"Open with is not implemented"),
                    2=>status(b"Copy is not implemented"),
                    3=>status(b"Cut unavailable on read-only volume"),
                    4=>status(b"Rename unavailable on read-only volume"),
                    5=>status(b"Delete unavailable on read-only volume"),
                    6=>{VIEW=VIEW_PROPS;status(b"Properties");},
                    7=>{if VIEW==VIEW_NTFS{let _=fs_ntfs::list_directory(NTFS_CWD_REF);}else{let _=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut [fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16]);}SEL=-1;status(b"Refreshed");},
                    8=>status(b"New folder unavailable on read-only volume"),
                    9=>status(b"New file unavailable on read-only volume"),
                    _=>{}
                }
                CTX=false;
                return true;
            }
            CTX=false;
            return true;
        }
        if right{
            let side=150i32.min(ww/3);
            let list_top=by+40;
            if mx>=wx+side+8&&my>=list_top{
                if VIEW==VIEW_NTFS{
                    let n=fs_ntfs::entry_count();
                    if my>=list_top+28{
                        let row=((my-list_top-28)/20)as i32;
                        if row>=0&&row<n as i32{SEL=row;}
                    }
                }else if VIEW==VIEW_ROOT{
                    let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
                    let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);
                    if my>=list_top+24{
                        let row=((my-list_top-24)/20)as i32;
                        if row>=0&&row<n as i32{SEL=row;}
                    }
                }
            }
            CTX=true;
            CTX_X=mx;
            CTX_Y=my;
            log(b"context menu opened");
            return true;
        }
        if rely>=5&&rely<35{
            if relx<44{go_back();}else if relx<80{go_forward();}else if relx<116{go_up();}else if relx<182{status(b"Organize");}else if relx<244{status(b"Views: Details");}else if relx<310{status(b"Share");}
            return true;
        }
        let side=150i32.min(ww/3);let list_top=by+40;
        if mx>=wx+side+8&&my>=list_top{
            let cx=mx-(wx+side+8);let _=cx;
            if VIEW==VIEW_COMPUTER{
                if my>=list_top+42&&my<list_top+98{
                    log(b"Local Disk (C:) open requested");
                    if fs_ntfs::mount_first(){
                        SEL=-1;
                        VIEW=VIEW_NTFS;
                        status(b"Local Disk (C:) opened - NTFS read-only");
                    }else{
                        status(b"Local Disk (C:) mount failed");
                    }
                    return true;
                }
                if my>=list_top+98&&my<list_top+154{
                    push_hist();
                    set_root();
                    log(b"AetherFS (A:) opened");
                    status(b"AetherFS (A:) opened");
                    return true;
                }
            }
            else if VIEW==VIEW_DISK{
                let n=part::count();
                if my<list_top+48{return false;}
                let row=((my-list_top-48)/24)as i32;
                if row>=0&&row<n as i32{
                    if SEL==row{
                        if let Some(p)=part::get(row as usize){
                            log(b"partition open requested");
                            if p.ptype==0x07{
                                if fs_ntfs::mount_partition(row as usize){
                                    VIEW=VIEW_NTFS;SEL=-1;
                                    status(b"NTFS root opened read-only");
                                }else{status(b"NTFS mount failed - read-only");}
                            }else if p.ptype==0x0B||p.ptype==0x0C||p.ptype==0x06||p.ptype==0x0E||p.ptype==0x04{
                                if fs_fat::mount_partition(row as usize){
                                    VIEW=VIEW_FAT;SEL=-1;
                                    status(b"FAT root opened read-only");
                                }else{status(b"FAT mount failed - read-only");}
                            }else{
                                status(b"Partition filesystem not implemented");
                            }
                        }
                    }else{
                        SEL=row;
                        status(b"Partition selected");
                    }
                    return true;
                }
                return false;
            }
            else if VIEW==VIEW_NTFS{
                let n=fs_ntfs::entry_count();
                if my<list_top+28{return false;}
                let row=((my-list_top-28)/20)as i32;
                if row>=0&&row<n as i32{
                    if SEL==row{
                        if let Some(e)=fs_ntfs::entry(row as usize){
                            if e.is_dir{
                                if fs_ntfs::list_directory(e.mft_ref){
                                    if NTFS_DEPTH<8{
                                        NTFS_PARENT[NTFS_DEPTH]=NTFS_CWD_REF;
                                        NTFS_DEPTH+=1;
                                    }
                                    NTFS_CWD_REF=e.mft_ref;
                                    SEL=-1;
                                    status(b"NTFS folder opened");
                                }else{
                                    status(b"NTFS folder read failed");
                                }
                            }else{
                                status(b"NTFS file preview not implemented");
                            }
                        }
                    }else{
                        SEL=row;
                        status(b"Selected");
                    }
                    return true;
                }
                return false;
            }
            else if VIEW==VIEW_ROOT{
                let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
                let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);
                if my<list_top+24{return false;}
                let row=((my-list_top-24)/20)as i32;
                if row>=0&&row<n as i32{
                    if SEL==row{
                        log(b"open selected");
                        open_selected();
                    }else{
                        SEL=row;
                        log(b"selection changed");
                        status(b"Selected");
                    }
                    return true;
                }
                // Empty list area: no redraw.
                return false;
            }
            return true;
        }
        if my>=list_top&&my<list_top+330&&mx<wx+side{
            // Sidebar: only implemented destinations change state.
            if my>=list_top+220&&my<list_top+260{
                push_hist();
                set_root();
                status(b"AetherFS (A:) opened");
                return true;
            }
            return false;
        }
        if VIEW==VIEW_TEXT||VIEW==VIEW_PROPS{VIEW=VIEW_ROOT;return true;}
    }
    false
}
fn go_back(){unsafe{if HIST_I>0{HIST_I-=1;let mut i=0;while i<32{CWD[i]=HIST[HIST_I][i];i+=1;}CWD_LEN=HIST_LEN[HIST_I];VIEW=VIEW_ROOT;SEL=-1;status(b"Back");}else{VIEW=VIEW_COMPUTER;SEL=-1;status(b"Computer");}}}
fn go_forward(){unsafe{if HIST_I+1<HIST_N{HIST_I+=1;VIEW=VIEW_ROOT;SEL=-1;status(b"Forward");}}}
fn go_up(){
    unsafe{
        if VIEW==VIEW_NTFS{
            if NTFS_DEPTH>0{
                NTFS_DEPTH-=1;
                NTFS_CWD_REF=NTFS_PARENT[NTFS_DEPTH];
                if fs_ntfs::list_directory(NTFS_CWD_REF){
                    SEL=-1;
                    status(b"Up");
                }else{
                    status(b"NTFS parent read failed");
                }
            }else{
                VIEW=VIEW_COMPUTER;
                SEL=-1;
                NTFS_CWD_REF=5;
                status(b"Computer");
            }
            return;
        }
        if VIEW!=VIEW_ROOT{
            VIEW=VIEW_COMPUTER;
            SEL=-1;
            CWD[0]=b'/';
            CWD_LEN=1;
            status(b"Computer");
            return;
        }
        if CWD_LEN<=1{
            VIEW=VIEW_COMPUTER;
            SEL=-1;
            status(b"Computer");
            return;
        }
        let mut i=CWD_LEN;
        while i>1{
            i-=1;
            if CWD[i]==b'/'{
                CWD_LEN=if i==0{1}else{i};
                if CWD_LEN==0{CWD_LEN=1;}
                SEL=-1;
                VIEW=VIEW_ROOT;
                status(b"Up");
                return;
            }
        }
        CWD[0]=b'/';
        CWD_LEN=1;
        SEL=-1;
        status(b"Up");
    }
}
