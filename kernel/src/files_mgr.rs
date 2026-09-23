//! Aether Explorer — native Windows 7 inspired file manager.
//! This replaces the old XP-style file window.  It is an original Aether UI,
//! not a copy of Microsoft code/assets.

use crate::fs;
use crate::graphics;
use crate::gui::{font, icon, theme};

const WHITE:u32=0x00FFFFFF;
const TEXT:u32=0x001F1F1F;
const DIM:u32=0x005A5A5A;
const BORDER:u32=0x00B7B7B7;
const TOOL:u32=0x00F4F4F4;
const TOOL2:u32=0x00E7E7E7;
const BLUE:u32=0x003A7CC1;
const BLUE2:u32=0x00DCEEFF;
const SELECT:u32=0x00CDE7FA;
const SIDE:u32=0x00F7F7F7;
const GREEN:u32=0x003B8D3B;
const RED:u32=0x00B03030;

pub const VIEW_COMPUTER:u8=0;
pub const VIEW_ROOT:u8=1;
pub const VIEW_PROPS:u8=2;
pub const VIEW_TEXT:u8=3;

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
static mut PREVIEW:[u8;512]=[0;512];
static mut PREVIEW_LEN:usize=0;
static mut STATUS:[u8;64]=[0;64];
static mut STATUS_LEN:usize=0;

fn status(s:&[u8]){unsafe{let mut i=0;while i<64{STATUS[i]=0;i+=1;}i=0;while i<s.len()&&i<63{STATUS[i]=s[i];i+=1;}STATUS_LEN=i;}}
fn set_root(){unsafe{CWD[0]=b'/';CWD_LEN=1;VIEW=VIEW_ROOT;SEL=-1;}}
fn push_hist(){unsafe{if HIST_N<8{let mut i=0;while i<32{HIST[HIST_N][i]=CWD[i];i+=1;}HIST_LEN[HIST_N]=CWD_LEN;HIST_N+=1;HIST_I=HIST_N;}}}
fn draw_num(x:usize,y:usize,mut n:u32){if n==0{graphics::draw_char(x,y,b'0',TEXT);return;}let mut d=[0u8;10];let mut c=0;while n>0{d[c]=(n%10)as u8+b'0';n/=10;c+=1;}while c>0{c-=1;graphics::draw_char(x+(d.len()-1-c)*0,y,d[c],TEXT);}}
fn btn(x:usize,y:usize,w:usize,label:&str,hot:bool){graphics::fill_rect(x,y,w,24,if hot{BLUE2}else{TOOL});graphics::border_rect(x,y,w,24,BORDER);graphics::draw_str(x+8,y+8,label,if hot{BLUE}else{TEXT});}

pub fn reset(){unsafe{VIEW=VIEW_COMPUTER;SEL=-1;HOVER=-1;FOCUS_ADDR=false;CTX=false;CONFIRM_DEL=false;HIST_N=0;HIST_I=0;CWD[0]=b'/';CWD_LEN=1;PREVIEW_LEN=0;}status(b"Ready");}

fn draw_header(wx:usize,wy:usize,ww:usize,body_y:usize){
    // Windows 7-like command bar.
    graphics::fill_rect(wx+3,body_y,ww-6,34,TOOL);
    btn(wx+8,body_y+5,34,"<",false);
    btn(wx+44,body_y+5,34,">",false);
    btn(wx+80,body_y+5,34,"Up",false);
    btn(wx+118,body_y+5,62,"Organize",false);
    btn(wx+184,body_y+5,58,"Views",false);
    btn(wx+246,body_y+5,62,"Share",false);
    // address bar
    let ax=wx+314; let aw=ww.saturating_sub(414);
    graphics::fill_rect(ax,body_y+5,aw,24,WHITE);
    graphics::border_rect(ax,body_y+5,aw,24,BORDER);
    graphics::draw_str(ax+8,body_y+13,"Computer",DIM);
    if aw>90{graphics::draw_str(ax+76,body_y+13,">",DIM);unsafe{if VIEW==VIEW_ROOT{graphics::draw_str(ax+90,body_y+13,"AetherFS (A:)",TEXT);}}}
    let sx=wx+ww.saturating_sub(94);
    graphics::fill_rect(sx,body_y+5,82,24,WHITE);graphics::border_rect(sx,body_y+5,82,24,BORDER);
    graphics::draw_str(sx+8,body_y+13,"Search",DIM);
    let _=wy;
}

