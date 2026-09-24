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

const W7_BAR_T:u32=0x00FAFBFD; const W7_BAR_B:u32=0x00E5EAF3;
const W7_HDR_T:u32=0x00FFFFFF; const W7_HDR_B:u32=0x00F1F3F7;
const W7_SEP:u32=0x00D5D5D5;
const W7_NAV:u32=0x001E395B;

fn lerp(a:u32,b:u32,t:usize,n:usize)->u32{
    let mut o=0u32;let mut s=0;
    while s<=16{
        let ca=((a>>s)&0xFF) as i32;let cb=((b>>s)&0xFF) as i32;
        o|=((ca+(cb-ca)*(t as i32)/(n as i32)) as u32)<<s;s+=8;
    }
    o
}
fn vgrad(x:usize,y:usize,w:usize,h:usize,top:u32,bot:u32){
    let mut i=0;while i<h{graphics::fill_rect(x,y+i,w,1,lerp(top,bot,i,h.max(1)));i+=1;}
}
fn sel_row(x:usize,y:usize,w:usize,h:usize,hot:bool){
    let (t,b,br)=if hot{(0x00FAFBFD,0x00EBF3FD,0x00B8D6FB)}else{(0x00F5FAFF,0x00DCEBFC,0x007DA2CE)};
    vgrad(x+1,y+1,w-2,h-2,t,b);
    graphics::border_rect(x,y,w,h,br);
    graphics::fill_rect(x,y,1,1,WHITE);graphics::fill_rect(x+w-1,y,1,1,WHITE);
    graphics::fill_rect(x,y+h-1,1,1,WHITE);graphics::fill_rect(x+w-1,y+h-1,1,1,WHITE);
}

pub const VIEW_COMPUTER:u8=0;
pub const VIEW_ROOT:u8=1;
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
static mut NTFS_VIEW_IDX:[u8;256]=[0;256];
static mut NTFS_VIEW_COUNT:usize=0;
static mut NTFS_SCROLL:usize=0;
static mut NTFS_VIEW_DIRTY:bool=true;
static mut NTFS_VISIBLE_ROWS:usize=1;
static mut PREVIEW:[u8;512]=[0;512];
static mut PREVIEW_LEN:usize=0;
static mut STATUS:[u8;64]=[0;64];
static mut STATUS_LEN:usize=0;

#[derive(Clone,Copy)]
struct Props{
    kind:u8, // 1=file, 2=folder
    name:[u8;48],
    name_len:usize,
    size:u64,
    alloc:u64,
    created:u64,
    modified:u64,
    accessed:u64,
    attrs:u32,
    files:u32,
    dirs:u32,
    mft:u32,
}
static mut PROPS_OPEN:bool=false;
static mut PROPS:Props=Props{
    kind:0,name:[0;48],name_len:0,size:0,alloc:0,
    created:0,modified:0,accessed:0,attrs:0,files:0,dirs:0,mft:0
};

