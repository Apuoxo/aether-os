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
fn put_str(b:&mut [u8;64],p:&mut usize,s:&[u8]){let mut i=0;while i<s.len()&&*p<63{b[*p]=s[i];*p+=1;i+=1;}}
fn put_num(b:&mut [u8;64],p:&mut usize,mut n:usize){let mut d=[0u8;10];let mut c=0;if n==0{d[0]=b'0';c=1;}while n>0&&c<10{d[c]=(n%10)as u8+b'0';n/=10;c+=1;}while c>0&&*p<63{c-=1;b[*p]=d[c];*p+=1;}}
fn trace_ntfs(tag:&[u8]){let n=fs_ntfs::entry_count();let mut b=[0u8;64];let mut p=0usize;put_str(&mut b,&mut p,tag);put_str(&mut b,&mut p,b" n=");put_num(&mut b,&mut p,n);let mut i=0usize;while i<3&&i<n{if let Some(e)=fs_ntfs::entry(i){put_str(&mut b,&mut p,b" ");put_num(&mut b,&mut p,i);put_str(&mut b,&mut p,b"=");let mut k=0;while k<e.name_len&&k<12&&p<62{put_str(&mut b,&mut p,&e.name[k..k+1]);k+=1;}}i+=1;}status(&b[..p]);}
fn trace_partition_click(mx:i32,my:i32,row:i32,part_no:i32){let mut b=[0u8;64];let mut p=0usize;put_str(&mut b,&mut p,b"PCLICK mx=");put_num(&mut b,&mut p,mx.max(0) as usize);put_str(&mut b,&mut p,b" my=");put_num(&mut b,&mut p,my.max(0) as usize);put_str(&mut b,&mut p,b" row=");put_num(&mut b,&mut p,row.max(0) as usize);put_str(&mut b,&mut p,b" part=");put_num(&mut b,&mut p,part_no.max(0) as usize);status(&b[..p]);}
const ROW_H:i32=24;
const ROW_TOP:i32=30;
fn hit_row(list_top:i32,my:i32)->i32{if my<list_top+ROW_TOP{-1}else{(my-list_top-ROW_TOP)/ROW_H}}
fn set_root(){unsafe{CWD[0]=b'/';CWD_LEN=1;VIEW=VIEW_ROOT;SEL=-1;}}
fn push_hist(){unsafe{if HIST_N<8{let mut i=0;while i<32{HIST[HIST_N][i]=CWD[i];i+=1;}HIST_LEN[HIST_N]=CWD_LEN;HIST_N+=1;HIST_I=HIST_N;}}}
fn draw_num(x:usize,y:usize,mut n:u32){if n==0{graphics::draw_char(x,y,b'0',TEXT);return;}let mut d=[0u8;10];let mut c=0;while n>0{d[c]=(n%10)as u8+b'0';n/=10;c+=1;}let start=c;while c>0{c-=1;graphics::draw_char(x+(start-1-c)*8,y,d[c],TEXT);}}
fn btn(x:usize,y:usize,w:usize,label:&str,hot:bool){graphics::fill_rect(x,y,w,24,if hot{BLUE2}else{TOOL});graphics::border_rect(x,y,w,24,BORDER);graphics::draw_str(x+8,y+8,label,if hot{BLUE}else{TEXT});}

pub fn reset(){unsafe{VIEW=VIEW_COMPUTER;SEL=-1;HOVER=-1;FOCUS_ADDR=false;CTX=false;CONFIRM_DEL=false;HIST_N=0;HIST_I=0;CWD[0]=b'/';CWD_LEN=1;NTFS_CWD_REF=5;NTFS_DEPTH=0;PREVIEW_LEN=0;}status(b"Ready");}