fn draw_sidebar(wx:usize,y:usize,w:usize,h:usize){
    graphics::fill_rect(wx+3,y,w,h,SIDE);
    graphics::border_rect(wx+3,y,w,h,BORDER);
    graphics::draw_str(wx+14,y+14,"Favorites",BLUE);
    graphics::draw_str(wx+24,y+34,"Desktop",TEXT);
    graphics::draw_str(wx+24,y+52,"Downloads",TEXT);
    graphics::draw_str(wx+24,y+70,"Documents",TEXT);
    graphics::draw_str(wx+14,y+100,"Libraries",BLUE);
    graphics::draw_str(wx+24,y+120,"Documents",TEXT);
    graphics::draw_str(wx+24,y+138,"Pictures",TEXT);
    graphics::draw_str(wx+24,y+156,"Music",TEXT);
    graphics::draw_str(wx+24,y+174,"Videos",TEXT);
    graphics::draw_str(wx+14,y+204,"Computer",BLUE);
    graphics::draw_str(wx+24,y+224,"Local Disk (C:)",TEXT);
    graphics::draw_str(wx+24,y+242,"AetherFS (A:)",TEXT);
    graphics::draw_str(wx+14,y+272,"Network",BLUE);
    graphics::draw_str(wx+24,y+292,"Network",TEXT);
    let _=h;
}

fn draw_drive(wx:usize,y:usize,name:&str,sub:&str,sel:bool){
    if sel{graphics::fill_rect(wx,y,250,52,SELECT);}
    icon::blit(icon::IconId::MyComputer,wx+8,y+8,false);
    graphics::draw_str(wx+50,y+12,name,TEXT);
    graphics::draw_str(wx+50,y+29,sub,DIM);
}

fn draw_computer(wx:usize,y:usize,ww:usize,h:usize){
    graphics::draw_str(wx,y+8,"Computer",TEXT);
    graphics::draw_str(wx,y+28,"Hard Disk Drives",DIM);
    draw_drive(wx,y+42,"Local Disk (C:)","Physical ATA device",false);
    draw_drive(wx,y+98,"AetherFS (A:)","Aether RAM volume",false);
    graphics::draw_str(wx,y+166,"Devices with Removable Storage",DIM);
    graphics::draw_str(wx+8,y+188,"No removable volumes mounted.",DIM);
    graphics::draw_str(wx,y+h.saturating_sub(20),"Double-click AetherFS (A:) to browse files.",DIM);
    let _=ww;
}

fn draw_list(wx:usize,y:usize,ww:usize,h:usize){
    if !fs::is_mounted(){graphics::draw_str(wx+10,y+10,"AetherFS is not mounted.",RED);return;}
    let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];
    let n=fs::list_ex(&mut a);
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
    unsafe{if SEL<0{graphics::draw_str(wx+14,y+40,"No item selected.",DIM);return;}let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];let n=fs::list_ex(&mut a);let s=SEL as usize;if s>=n{return;}graphics::draw_str(wx+14,y+46,"Name:",DIM);let mut k=0;while k<a[s].name_len{graphics::draw_char(wx+66+k*8,y+46,a[s].name[k],TEXT);k+=1;}graphics::draw_str(wx+14,y+66,if a[s].is_dir{"Type: Folder"}else{"Type: File"},TEXT);if !a[s].is_dir{graphics::draw_str(wx+14,y+86,"Size:",DIM);draw_num(wx+56,y+86,a[s].size);}graphics::draw_str(wx+14,y+106,"Location: AetherFS (A:)",DIM);}
}

fn draw_context(){
    unsafe{let x=CTX_X as usize;let y=CTX_Y as usize;graphics::fill_rect(x,y,150,112,WHITE);graphics::border_rect(x,y,150,112,BORDER);graphics::draw_str(x+10,y+10,"Open",TEXT);graphics::draw_str(x+10,y+30,"New folder",TEXT);graphics::draw_str(x+10,y+50,"New text file",TEXT);graphics::draw_str(x+10,y+70,"Rename",TEXT);graphics::draw_str(x+10,y+90,"Delete",TEXT);}
}
fn draw_confirm(wx:usize,wy:usize,ww:usize,wh:usize){
    let x=wx+ww/2-120;let y=wy+wh/2-45;graphics::fill_rect(x,y,240,90,WHITE);graphics::border_rect(x,y,240,90,BORDER);graphics::draw_str(x+18,y+18,"Delete selected item?",TEXT);btn(x+24,y+52,70,"Delete",true);btn(x+112,y+52,70,"Cancel",false);
}