struct Buf{b:[u8;72],n:usize}
impl Buf{
    fn new()->Self{Buf{b:[0;72],n:0}}
    fn s(&mut self,t:&str){for c in t.bytes(){if self.n<72{self.b[self.n]=c;self.n+=1;}}}
    fn u(&mut self,mut v:u64){let mut d=[0u8;20];let mut c=0;if v==0{d[0]=b'0';c=1;}while v>0{d[c]=(v%10) as u8+b'0';v/=10;c+=1;}while c>0{c-=1;if self.n<72{self.b[self.n]=d[c];self.n+=1;}}}
    fn size(&mut self,v:u64){
        let (dv,u)=if v>=1<<30{(1u64<<30,"GB")}else if v>=1<<20{(1u64<<20,"MB")}else if v>=1<<10{(1u64<<10,"KB")}else{(1,"B")};
        self.u(v/dv);if dv>1{self.s(".");self.u((v%dv)*10/dv);}self.s(" ");self.s(u);
    }
    fn as_str(&self)->&str{unsafe{core::str::from_utf8_unchecked(&self.b[..self.n])}}
}
fn prow(x:usize,y:&mut usize,label:&str,v:&Buf){
    graphics::draw_str(x,*y,label,DIM);graphics::draw_str(x+110,*y,v.as_str(),TEXT);*y+=18;
}
fn prop_name(){
    unsafe{
        let mut i=0;
        while i<48{PROPS.name[i]=0;i+=1;}
    }
}
fn fill_props_selected(){
    unsafe{
        prop_name();
        PROPS.kind=0;PROPS.name_len=0;PROPS.size=0;PROPS.alloc=0;
        PROPS.created=0;PROPS.modified=0;PROPS.accessed=0;PROPS.attrs=0;
        PROPS.files=0;PROPS.dirs=0;PROPS.mft=0;
        if VIEW==VIEW_NTFS{
            if NTFS_VIEW_DIRTY{rebuild_ntfs_view();}
            if SEL<0{return;}
            let idx=match ntfs_row_index(NTFS_SCROLL+SEL as usize){Some(v)=>v,None=>return};
            if let Some(e)=fs_ntfs::entry(idx){
                PROPS.kind=if e.is_dir{2}else{1};
                PROPS.name_len=e.name_len.min(48);
                let mut i=0;while i<PROPS.name_len{PROPS.name[i]=e.name[i];i+=1;}
                PROPS.size=e.size;PROPS.alloc=e.size;PROPS.modified=e.mtime;PROPS.mft=e.mft_ref;
                PROPS_OPEN=true;
                status(b"Properties opened");
            }
        }else if VIEW==VIEW_ROOT{
            if SEL<0{return;}
            let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
            let n=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a);
            let s=SEL as usize;
            if s>=n{return;}
            PROPS.kind=if a[s].is_dir{2}else{1};
            PROPS.name_len=a[s].name_len.min(48);
            let mut i=0;while i<PROPS.name_len{PROPS.name[i]=a[s].name[i];i+=1;}
            PROPS.size=a[s].size as u64;PROPS.alloc=PROPS.size;
            PROPS_OPEN=true;
            status(b"Properties opened");
        }
    }
}
fn draw_props_dialog(wx:usize,wy:usize,ww:usize,wh:usize){
    unsafe{
        if !PROPS_OPEN{return;}
        let x=wx+ww/2-160;let y=wy+wh/2-130;
        graphics::fill_rect(x+4,y+4,320,260,0x00202020);
        graphics::fill_rect(x,y,320,260,WHITE);
        graphics::border_rect(x,y,320,260,BORDER);
        vgrad(x+1,y+1,318,30,W7_BAR_T,W7_BAR_B);
        graphics::draw_str(x+12,y+10,"Properties",TEXT);
        btn(x+272,y+4,36,"Close",false);
        if PROPS.kind==2{icon::blit(icon::IconId::Folder,x+16,y+48,false);}else{icon::blit(icon::IconId::File,x+16,y+48,false);}
        let mut k=0;while k<PROPS.name_len&&k<32{graphics::draw_char(x+54+k*8,y+55,PROPS.name[k],TEXT);k+=1;}
        graphics::fill_rect(x+14,y+82,292,1,W7_SEP);
        let mut yy=y+94;
        let mut b=Buf::new();
        b.s(if PROPS.kind==2{"Folder"}else{"File"});prow(x+14,&mut yy,"Type:",&b);
        b=Buf::new();b.size(PROPS.size);prow(x+14,&mut yy,"Size:",&b);
        b=Buf::new();b.size(PROPS.alloc);prow(x+14,&mut yy,"Size on disk:",&b);
        if PROPS.kind==2{
            b=Buf::new();b.u(PROPS.files as u64);prow(x+14,&mut yy,"Files:",&b);
            b=Buf::new();b.u(PROPS.dirs as u64);prow(x+14,&mut yy,"Folders:",&b);
        }else if PROPS.mft!=0{
            b=Buf::new();b.u(PROPS.mft as u64);prow(x+14,&mut yy,"MFT:",&b);
        }
        if PROPS.modified!=0{
            graphics::draw_str(x+14,yy,"Modified:",DIM);draw_mtime(x+124,yy,PROPS.modified);yy+=18;
        }else{
            graphics::draw_str(x+14,yy,"Dates:",DIM);graphics::draw_str(x+124,yy,"Not available yet",TEXT);yy+=18;
        }
        graphics::draw_str(x+14,yy,"Attributes:",DIM);
        graphics::draw_str(x+124,yy,if PROPS.attrs==0{"None reported"}else{"Present"},TEXT);
    }
}