fn draw_header(wx:usize,wy:usize,ww:usize,body_y:usize){
    // Modern, compact Aether Explorer chrome. Designed for the real 800x600 target.
    graphics::fill_rect(wx+3,body_y,ww-6,34,TOOL);
    graphics::fill_rect(wx+3,body_y,ww-6,1,BLUE);

    btn(wx+8,body_y+5,28,"<",false);
    btn(wx+38,body_y+5,28,">",false);
    btn(wx+68,body_y+5,32,"Up",false);

    let ax=wx+106;
    let search_w=126usize.min(ww.saturating_sub(250));
    let aw=ww.saturating_sub(118+search_w);
    graphics::fill_rect(ax,body_y+5,aw,24,WHITE);
    graphics::border_rect(ax,body_y+5,aw,24,BORDER);
    graphics::draw_str(ax+8,body_y+13,"Computer",DIM);
    unsafe{
        if VIEW==VIEW_NTFS{
            graphics::draw_str(ax+76,body_y+13,">",DIM);
            graphics::draw_str(ax+88,body_y+13,"Local Disk (C:)",TEXT);
        }else if VIEW==VIEW_ROOT{
            graphics::draw_str(ax+76,body_y+13,">",DIM);
            graphics::draw_str(ax+88,body_y+13,"AetherFS (A:)",TEXT);
        }else if VIEW==VIEW_FAT{
            graphics::draw_str(ax+76,body_y+13,">",DIM);
            graphics::draw_str(ax+88,body_y+13,"FAT volume",TEXT);
        }else if VIEW==VIEW_DISK{
            graphics::draw_str(ax+76,body_y+13,">",DIM);
            graphics::draw_str(ax+88,body_y+13,"Partitions",TEXT);
        }else if VIEW==VIEW_PROPS{
            graphics::draw_str(ax+76,body_y+13,">",DIM);
            graphics::draw_str(ax+88,body_y+13,"Properties",TEXT);
        }else{
            graphics::draw_str(ax+76,body_y+13,">",DIM);
            graphics::draw_str(ax+88,body_y+13,"Computer",TEXT);
        }
    }

    let sx=wx+ww.saturating_sub(search_w+8);
    graphics::fill_rect(sx,body_y+5,search_w,24,WHITE);
    graphics::border_rect(sx,body_y+5,search_w,24,BORDER);
    graphics::draw_str(sx+8,body_y+13,"Search",DIM);
    graphics::draw_str(sx+search_w-18,body_y+13,"?",DIM);
    let _=wy;
}
fn draw_sidebar(wx:usize,y:usize,w:usize,h:usize){
    graphics::fill_rect(wx+3,y,w,h,GLASS);
    graphics::border_rect(wx+3,y,w,h,BORDER);

    graphics::draw_str(wx+14,y+14,"QUICK ACCESS",DIM);
    icon::blit(icon::IconId::Folder,wx+10,y+28,false);
    graphics::draw_str(wx+40,y+36,"Desktop",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+48,false);
    graphics::draw_str(wx+40,y+56,"Documents",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+68,false);
    graphics::draw_str(wx+40,y+76,"Downloads",TEXT);

    graphics::draw_str(wx+14,y+106,"THIS PC",DIM);
    icon::blit(icon::IconId::MyComputer,wx+10,y+121,false);
    graphics::draw_str(wx+40,y+129,"Computer",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+141,false);
    graphics::draw_str(wx+40,y+149,"Local Disk (C:)",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+161,false);
    graphics::draw_str(wx+40,y+169,"AetherFS (A:)",TEXT);

    graphics::draw_str(wx+14,y+200,"SYSTEM",DIM);
    icon::blit(icon::IconId::MyComputer,wx+10,y+215,false);
    graphics::draw_str(wx+40,y+223,"Network",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+235,false);
    graphics::draw_str(wx+40,y+243,"Devices",TEXT);
    let _=h;
}
fn draw_drive(wx:usize,y:usize,name:&str,sub:&str,sel:bool){
    if sel{graphics::fill_rect(wx,y,252,72,SELECT);}
    icon::blit(icon::IconId::MyComputer,wx+12,y+10,false);
    graphics::draw_str(wx+54,y+12,name,TEXT);
    graphics::draw_str(wx+54,y+29,sub,DIM);
    graphics::fill_rect(wx+54,y+49,178,8,TOOL2);
    graphics::fill_rect(wx+54,y+49,96,8,BLUE);
    graphics::border_rect(wx+54,y+49,178,8,BORDER);
}
fn draw_computer(wx:usize,y:usize,ww:usize,h:usize){
    graphics::draw_str(wx,y+8,"This PC",TEXT);
    graphics::draw_str(wx,y+31,"Physical drives",DIM);

    // Render every physical disk discovered by the block layer. Do not collapse
    // multiple disks into a single "C:" card.
    let mut disks=[false;8];
    let mut dn=0usize;
    let pn=part::count();
    let mut i=0usize;
    while i<pn{
        if let Some(p)=part::get(i){
            let d=p.disk as usize;
            if d<8&&!disks[d]{disks[d]=true;dn+=1;}
        }
        i+=1;
    }

    let mut d=0usize;
    let mut row=0usize;
    while d<8{
        if disks[d]{
            let cy=y+45+row*82;
            graphics::fill_rect(wx,cy,ww,74,WHITE);
            graphics::border_rect(wx,cy,ww,74,BORDER);
            icon::blit(icon::IconId::MyComputer,wx+10,cy+11,false);

            if d==0{
                graphics::draw_str(wx+52,cy+12,"Disk 0",TEXT);
            }else if d==1{
                graphics::draw_str(wx+52,cy+12,"Disk 1",TEXT);
            }else{
                graphics::draw_str(wx+52,cy+12,"Disk",TEXT);
            }

            let mut parts_on_disk=0usize;
            let mut total_mb=0u32;
            let mut j=0usize;
            while j<pn{
                if let Some(p)=part::get(j){
                    if p.disk as usize==d{
                        parts_on_disk+=1;
                        total_mb=total_mb.saturating_add(p.sectors/2048);
                    }
                }
                j+=1;
            }
            graphics::draw_str(wx+52,cy+30,"Partitions:",DIM);
            draw_num(wx+112,cy+30,parts_on_disk as u32);
            graphics::draw_str(wx+145,cy+30,"•",DIM);
            draw_num(wx+157,cy+30,total_mb);
            graphics::draw_str(wx+198,cy+30,"MB",DIM);
            graphics::fill_rect(wx+52,cy+50,ww.saturating_sub(68),8,TOOL2);
            graphics::border_rect(wx+52,cy+50,ww.saturating_sub(68),8,BORDER);
            row+=1;
        }
        d+=1;
    }

    let ay=y+45+row*82;
    graphics::fill_rect(wx,ay,ww,74,WHITE);
    graphics::border_rect(wx,ay,ww,74,BORDER);
    icon::blit(icon::IconId::MyComputer,wx+10,ay+11,false);
    graphics::draw_str(wx+52,ay+12,"AetherFS (A:)",TEXT);
    graphics::draw_str(wx+52,ay+30,"RAM filesystem • Read/Write",DIM);
    graphics::fill_rect(wx+52,ay+50,ww.saturating_sub(68),8,TOOL2);
    graphics::draw_str(wx,y+h.saturating_sub(18),"Double-click a physical disk to inspect its partitions.",DIM);
}
fn draw_disk(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::draw_str(wx+8,y+8,"Physical disk partitions",TEXT);
    let n=part::count();
    if n==0{
        graphics::draw_str(wx+10,y+34,"No partitions detected.",RED);
        return;
    }
    graphics::fill_rect(wx,y+26,ww,24,TOOL2);
    graphics::draw_str(wx+8,y+34,"Disk",TEXT);
    graphics::draw_str(wx+58,y+34,"Partition",TEXT);
    graphics::draw_str(wx+140,y+34,"Filesystem",TEXT);
    graphics::draw_str(wx+280,y+34,"Size MB",TEXT);
    unsafe{
        let mut row=0usize;
        let mut i=0usize;
        while i<n&&row<18{
            if let Some(p)=part::get(i){
                let ry=y+52+row*24;
                if SEL==row as i32{graphics::fill_rect(wx,ry,ww,24,SELECT);}
                icon::blit(icon::IconId::MyComputer,wx+4,ry-1,false);
                draw_num(wx+28,ry+7,p.disk as u32);
                draw_num(wx+92,ry+7,p.index as u32);
                graphics::draw_str(wx+140,ry+7,part::type_name(p.ptype),DIM);
                draw_num(wx+280,ry+7,p.sectors/2048);
                row+=1;
            }
            i+=1;
        }
        graphics::draw_str(wx+8,y+h.saturating_sub(20),"Double-click a partition to open its read-only filesystem.",DIM);
    }
}
fn draw_fat(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::fill_rect(wx,y,ww,28,TOOL2);
    graphics::draw_str(wx+10,y+9,"Name",TEXT);
    graphics::draw_str(wx+250,y+9,"Type",TEXT);
    graphics::draw_str(wx+350,y+9,"Size",TEXT);
    let n=fs_fat::entry_count();
    if n==0{graphics::draw_str(wx+16,y+52,"This volume is empty or unreadable.",DIM);return;}
    let mut i=0usize;
    while i<n&&i<32{
        if let Some(e)=fs_fat::entry(i){
            let ry=y+ROW_TOP as usize+i*ROW_H as usize;
            unsafe{if SEL==i as i32{graphics::fill_rect(wx,ry,ww,24,SELECT);}}
            if e.is_dir{icon::blit(icon::IconId::Folder,wx+7,ry-2,false);}else{icon::blit(icon::IconId::File,wx+7,ry-2,false);}
            let mut k=0;while k<e.name_len&&k<27{graphics::draw_char(wx+42+k*8,ry+8,e.name[k],TEXT);k+=1;}
            graphics::draw_str(wx+250,ry+8,if e.is_dir{"Folder"}else{"File"},DIM);
            if !e.is_dir{draw_num(wx+350,ry+8,e.size);}
        }
        i+=1;
    }
}
fn draw_ntfs(wx:usize,y:usize,ww:usize,h:usize){
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::fill_rect(wx,y,ww,28,TOOL2);
    graphics::draw_str(wx+10,y+9,"Name",TEXT);
    graphics::draw_str(wx+250,y+9,"Type",TEXT);
    graphics::draw_str(wx+350,y+9,"Size",TEXT);
    let n=fs_ntfs::entry_count();
    if n==0{
        graphics::draw_str(wx+16,y+52,"This folder is empty or unavailable.",DIM);
        graphics::draw_str(wx+16,y+70,"NTFS is currently mounted read-only.",DIM);
        return;
    }
    let mut i=0usize;
    while i<n&&i<32{
        if let Some(e)=fs_ntfs::entry(i){
            let ry=y+ROW_TOP as usize+i*ROW_H as usize;
            unsafe{if SEL==i as i32{graphics::fill_rect(wx,ry,ww,24,SELECT);}if HOVER==i as i32{graphics::border_rect(wx,ry,ww,24,BLUE);}}
            if e.is_dir{icon::blit(icon::IconId::Folder,wx+7,ry-2,false);}else{icon::blit(icon::IconId::File,wx+7,ry-2,false);}
            let col=unsafe{if SEL==i as i32{BLUE}else{TEXT}};
            let mut k=0;while k<e.name_len&&k<27{graphics::draw_char(wx+42+k*8,ry+8,e.name[k],col);k+=1;}
            graphics::draw_str(wx+250,ry+8,if e.is_dir{"Folder"}else{"File"},DIM);
            if !e.is_dir{draw_num(wx+350,ry+8,if e.size>0xFFFF_FFFF{0xFFFF_FFFF}else{e.size as u32});}
        }
        i+=1;
    }
}
fn draw_list(wx:usize,y:usize,ww:usize,h:usize){
    if !fs::is_mounted(){graphics::draw_str(wx+10,y+10,"AetherFS is not mounted.",RED);return;}
    let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
    let n=unsafe{fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a)};
    graphics::fill_rect(wx,y,ww,h,WHITE);
    graphics::fill_rect(wx,y,ww,28,TOOL2);
    graphics::draw_str(wx+10,y+9,"Name",TEXT);
    graphics::draw_str(wx+220,y+9,"Type",TEXT);
    graphics::draw_str(wx+310,y+9,"Size",TEXT);
    if n==0{
        graphics::draw_str(wx+16,y+52,"This folder is empty.",DIM);
        graphics::draw_str(wx+16,y+70,"Create a folder or file from the context menu.",DIM);
        return;
    }
    unsafe{
        let mut i=0;
        while i<n{
            let ry=y+ROW_TOP as usize+i*ROW_H as usize;
            if SEL==i as i32{graphics::fill_rect(wx,ry,ww,24,SELECT);}
            if HOVER==i as i32{graphics::border_rect(wx,ry,ww,24,BLUE);}
            if a[i].is_dir{icon::blit(icon::IconId::Folder,wx+7,ry-2,false);}else{icon::blit(icon::IconId::File,wx+7,ry-2,false);}
            let col=if SEL==i as i32{BLUE}else{TEXT};
            let mut k=0;while k<a[i].name_len&&k<22{graphics::draw_char(wx+42+k*8,ry+8,a[i].name[k],col);k+=1;}
            graphics::draw_str(wx+220,ry+8,if a[i].is_dir{"Folder"}else{"File"},DIM);
            if !a[i].is_dir{draw_num(wx+310,ry+8,a[i].size);}
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

// Recreated Explorer shell: keep the visual layer self-contained and native to Aether.
fn draw_context(){
    unsafe{
        let x=CTX_X as usize;
        let y=CTX_Y as usize;
        let w=224usize;
        let h=224usize;
        graphics::fill_rect(x+3,y+3,w,h,0x00202020);
        graphics::fill_rect(x,y,w,h,WHITE);
        graphics::border_rect(x,y,w,h,BORDER);
        graphics::fill_rect(x+1,y+1,3,h-2,BLUE);
        graphics::draw_str(x+16,y+13,"Open",TEXT);
        graphics::draw_str(x+16,y+35,"Open with",TEXT);
        graphics::fill_rect(x+10,y+53,w-20,1,BORDER);
        graphics::draw_str(x+16,y+67,"Copy",TEXT);
        graphics::draw_str(x+16,y+89,"Cut",DIM);
        graphics::draw_str(x+16,y+111,"Rename",DIM);
        graphics::draw_str(x+16,y+133,"Delete",DIM);
        graphics::fill_rect(x+10,y+151,w-20,1,BORDER);
        graphics::draw_str(x+16,y+165,"Properties",TEXT);
        graphics::draw_str(x+16,y+187,"Refresh",TEXT);
        graphics::draw_str(x+16,y+209,"New folder",DIM);
    }
}
fn draw_confirm(wx:usize,wy:usize,ww:usize,wh:usize){
    let x=wx+ww/2-120;let y=wy+wh/2-45;
    graphics::fill_rect(x,y,240,90,WHITE);
    graphics::border_rect(x,y,240,90,BORDER);
    graphics::draw_str(x+18,y+18,"Delete selected item?",TEXT);
    btn(x+24,y+52,70,"Delete",true);
    btn(x+112,y+52,70,"Cancel",false);
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
                    7=>{if VIEW==VIEW_NTFS{if fs_ntfs::list_directory(NTFS_CWD_REF){trace_ntfs(b"DIR");}}else{let _=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut [fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16]);}SEL=-1;status(b"Refreshed");},
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
                    if my>=list_top+ROW_TOP{
                        let row=hit_row(list_top,my);
                        if row>=0&&row<n as i32{SEL=row;}
                    }
                }else if VIEW==VIEW_ROOT{
                    let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
                    let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);
                    if my>=list_top+ROW_TOP{
                        let row=hit_row(list_top,my);
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
            if VIEW==VIEW_COMPUTER{
                // Physical disk cards: clicking any disk opens the partition view.
                let pn=part::count();
                let mut disks=[false;8];
                let mut d=0usize;
                while d<8{
                    let mut j=0usize;
                    while j<pn{
                        if let Some(p)=part::get(j){if p.disk as usize==d{disks[d]=true;break;}}
                        j+=1;
                    }
                    d+=1;
                }
                let mut card=0usize;
                let mut dd=0usize;
                while dd<8{
                    if disks[dd]{
                        let top=list_top+45+(card*82) as i32;
                        if my>=top&&my<top+74{
                            // Select the first partition belonging to this physical disk.
                            let mut j=0usize;
                            while j<pn{
                                if let Some(p)=part::get(j){
                                    if p.disk as usize==dd{
                                        SEL=j as i32;
                                        VIEW=VIEW_DISK;
                                        status(b"Physical disk opened");
                                        return true;
                                    }
                                }
                                j+=1;
                            }
                        }
                        card+=1;
                    }
                    dd+=1;
                }
                // AetherFS card.
                let atop=list_top+45+(card*82) as i32;
                if my>=atop&&my<atop+74{
                    push_hist();
                    set_root();
                    status(b"AetherFS (A:) opened");
                    return true;
                }
            }
            else if VIEW==VIEW_DISK{
                let n=part::count();
                if my<list_top+52{return false;}
                let row=((my-list_top-52)/24)as i32;
                if row>=0&&row<n as i32{
                    SEL=row;
                    if let Some(p)=part::get(row as usize){
                        trace_partition_click(mx,my,row,p.index as i32);
                        log(b"partition open requested");
                        if p.ptype==0x07{
                            if fs_ntfs::mount_partition(row as usize){
                                VIEW=VIEW_NTFS;SEL=-1;NTFS_CWD_REF=5;NTFS_DEPTH=0;
                                trace_ntfs(b"OPEN");
                            }else{status(b"NTFS mount failed - read-only");}
                        }else if p.ptype==0x0B||p.ptype==0x0C||p.ptype==0x06||p.ptype==0x0E||p.ptype==0x04{
                            if fs_fat::mount_partition(row as usize){
                                VIEW=VIEW_FAT;SEL=-1;
                                status(b"FAT root opened read-only");
                            }else{status(b"FAT mount failed - read-only");}
                        }else{
                            status(b"Linux/ext4 filesystem not implemented");
                        }
                    }
                    return true;
                }
                return false;
            }
            else if VIEW==VIEW_NTFS{
                let n=fs_ntfs::entry_count();
                if my<list_top+28{return false;}
                let row=hit_row(list_top,my);
                if row>=0&&row<n as i32{
                    SEL=row;
                    if let Some(e)=fs_ntfs::entry(row as usize){
                        if e.is_dir{
                            if fs_ntfs::list_directory(e.mft_ref){
                                if NTFS_DEPTH<8{
                                    NTFS_PARENT[NTFS_DEPTH]=NTFS_CWD_REF;
                                    NTFS_DEPTH+=1;
                                }
                                NTFS_CWD_REF=e.mft_ref;
                                SEL=-1;
                                trace_ntfs(b"DIR");
                            }else{
                                status(b"NTFS folder read failed");
                            }
                        }else{
                            status(b"NTFS file preview not implemented");
                        }
                    }
                    return true;
                }
                return false;
            }
            else if VIEW==VIEW_ROOT{
                let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
                let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);
                if my<list_top+24{return false;}
                let row=hit_row(list_top,my);
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
            // Sidebar hit boxes match the labels/icons drawn above.
            if my>=list_top+132&&my<list_top+156{
                if fs_ntfs::mount_first(){
                    trace_ntfs(b"SIDEBAR");
                    VIEW=VIEW_NTFS;
                    SEL=-1;
                    NTFS_CWD_REF=5;
                    NTFS_DEPTH=0;
                    status(b"Local Disk (C:) opened");
                }else{
                    status(b"Local Disk (C:) mount failed");
                }
                return true;
            }
            if my>=list_top+156&&my<list_top+182{
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
                    trace_ntfs(b"DIR");
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