pub fn draw(wx:usize,wy:usize,ww:usize,wh:usize,title_h:usize){
    let by=wy+title_h;
    graphics::fill_rect(wx+3,by,ww-6,wh.saturating_sub(title_h+3),WHITE);
    draw_header(wx,wy,ww,by);
    let sy=by+40;let sh=wh.saturating_sub(title_h+62);
    let side=150usize.min(ww/3);
    draw_sidebar(wx,sy,side,sh);
    let cx=wx+side+8;let cw=ww.saturating_sub(side+16);
    match unsafe{VIEW}{VIEW_COMPUTER=>draw_computer(cx,sy,cw,sh),VIEW_ROOT=>draw_list(cx,sy,cw,sh),VIEW_TEXT=>draw_text(cx,sy,cw,sh),VIEW_PROPS=>draw_props(cx,sy,cw,sh),_=>{}}
    if unsafe{CTX}{draw_context();}
    if unsafe{CONFIRM_DEL}{draw_confirm(wx,wy,ww,wh);}
    let status_y=wy+wh-18;graphics::fill_rect(wx+3,status_y,ww-6,16,TOOL);graphics::border_rect(wx+3,status_y,ww-6,16,BORDER);unsafe{let mut i=0;while i<STATUS_LEN{graphics::draw_char(wx+9+i*8,status_y+4,STATUS[i],DIM);i+=1;}}
}


fn open_selected(){
    unsafe{
        if SEL<0{return;}
        let mut a=[fs::ListItem{name:[0;24],name_len:0,size:0,is_dir:false};16];let n=fs::list_ex(&mut a);let s=SEL as usize;if s>=n{return;}
        if a[s].is_dir{status(b"Folder navigation is next VFS stage");return;}
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
        if CTX{if mx>=CTX_X&&mx<CTX_X+150&&my>=CTX_Y&&my<CTX_Y+112{let r=(my-CTX_Y)/20;match r{0=>open_selected(),1=>new_folder(),2=>new_file(),3=>rename_selected(),4=>{CONFIRM_DEL=true;},_=>{}}CTX=false;return true;}CTX=false;return true;}
        if right{CTX=true;CTX_X=mx;CTX_Y=my;return true;}
        if rely>=5&&rely<35{
            if relx<44{go_back();}else if relx<80{go_forward();}else if relx<116{go_up();}else if relx<182{status(b"Organize");}else if relx<244{status(b"Views: Details");}else if relx<310{status(b"Share");}
            return true;
        }
        let side=150i32.min(ww/3);let list_top=by+40;
        if mx>=wx+side+8&&my>=list_top{
            let cx=mx-(wx+side+8);let _=cx;
            if VIEW==VIEW_COMPUTER{if my>=list_top+42&&my<list_top+150{push_hist();set_root();status(b"AetherFS (A:) opened");}}
            else if VIEW==VIEW_ROOT{let row=((my-list_top-24)/20)as i32;if row>=0&&row<16{if SEL==row{open_selected();}else{SEL=row;status(b"Selected");}}}
            return true;
        }
        if my>=list_top&&my<list_top+330&&mx<wx+side{
            // sidebar entries
            if my>list_top+200{push_hist();set_root();status(b"AetherFS (A:)");}
            return true;
        }
        if VIEW==VIEW_TEXT||VIEW==VIEW_PROPS{VIEW=VIEW_ROOT;return true;}
    }
    true
}
fn go_back(){unsafe{if HIST_I>0{HIST_I-=1;let mut i=0;while i<32{CWD[i]=HIST[HIST_I][i];i+=1;}CWD_LEN=HIST_LEN[HIST_I];VIEW=VIEW_ROOT;SEL=-1;status(b"Back");}else{VIEW=VIEW_COMPUTER;SEL=-1;status(b"Computer");}}}
fn go_forward(){unsafe{if HIST_I+1<HIST_N{HIST_I+=1;VIEW=VIEW_ROOT;SEL=-1;status(b"Forward");}}}
fn go_up(){unsafe{VIEW=VIEW_COMPUTER;SEL=-1;CWD[0]=b'/';CWD_LEN=1;status(b"Computer");}}