fn log(s:&[u8]){serial::write_str("[EXPLORER] ");if let Ok(v)=core::str::from_utf8(s){serial::write_str(v);}serial::write_str("\n");}
fn status(s:&[u8]){log(s);unsafe{let mut i=0;while i<64{STATUS[i]=0;i+=1;}i=0;while i<s.len()&&i<63{STATUS[i]=s[i];i+=1;}STATUS_LEN=i;}}
fn put_str(b:&mut [u8;64],p:&mut usize,s:&[u8]){let mut i=0;while i<s.len()&&*p<63{b[*p]=s[i];*p+=1;i+=1;}}
fn put_num(b:&mut [u8;64],p:&mut usize,mut n:usize){let mut d=[0u8;10];let mut c=0;if n==0{d[0]=b'0';c=1;}while n>0&&c<10{d[c]=(n%10)as u8+b'0';n/=10;c+=1;}while c>0&&*p<63{c-=1;b[*p]=d[c];*p+=1;}}
fn trace_ntfs(tag:&[u8]){let n=fs_ntfs::entry_count();let mut b=[0u8;64];let mut p=0usize;put_str(&mut b,&mut p,tag);put_str(&mut b,&mut p,b" n=");put_num(&mut b,&mut p,n);let mut i=0usize;while i<3&&i<n{if let Some(e)=fs_ntfs::entry(i){put_str(&mut b,&mut p,b" ");put_num(&mut b,&mut p,i);put_str(&mut b,&mut p,b"=");let mut k=0;while k<e.name_len&&k<12&&p<62{put_str(&mut b,&mut p,&e.name[k..k+1]);k+=1;}}i+=1;}status(&b[..p]);}
fn trace_partition_click(mx:i32,my:i32,row:i32,part_no:i32){let mut b=[0u8;64];let mut p=0usize;put_str(&mut b,&mut p,b"PCLICK mx=");put_num(&mut b,&mut p,mx.max(0) as usize);put_str(&mut b,&mut p,b" my=");put_num(&mut b,&mut p,my.max(0) as usize);put_str(&mut b,&mut p,b" row=");put_num(&mut b,&mut p,row.max(0) as usize);put_str(&mut b,&mut p,b" part=");put_num(&mut b,&mut p,part_no.max(0) as usize);status(&b[..p]);}
const ROW_H:i32=32;
const ROW_TOP:i32=30;
fn hit_row(list_top:i32,my:i32)->i32{if my<list_top+ROW_TOP{-1}else{(my-list_top-ROW_TOP)/ROW_H}}
fn ntfs_visible(e:&fs_ntfs::NtfsEntry)->bool{if e.name_len==0{return false;}if e.name[0]==b'.'||e.name[0]==b'$'{return false;}true}
fn ntfs_cmp(a:usize,b:usize)->core::cmp::Ordering{let ea=match fs_ntfs::entry(a){Some(v)=>v,None=>return core::cmp::Ordering::Equal};let eb=match fs_ntfs::entry(b){Some(v)=>v,None=>return core::cmp::Ordering::Equal};if ea.is_dir!=eb.is_dir{return if ea.is_dir{core::cmp::Ordering::Less}else{core::cmp::Ordering::Greater};}let mut i=0usize;while i<ea.name_len&&i<eb.name_len{let mut ca=ea.name[i];let mut cb=eb.name[i];if ca>=b'a'&&ca<=b'z'{ca-=32;}if cb>=b'a'&&cb<=b'z'{cb-=32;}if ca<cb{return core::cmp::Ordering::Less;}if ca>cb{return core::cmp::Ordering::Greater;}i+=1;}ea.name_len.cmp(&eb.name_len)}
fn rebuild_ntfs_view(){unsafe{let n=fs_ntfs::entry_count();NTFS_VIEW_COUNT=0;let mut i=0usize;while i<n&&NTFS_VIEW_COUNT<256{if let Some(e)=fs_ntfs::entry(i){if ntfs_visible(&e){NTFS_VIEW_IDX[NTFS_VIEW_COUNT]=i as u8;NTFS_VIEW_COUNT+=1;}}i+=1;}let mut i=1usize;while i<NTFS_VIEW_COUNT{let key=NTFS_VIEW_IDX[i];let mut j=i;while j>0{let prev=NTFS_VIEW_IDX[j-1]as usize;if ntfs_cmp(key as usize,prev)!=core::cmp::Ordering::Less{break;}NTFS_VIEW_IDX[j]=NTFS_VIEW_IDX[j-1];j-=1;}NTFS_VIEW_IDX[j]=key;i+=1;}NTFS_SCROLL=0;NTFS_VIEW_DIRTY=false;}}
fn ntfs_row_index(row:usize)->Option<usize>{unsafe{if NTFS_VIEW_DIRTY{rebuild_ntfs_view();}if row<NTFS_VIEW_COUNT{Some(NTFS_VIEW_IDX[row]as usize)}else{None}}}
fn row_h(_view:u8)->i32{32}
fn visible_rows(h:usize)->usize{h.saturating_sub(48)/row_h(VIEW_NTFS) as usize}
pub fn on_escape()->bool{unsafe{if PROPS_OPEN{PROPS_OPEN=false;return true;}if CTX{CTX=false;return true;}false}}
pub fn on_nav_key(sc:u8)->bool{unsafe{if VIEW!=VIEW_NTFS{return false;}if NTFS_VIEW_DIRTY{rebuild_ntfs_view();}let rows=unsafe{NTFS_VISIBLE_ROWS};if sc==0x48{if SEL>0{SEL-=1;}else if NTFS_SCROLL>0{NTFS_SCROLL-=1;}return true;}if sc==0x50{if SEL>=0&&(SEL as usize)+1<rows&&NTFS_SCROLL+(SEL as usize)+1<NTFS_VIEW_COUNT{SEL+=1;}else if NTFS_SCROLL+rows<NTFS_VIEW_COUNT{NTFS_SCROLL+=1;SEL=if rows>0{(rows-1)as i32}else{-1};}return true;}}false}
fn set_root(){unsafe{CWD[0]=b'/';CWD_LEN=1;VIEW=VIEW_ROOT;SEL=-1;}}
fn push_hist(){unsafe{if HIST_N<8{let mut i=0;while i<32{HIST[HIST_N][i]=CWD[i];i+=1;}HIST_LEN[HIST_N]=CWD_LEN;HIST_N+=1;HIST_I=HIST_N;}}}
fn draw_num(x:usize,y:usize,mut n:u32){if n==0{graphics::draw_char(x,y,b'0',TEXT);return;}let mut d=[0u8;10];let mut c=0;while n>0{d[c]=(n%10)as u8+b'0';n/=10;c+=1;}let start=c;while c>0{c-=1;graphics::draw_char(x+(start-1-c)*8,y,d[c],TEXT);}}
fn ft_ymdhm(ft:u64)->(u32,u32,u32,u32,u32){
    let secs=(ft/10_000_000).saturating_sub(11_644_473_600);
    let days=(secs/86400) as i64; let rem=(secs%86400) as u32;
    let z=days+719468; let era=z.div_euclid(146097); let doe=z-era*146097;
    let yoe=(doe-doe/1460+doe/36524-doe/146096)/365;
    let doy=doe-(365*yoe+yoe/4-yoe/100); let mp=(5*doy+2)/153;
    let d=(doy-(153*mp+2)/5+1) as u32; let m=(if mp<10{mp+3}else{mp-9}) as u32;
    let y=(yoe+era*400+if m<=2{1}else{0}) as u32;
    (y,m,d,rem/3600,(rem%3600)/60)
}
fn draw_2(x:usize,y:usize,v:u32){
    graphics::draw_char(x,y,b'0'+((v/10)%10) as u8,TEXT);
    graphics::draw_char(x+8,y,b'0'+(v%10) as u8,TEXT);
}
fn draw_mtime(x:usize,y:usize,ft:u64){
    let (yr,mo,d,hh,mi)=ft_ymdhm(ft);
    let mut q=yr; let mut digs=[0u8;4]; let mut i=4usize;
    while i>0{i-=1;digs[i]=b'0'+(q%10) as u8;q/=10;}
    graphics::draw_char(x,y,digs[2],TEXT); graphics::draw_char(x+8,y,digs[3],TEXT);
    graphics::draw_char(x+16,y,b'.',TEXT); draw_2(x+24,y,d); graphics::draw_char(x+40,y,b'.',TEXT); draw_2(x+48,y,mo);
    graphics::draw_char(x+64,y,b' ',TEXT); draw_2(x+72,y,hh); graphics::draw_char(x+88,y,b':',TEXT); draw_2(x+96,y,mi);
}
fn btn(x:usize,y:usize,w:usize,label:&str,hot:bool){
    if hot{vgrad(x,y,w,24,0x00EAF6FD,0x00A7D9F5);}else{vgrad(x,y,w,24,0x00F6F7F9,0x00E3E8EF);}
    graphics::border_rect(x,y,w,24,if hot{0x003C7FB1}else{0x00A0A8B4});
    graphics::draw_str(x+8,y+8,label,TEXT);
}

pub fn reset(){unsafe{VIEW=VIEW_COMPUTER;SEL=-1;HOVER=-1;FOCUS_ADDR=false;CTX=false;CONFIRM_DEL=false;HIST_N=0;HIST_I=0;CWD[0]=b'/';CWD_LEN=1;NTFS_CWD_REF=5;NTFS_DEPTH=0;NTFS_VIEW_COUNT=0;NTFS_SCROLL=0;NTFS_VISIBLE_ROWS=1;NTFS_VIEW_DIRTY=true;PREVIEW_LEN=0;PROPS_OPEN=false;}status(b"Ready");}

fn draw_header(wx:usize,wy:usize,ww:usize,body_y:usize){
    // Modern, compact Aether Explorer chrome. Designed for the real 800x600 target.
    vgrad(wx+3,body_y,ww-6,34,W7_BAR_T,W7_BAR_B);
    graphics::fill_rect(wx+3,body_y+33,ww-6,1,0x00C5CDD8);

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
    graphics::fill_rect(wx+3,y,w,h,WHITE);
    graphics::border_rect(wx+3,y,w,h,BORDER);

    graphics::draw_str(wx+14,y+14,"QUICK ACCESS",W7_NAV);
    icon::blit(icon::IconId::Folder,wx+10,y+28,false);
    graphics::draw_str(wx+40,y+36,"Desktop",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+60,false);
    graphics::draw_str(wx+40,y+68,"Documents",TEXT);
    icon::blit(icon::IconId::Folder,wx+10,y+92,false);
    graphics::draw_str(wx+40,y+100,"Downloads",TEXT);

    graphics::draw_str(wx+14,y+122,"THIS PC",W7_NAV);
    icon::blit(icon::IconId::MyComputer,wx+10,y+137,false);
    graphics::draw_str(wx+40,y+145,"Computer",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+169,false);
    graphics::draw_str(wx+40,y+177,"Local Disk (C:)",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+201,false);
    graphics::draw_str(wx+40,y+209,"AetherFS (A:)",TEXT);

    graphics::draw_str(wx+14,y+246,"SYSTEM",W7_NAV);
    icon::blit(icon::IconId::MyComputer,wx+10,y+261,false);
    graphics::draw_str(wx+40,y+269,"Network",TEXT);
    icon::blit(icon::IconId::MyComputer,wx+10,y+293,false);
    graphics::draw_str(wx+40,y+301,"Devices",TEXT);
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
            draw_num(wx+132,cy+30,parts_on_disk as u32);
            graphics::draw_str(wx+145,cy+30,"-",DIM);
            draw_num(wx+166,cy+30,total_mb);
            graphics::draw_str(wx+214,cy+30,"MB",DIM);
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
    vgrad(wx,y,ww,28,W7_HDR_T,W7_HDR_B);
    graphics::fill_rect(wx,y+27,ww,1,W7_SEP);
    graphics::fill_rect(wx+244,y+3,1,22,W7_SEP);
    graphics::fill_rect(wx+344,y+3,1,22,W7_SEP);
    graphics::draw_str(wx+8,y+9,"Disk",TEXT);
    graphics::draw_str(wx+76,y+9,"Partition",TEXT);
    graphics::draw_str(wx+140,y+9,"Filesystem",TEXT);
    graphics::draw_str(wx+280,y+9,"Size MB",TEXT);
    unsafe{
        let mut row=0usize;
        let mut i=0usize;
        while i<n&&row<18{
            if let Some(p)=part::get(i){
                let ry=y+52+row*32;
                unsafe{
                    if SEL==row as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,false);}
                    else if HOVER==row as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,true);}
                }
                icon::blit(icon::IconId::MyComputer,wx+4,ry,false);
                draw_num(wx+44,ry+7,p.disk as u32);
                draw_num(wx+108,ry+7,p.index as u32);
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
    vgrad(wx,y,ww,28,W7_HDR_T,W7_HDR_B);
    graphics::fill_rect(wx,y+27,ww,1,W7_SEP);
    graphics::fill_rect(wx+244,y+3,1,22,W7_SEP);
    graphics::fill_rect(wx+344,y+3,1,22,W7_SEP);
    graphics::draw_str(wx+10,y+9,"Name",TEXT);
    graphics::draw_str(wx+250,y+9,"Type",TEXT);
    graphics::draw_str(wx+350,y+9,"Size",TEXT);
    let n=fs_fat::entry_count();
    if n==0{graphics::draw_str(wx+16,y+52,"This volume is empty or unreadable.",DIM);return;}
    let mut i=0usize;
    while i<n&&i<32&&y+ROW_TOP as usize+i*ROW_H as usize+24<=y+h{
        if let Some(e)=fs_fat::entry(i){
            let ry=y+ROW_TOP as usize+i*ROW_H as usize;
            unsafe{
                if SEL==i as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,false);}
                else if HOVER==i as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,true);}
            }
            if e.is_dir{icon::blit(icon::IconId::Folder,wx+7,ry,false);}else{icon::blit(icon::IconId::File,wx+7,ry,false);}
            let col=TEXT;
            let mut k=0;while k<e.name_len&&k<27{graphics::draw_char(wx+42+k*8,ry+10,e.name[k],col);k+=1;}
            graphics::draw_str(wx+250,ry+8,if e.is_dir{"Folder"}else{"File"},DIM);
            if !e.is_dir{draw_num(wx+350,ry+8,e.size);}
        }
        i+=1;
    }
}
fn draw_ntfs(wx:usize,y:usize,ww:usize,h:usize){
    if unsafe{NTFS_VIEW_DIRTY}{rebuild_ntfs_view();}
    graphics::fill_rect(wx,y,ww,h,WHITE);
    vgrad(wx,y,ww,28,W7_HDR_T,W7_HDR_B);
    graphics::fill_rect(wx,y+27,ww,1,W7_SEP);
    graphics::fill_rect(wx+244,y+3,1,22,W7_SEP);
    graphics::fill_rect(wx+384,y+3,1,22,W7_SEP);
    graphics::fill_rect(wx+454,y+3,1,22,W7_SEP);
    graphics::draw_str(wx+10,y+9,"Name",TEXT);
    graphics::draw_str(wx+250,y+9,"Date modified",TEXT);
    graphics::draw_str(wx+390,y+9,"Type",TEXT);
    graphics::draw_str(wx+460,y+9,"Size",TEXT);
    if unsafe{NTFS_VIEW_COUNT==0}{graphics::draw_str(wx+16,y+52,"This folder is empty or unavailable.",DIM);graphics::draw_str(wx+16,y+70,"NTFS is currently mounted read-only.",DIM);return;}
    let rows=visible_rows(h);unsafe{NTFS_VISIBLE_ROWS=if rows==0{1}else{rows};if NTFS_SCROLL+NTFS_VISIBLE_ROWS>NTFS_VIEW_COUNT{NTFS_SCROLL=NTFS_VIEW_COUNT.saturating_sub(NTFS_VISIBLE_ROWS);}}
    let mut r=0usize;
    while r<rows{
        let vi=unsafe{NTFS_SCROLL+r};
        let idx=match ntfs_row_index(vi){Some(v)=>v,None=>break};
        if let Some(e)=fs_ntfs::entry(idx){
            let ry=y+ROW_TOP as usize+r*32;
            unsafe{
                if SEL==r as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,false);}
                else if HOVER==r as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,true);}
            }
            if e.is_dir{icon::blit(icon::IconId::Folder,wx+7,ry,false);}else{icon::blit(icon::IconId::File,wx+7,ry,false);}
            let col=TEXT;
            let mut k=0;while k<e.name_len&&k<27{graphics::draw_char(wx+42+k*8,ry+10,e.name[k],col);k+=1;}
            draw_mtime(wx+250,ry+10,e.mtime);
            graphics::draw_str(wx+390,ry+10,if e.is_dir{"Folder"}else{"File"},DIM);
            if !e.is_dir{draw_num(wx+460,ry+10,if e.size>0xFFFF_FFFF{0xFFFF_FFFF}else{e.size as u32});}
        }
        r+=1;
    }
}
fn draw_list(wx:usize,y:usize,ww:usize,h:usize){
    if !fs::is_mounted(){graphics::draw_str(wx+10,y+10,"AetherFS is not mounted.",RED);return;}
    let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
    let n=unsafe{fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut a)};
    graphics::fill_rect(wx,y,ww,h,WHITE);
    vgrad(wx,y,ww,28,W7_HDR_T,W7_HDR_B);
    graphics::fill_rect(wx,y+27,ww,1,W7_SEP);
    graphics::fill_rect(wx+214,y+3,1,22,W7_SEP);
    graphics::fill_rect(wx+304,y+3,1,22,W7_SEP);
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
            if SEL==i as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,false);}
            else if HOVER==i as i32{sel_row(wx+2,ry,ww-4,ROW_H as usize,true);}
            if a[i].is_dir{icon::blit(icon::IconId::Folder,wx+7,ry,false);}else{icon::blit(icon::IconId::File,wx+7,ry,false);}
            let col=TEXT;
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


pub fn draw(wx:usize,wy:usize,ww:usize,wh:usize,title_h:usize){
    let by=wy+title_h;
    graphics::fill_rect(wx+3,by,ww-6,wh.saturating_sub(title_h+3),WHITE);
    draw_header(wx,wy,ww,by);
    let sy=by+40;let sh=wh.saturating_sub(title_h+62);
    let side=150usize.min(ww/3);
    draw_sidebar(wx,sy,side,sh);
    let cx=wx+side+8;let cw=ww.saturating_sub(side+16);
    match unsafe{VIEW}{VIEW_COMPUTER=>draw_computer(cx,sy,cw,sh),VIEW_ROOT=>draw_list(cx,sy,cw,sh),VIEW_DISK=>draw_disk(cx,sy,cw,sh),VIEW_FAT=>draw_fat(cx,sy,cw,sh),VIEW_NTFS=>draw_ntfs(cx,sy,cw,sh),VIEW_TEXT=>draw_text(cx,sy,cw,sh),_=>{}}
    if unsafe{CTX}{draw_context();}
    if unsafe{CONFIRM_DEL}{draw_confirm(wx,wy,ww,wh);} if unsafe{PROPS_OPEN}{draw_props_dialog(wx,wy,ww,wh);}
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

pub fn on_click(wx:i32,wy:i32,ww:i32,wh:i32,title_h:i32,mx:i32,my:i32,right:bool,double_click:bool)->bool{
    let by=wy+title_h;let relx=mx-wx;let rely=my-by;
    unsafe{
        if PROPS_OPEN{
            let px=wx+ww/2-160;let py=wy+wh/2-130;
            if (mx>=px+272&&mx<px+308&&my>=py+4&&my<py+28) || (right && mx>=px&&mx<px+320&&my>=py&&my<py+260) {
                PROPS_OPEN=false;
            }
            return true;
        }
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
                    6=>{fill_props_selected();},
                    7=>{if VIEW==VIEW_NTFS{if fs_ntfs::list_directory(NTFS_CWD_REF){NTFS_VIEW_DIRTY=true;NTFS_SCROLL=0;trace_ntfs(b"DIR");}}else{let _=fs::list_ex_path(core::str::from_utf8_unchecked(&CWD[..CWD_LEN]),&mut [fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16]);}SEL=-1;status(b"Refreshed");},
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
            status(b"RCLICK");
            let side=150i32.min(ww/3);
            let list_top=by+40;
            if mx>=wx+side+8&&my>=list_top{
                if VIEW==VIEW_NTFS{
                    if unsafe{NTFS_VIEW_DIRTY}{rebuild_ntfs_view();}
                    let n=unsafe{NTFS_VIEW_COUNT};
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
                let row=((my-list_top-52)/ROW_H)as i32;
                if row>=0&&row<n as i32{
                    SEL=row;
                    if let Some(p)=part::get(row as usize){
                        trace_partition_click(mx,my,row,p.index as i32);
                        log(b"partition open requested");
                        if p.ptype==0x07{
                            if fs_ntfs::mount_partition(row as usize){
                                VIEW=VIEW_NTFS;SEL=-1;NTFS_CWD_REF=5;NTFS_DEPTH=0;NTFS_VIEW_DIRTY=true;NTFS_SCROLL=0;
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
                    }
                    return true;
                }
                return false;
            }
            else if VIEW==VIEW_NTFS{
                if unsafe{NTFS_VIEW_DIRTY}{rebuild_ntfs_view();}
                let n=unsafe{NTFS_VIEW_COUNT};
                if my<list_top+ROW_TOP{return false;}
                let row=hit_row(list_top,my);
                if row>=0&&row<n as i32{
                    let was_selected = SEL==row;
                    SEL=row;
                    let idx=match ntfs_row_index((unsafe{NTFS_SCROLL})+row as usize){Some(v)=>v,None=>return false};
                    if double_click && was_selected {
                    if let Some(e)=fs_ntfs::entry(idx){
                        if e.is_dir{
                            if fs_ntfs::list_directory(e.mft_ref){
                                NTFS_VIEW_DIRTY=true;NTFS_SCROLL=0;
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
                if my<list_top+ROW_TOP{return false;}
                let row=hit_row(list_top,my);
                if row>=0&&row<n as i32{
                    if SEL==row && double_click{
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
            if my>=list_top+160&&my<list_top+192{
                if fs_ntfs::mount_first(){
                    trace_ntfs(b"SIDEBAR");
                    VIEW=VIEW_NTFS;
                    SEL=-1;
                    NTFS_CWD_REF=5;
                    NTFS_DEPTH=0;
                    NTFS_VIEW_DIRTY=true;
                    NTFS_SCROLL=0;
                    status(b"Local Disk (C:) opened");
                }else{
                    status(b"Local Disk (C:) mount failed");
                }
                return true;
            }
            if my>=list_top+192&&my<list_top+224{
                push_hist();
                set_root();
                status(b"AetherFS (A:) opened");
                return true;
            }
            return false;
        }
        if VIEW==VIEW_TEXT{return true;}
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
                    NTFS_VIEW_DIRTY=true;NTFS_SCROLL=0;SEL=-1;
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
