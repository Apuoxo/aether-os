#![allow(dead_code)]
#![allow(static_mut_refs)]

use core::arch::asm;

pub const ALARM_W: i32 = 460;
pub const ALARM_H: i32 = 330;
pub const MAX_ALARMS: usize = 8;
const LABEL_LEN: usize = 16;
const NO_RING: usize = usize::MAX;
const CATCHUP_MIN: u64 = 5;
const SNOOZE_MIN: u64 = 5;
const AUTO_STOP_MS: u64 = 300_000;

pub const KEY_UP: u8 = 0x81;
pub const KEY_DOWN: u8 = 0x82;

#[inline] fn gfx_rect(x: usize,y: usize,w: usize,h: usize,c:u32){crate::graphics::fill_rect(x,y,w,h,c);}
#[inline] fn gfx_char(x: usize,y: usize,ch:u8,c:u32){crate::graphics::draw_char(x,y,ch,c);}
fn outb(port:u16,v:u8){unsafe{asm!("out dx, al",in("dx")port,in("al")v,options(nomem,nostack,preserves_flags));}}
fn inb(port:u16)->u8{let v:u8;unsafe{asm!("in al, dx",in("dx")port,out("al")v,options(nomem,nostack,preserves_flags));}v}

fn irq_save()->bool{let flags:u64;unsafe{asm!("pushfq","pop {}",out(reg)flags);asm!("cli",options(nomem,nostack));}flags&0x200!=0}
fn irq_restore(was_on:bool){if was_on{unsafe{asm!("sti",options(nomem,nostack));}}}

fn pcspk_on(freq:u32){if freq==0{return;}let div=1_193_180u32/freq;if div==0{return;}outb(0x43,0xB6);outb(0x42,(div&0xFF)as u8);outb(0x42,(div>>8)as u8);let v=inb(0x61);if v&3!=3{outb(0x61,v|3);}}
fn pcspk_off(){let v=inb(0x61);outb(0x61,v&0xFC);}
fn snd_start(freq:u32){pcspk_on(freq);}
fn snd_stop(){pcspk_off();}

const CMOS_NMI_BIT:u8=0;
#[derive(Clone,Copy,PartialEq,Eq)]
pub struct DateTime{pub year:u16,pub month:u8,pub day:u8,pub hour:u8,pub min:u8,pub sec:u8}
const DT_ZERO:DateTime=DateTime{year:2026,month:1,day:1,hour:0,min:0,sec:0};
fn cmos_read(reg:u8)->u8{outb(0x70,CMOS_NMI_BIT|reg);inb(0x71)}
fn rtc_wait_ready(){let mut spin=0;while cmos_read(0x0A)&0x80!=0&&spin<50_000{spin+=1;}}
fn rtc_raw()->[u8;6]{[cmos_read(0),cmos_read(2),cmos_read(4),cmos_read(7),cmos_read(8),cmos_read(9)]}
fn days_in_month(y:u16,m:u8)->u8{match m{2=>if y%4==0&&(y%100!=0||y%400==0){29}else{28},4|6|9|11=>30,_=>31}}
fn rtc_read()->Option<DateTime>{
 let was=irq_save();let mut a=[0u8;6];let mut ok=false;let mut tries=0;
 while tries<5{rtc_wait_ready();let first=rtc_raw();rtc_wait_ready();let second=rtc_raw();if first==second{a=second;ok=true;break;}tries+=1;}
 let sb=cmos_read(0x0B);irq_restore(was);if !ok{return None;}
 let bin=sb&4!=0;let h24=sb&2!=0;let pm=a[2]&0x80!=0;
 let conv=|v:u8|->u8{if bin{v}else{(v&0x0F)+(v>>4)*10}};
 let mut hour=conv(a[2]&0x7F);if !h24{hour%=12;if pm{hour+=12;}}
 let dt=DateTime{year:2000+conv(a[5])as u16,month:conv(a[4]),day:conv(a[3]),hour,min:conv(a[1]),sec:conv(a[0])};
 if dt.month<1||dt.month>12||dt.day<1||dt.day>days_in_month(dt.year,dt.month)||dt.hour>23||dt.min>59||dt.sec>59||dt.year<2020||dt.year>2099{return None;}
 Some(dt)
}
fn days_from_civil(y:i64,m:i64,d:i64)->i64{let y=if m<=2{y-1}else{y};let era=y/400;let yoe=y-era*400;let mp=(m+9)%12;let doy=(153*mp+2)/5+d-1;let doe=yoe*365+yoe/4-yoe/100+doy;era*146_097+doe-719_468}
fn weekday(y:u16,m:u8,d:u8)->u8{((days_from_civil(y as i64,m as i64,d as i64)+3).rem_euclid(7))as u8}
fn abs_minutes(n:&DateTime)->u64{let days=days_from_civil(n.year as i64,n.month as i64,n.day as i64);if days<0{0}else{days as u64*1440+n.hour as u64*60+n.min as u64}}
#[derive(Clone,Copy)] pub struct Alarm{pub used:bool,pub enabled:bool,pub hour:u8,pub min:u8,pub days:u8,pub label:[u8;LABEL_LEN],pub label_len:usize,pub last_fired:u64,pub snooze_due:u64}
const EMPTY:Alarm=Alarm{used:false,enabled:false,hour:7,min:0,days:0,label:[0;LABEL_LEN],label_len:0,last_fired:0,snooze_due:0};
static mut ALARMS:[Alarm;MAX_ALARMS]=[EMPTY;MAX_ALARMS];
static mut SEL:usize=0;static mut NOW:DateTime=DT_ZERO;static mut NOW_ABS:u64=0;static mut RTC_OK:bool=false;static mut POLLED:bool=false;static mut LAST_POLL_MS:u64=0;static mut RING:usize=NO_RING;static mut RING_START_MS:u64=0;static mut RING_START_ABS:u64=0;static mut BEEPING:bool=false;static mut BLINK:bool=false;static mut LABEL_EDIT:bool=false;static mut HOVER:u8=0;static mut OX:i32=0;static mut OY:i32=0;
fn set_label(a:&mut Alarm,s:&[u8]){let mut i=0;while i<s.len()&&i<LABEL_LEN{a.label[i]=s[i];i+=1;}a.label_len=i;}
fn sel_ok()->bool{unsafe{SEL<MAX_ALARMS&&ALARMS[SEL].used}}
fn prev_minute()->u64{unsafe{if NOW_ABS>0{NOW_ABS-1}else{0}}}
fn touch(){unsafe{ALARMS[SEL].snooze_due=0;ALARMS[SEL].last_fired=prev_minute();}}

const FACE:u32=0x00F0F0F0;const C_TEXT:u32=0;const C_DIM:u32=0x00808080;const C_BORDER:u32=0x00ABADB3;const C_ERR:u32=0x00A01010;const SEG_ON:u32=0x001F3D63;const SEG_OFF:u32=0x00C5D6EC;const SEG_OFF_SM:u32=0x00EEF2F8;
const B_HUP:u8=1;const B_HDN:u8=2;const B_MUP:u8=3;const B_MDN:u8=4;const B_NEW:u8=12;const B_DEL:u8=13;const B_ON:u8=14;const B_SNOOZE:u8=15;const B_DISMISS:u8=16;const B_LABEL:u8=17;
fn btn_rect(id:u8)->(i32,i32,i32,i32){match id{B_HUP=>(286,136,44,16),B_HDN=>(286,185,44,16),B_MUP=>(342,136,44,16),B_MDN=>(342,185,44,16),5..=11=>(286+(id as i32-5)*23,222,20,20),B_NEW=>(274,292,84,26),B_DEL=>(364,292,84,26),B_ON=>(396,154,50,20),B_SNOOZE=>(92,208,140,28),B_DISMISS=>(240,208,140,28),B_LABEL=>(330,246,112,20),_=>(0,0,0,0)}}
fn hit_button(lx:i32,ly:i32)->u8{let ringing=unsafe{RING!=NO_RING};let mut id=1;while id<=B_LABEL{let rb=id==B_SNOOZE||id==B_DISMISS;if rb==ringing{let(x,y,w,h)=btn_rect(id);if lx>=x&&lx<x+w&&ly>=y&&ly<y+h{return id;}}id+=1;}0}
fn r(x:i32,y:i32,w:i32,h:i32,c:u32){if w<=0||h<=0{return;}let(ox,oy)=unsafe{(OX,OY)};gfx_rect((ox+x)as usize,(oy+y)as usize,w as usize,h as usize,c);}
fn t(x:i32,y:i32,s:&[u8],c:u32){let(ox,oy)=unsafe{(OX,OY)};let mut i=0;while i<s.len(){gfx_char((ox+x+i as i32*8)as usize,(oy+y)as usize,s[i],c);i+=1;}}
fn lerp(a:u32,b:u32,i:i32,n:i32)->u32{if n<=1{return a;}let mut o=0;let mut sh=16;loop{let ca=((a>>sh)&255)as i32;let cb=((b>>sh)&255)as i32;o|=((ca+(cb-ca)*i/(n-1))as u32)<<sh;if sh==0{break;}sh-=8;}o}
fn vgrad(x:i32,y:i32,w:i32,h:i32,top:u32,bot:u32){let mut i=0;while i<h{r(x,y+i,w,1,lerp(top,bot,i,h));i+=1;}}
fn rframe(x:i32,y:i32,w:i32,h:i32,c:u32){r(x+1,y,w-2,1,c);r(x+1,y+h-1,w-2,1,c);r(x,y+1,1,h-2,c);r(x+w-1,y+1,1,h-2,c);}
fn arrow(cx:i32,cy:i32,up:bool,c:u32){let mut i=0;while i<4{let w=if up{1+i*2}else{7-i*2};r(cx-w/2,cy+i,w,1,c);i+=1;}}
fn button_face(id:u8,state:u8){let(x,y,w,h)=btn_rect(id);if state==2{r(x,y,w,h,0x00F4F4F4);rframe(x,y,w,h,0x00ADB2B5);return;}let hot=unsafe{HOVER}==id;let(a0,a1,b0,b1,bd)=if state==1{(0x00E5F4FC,0x00C4E5F6,0x0098D1EF,0x0068B3DB,0x002C628B)}else if hot{(0x00EAF6FD,0x00D9F0FC,0x00BEE6FD,0x00A7D9F5,0x003C7FB1)}else{(0x00F2F2F2,0x00EBEBEB,0x00DDDDDD,0x00CFCFCF,0x00707070)};let half=h/2;vgrad(x+1,y+1,w-2,half-1,a0,a1);vgrad(x+1,y+half,w-2,h-half-1,b0,b1);rframe(x,y,w,h,bd);if state==0{rframe(x+1,y+1,w-2,h-2,0x00FBFBFB);}}
fn btn_text(id:u8,s:&[u8],c:u32){let(x,y,w,h)=btn_rect(id);t(x+(w-s.len()as i32*8)/2,y+(h-10)/2,s,c);}
const MARK:[(i32,i32);8]=[(2,6),(3,7),(4,8),(5,7),(6,6),(7,5),(8,4),(9,3)];
fn checkbox(x:i32,y:i32,on:bool){r(x,y,13,13,0x008E8F8F);vgrad(x+1,y+1,11,11,0x00FFFFFF,0x00E6E7EE);if on{let mut i=0;while i<MARK.len(){r(x+MARK[i].0,y+MARK[i].1,2,2,0x001E395B);i+=1;}}}
const SEG:[u8;10]=[63,6,91,79,102,109,125,7,127,111];
fn seg_digit(x:i32,y:i32,w:i32,tk:i32,half:i32,d:u8,on:u32,off:u32){let h=3*tk+2*half;let m=SEG[(d%10)as usize];let ss=[(x+tk,y,w-2*tk,tk),(x+w-tk,y+tk,tk,half),(x+w-tk,y+2*tk+half,tk,half),(x+tk,y+h-tk,w-2*tk,tk),(x,y+2*tk+half,tk,half),(x,y+tk,tk,half),(x+tk,y+tk+half,w-2*tk,tk)];let mut i=0;while i<7{let c=if m&(1u8<<i)!=0{on}else{off};r(ss[i].0,ss[i].1,ss[i].2,ss[i].3,c);i+=1;}}
fn days_str(m:u8)->&'static[u8]{match m{0=>b"Once",127=>b"Daily",31=>b"Mon-Fri",96=>b"Sat,Sun",_=>b"Custom"}}
fn draw_header(){vgrad(0,0,ALARM_W,96,0x00E9F1FB,0x00BCD2EE);r(0,96,ALARM_W,1,0x0099B4D1);unsafe{if !RTC_OK{t(24,32,b"RTC time is invalid or unreadable",C_ERR);t(24,50,b"Alarms are paused. Check date/time.",C_ERR);return;}}let n=unsafe{NOW};let xs=[24,62,118,156,212,250];let ds=[n.hour/10,n.hour%10,n.min/10,n.min%10,n.sec/10,n.sec%10];let mut i=0;while i<6{seg_digit(xs[i],14,30,6,18,ds[i],SEG_ON,SEG_OFF);i+=1;}r(102,28,6,6,SEG_ON);r(102,48,6,6,SEG_ON);r(196,28,6,6,SEG_ON);r(196,48,6,6,SEG_ON);let wd=weekday(n.year,n.month,n.day)as usize;let names=b"MonTueWedThuFriSatSun";let mut b=[b' ';14];b[0]=names[wd*3];b[1]=names[wd*3+1];b[2]=names[wd*3+2];b[4]=b'0'+n.day/10%10;b[5]=b'0'+n.day%10;b[6]=b'.';b[7]=b'0'+n.month/10%10;b[8]=b'0'+n.month%10;b[9]=b'.';b[10]=b'0'+((n.year/1000)%10)as u8;b[11]=b'0'+((n.year/100)%10)as u8;b[12]=b'0'+((n.year/10)%10)as u8;b[13]=b'0'+(n.year%10)as u8;t(300,30,&b,0x001E395B);}
fn draw_list(){r(12,104,250,180,C_BORDER);r(13,105,248,178,0x00FFFFFF);let mut any=false;let mut i=0;while i<MAX_ALARMS{let a=unsafe{ALARMS[i]};if a.used{any=true;let y=106+i as i32*22;if unsafe{SEL}==i{vgrad(14,y,246,22,0x00DAECFC,0x00C1DBFC);rframe(14,y,246,22,0x007DA2CE);}checkbox(22,y+4,a.enabled);let col=if a.enabled{C_TEXT}else{C_DIM};let tt=[b'0'+a.hour/10,b'0'+a.hour%10,b':',b'0'+a.min/10,b'0'+a.min%10];t(44,y+6,&tt,col);let n=if a.label_len>12{12}else{a.label_len};t(92,y+6,&a.label[..n],col);t(200,y+6,days_str(a.days),C_DIM);}i+=1;}if !any{t(40,180,b"No alarms. Click New.",C_DIM);}}
fn spinner(x:i32,v:u8,up:u8,dn:u8){button_face(up,0);let(bx,by,bw,_)=btn_rect(up);arrow(bx+bw/2,by+6,true,C_TEXT);button_face(dn,0);let(bx2,by2,bw2,_)=btn_rect(dn);arrow(bx2+bw2/2,by2+6,false,C_TEXT);r(x,152,44,33,C_BORDER);r(x+1,153,42,31,0x00FFFFFF);seg_digit(x+6,156,14,3,8,v/10,SEG_ON,SEG_OFF_SM);seg_digit(x+24,156,14,3,8,v%10,SEG_ON,SEG_OFF_SM);}
fn draw_edit(){rframe(274,104,174,182,0x00D5DFE5);r(282,99,84,11,FACE);t(286,100,b"Edit alarm",C_TEXT);unsafe{if sel_ok(){let a=ALARMS[SEL];t(286,122,b"Time",C_TEXT);spinner(286,a.hour,B_HUP,B_HDN);r(336,162,3,3,SEG_ON);r(336,173,3,3,SEG_ON);spinner(342,a.min,B_MUP,B_MDN);checkbox(398,158,a.enabled);t(416,159,b"On",C_TEXT);t(286,208,b"Repeat:",C_TEXT);let mut d=0;while d<7{let id=5+d as u8;let on=a.days&(1u8<<d)!=0;button_face(id,if on{1}else{0});let(x,y,_,_)=btn_rect(id);t(x+6,y+5,&b"MTWTFSS"[d..d+1],C_TEXT);d+=1;}t(286,251,b"Label",C_TEXT);let(lx,ly,lw,lh)=btn_rect(B_LABEL);let edit=LABEL_EDIT;r(lx,ly,lw,lh,if edit{0x003399FF}else{C_BORDER});r(lx+1,ly+1,lw-2,lh-2,0x00FFFFFF);let n=if a.label_len>12{12}else{a.label_len};t(lx+4,ly+5,&a.label[..n],C_TEXT);if edit&&!BLINK{t(lx+4+n as i32*8,ly+5,b"_",C_TEXT);}}else{t(292,180,b"Select an alarm",C_DIM);}}button_face(B_NEW,0);btn_text(B_NEW,b"New",C_TEXT);let ok=sel_ok();button_face(B_DEL,if ok{0}else{2});btn_text(B_DEL,b"Delete",if ok{C_TEXT}else{C_DIM});}
fn draw_ring(){let(i,blink)=unsafe{(RING,BLINK)};if i>=MAX_ALARMS{return;}let a=unsafe{ALARMS[i]};r(73,83,320,170,0x00888888);r(70,80,320,170,0x00465B7A);r(71,81,318,168,0x00FFFFFF);if blink{vgrad(71,81,318,24,0x00FFE2A8,0x00F5B94E)}else{vgrad(71,81,318,24,0x00DCE9F8,0x00A9C6E8)}t(80,87,b"Alarm",C_TEXT);let x0=173;let y0=122;seg_digit(x0,y0,20,4,12,a.hour/10,SEG_ON,SEG_OFF_SM);seg_digit(x0+26,y0,20,4,12,a.hour%10,SEG_ON,SEG_OFF_SM);r(x0+59,y0+10,4,4,SEG_ON);r(x0+59,y0+22,4,4,SEG_ON);seg_digit(x0+68,y0,20,4,12,a.min/10,SEG_ON,SEG_OFF_SM);seg_digit(x0+94,y0,20,4,12,a.min%10,SEG_ON,SEG_OFF_SM);let n=a.label_len;t(70+(320-n as i32*8)/2,172,&a.label[..n],C_TEXT);r(71,196,318,53,FACE);r(71,196,318,1,0x00DFDFDF);button_face(B_SNOOZE,0);btn_text(B_SNOOZE,b"Snooze 5 min",C_TEXT);button_face(B_DISMISS,0);btn_text(B_DISMISS,b"Dismiss",C_TEXT);}
fn add_alarm()->bool{unsafe{let mut i=0;while i<MAX_ALARMS{if !ALARMS[i].used{let mut a=EMPTY;a.used=true;a.enabled=true;a.hour=(NOW.hour+1)%24;a.min=0;a.last_fired=prev_minute();set_label(&mut a,b"Alarm");ALARMS[i]=a;SEL=i;return true;}i+=1;}}false}
fn delete_sel(){unsafe{if !sel_ok()||RING!=NO_RING{return;}let mut i=SEL;while i+1<MAX_ALARMS{ALARMS[i]=ALARMS[i+1];i+=1;}ALARMS[MAX_ALARMS-1]=EMPTY;LABEL_EDIT=false;if !ALARMS[SEL].used&&SEL>0{SEL-=1;}}}
fn toggle_enabled(){unsafe{if sel_ok(){ALARMS[SEL].enabled=!ALARMS[SEL].enabled;touch();}}}
fn shift_hour(d:i32){unsafe{if sel_ok(){ALARMS[SEL].hour=((ALARMS[SEL].hour as i32+d+24)%24)as u8;touch();}}}
fn shift_min(d:i32){unsafe{if sel_ok(){ALARMS[SEL].min=((ALARMS[SEL].min as i32+d+60)%60)as u8;touch();}}}
fn toggle_day(d:usize){unsafe{if sel_ok()&&d<7{ALARMS[SEL].days^=1u8<<d;touch();}}}
fn select_step(d:i32){unsafe{let mut i=SEL as i32;let mut n=0;while n<MAX_ALARMS as i32{i+=d;if i<0{i=MAX_ALARMS as i32-1;}if i>=MAX_ALARMS as i32{i=0;}if ALARMS[i as usize].used{SEL=i as usize;return;}n+=1;}}}
fn stop_ring(){unsafe{if BEEPING{snd_stop();BEEPING=false;}RING=NO_RING;}}
fn snooze_ring(){unsafe{if RING<MAX_ALARMS&&ALARMS[RING].used{ALARMS[RING].snooze_due=NOW_ABS+SNOOZE_MIN;}}stop_ring();}
fn rebase_alarms(){unsafe{let p=prev_minute();let mut i=0;while i<MAX_ALARMS{if ALARMS[i].used{ALARMS[i].last_fired=p;ALARMS[i].snooze_due=0;}i+=1;}}}
fn check_alarms(){unsafe{let now=NOW_ABS;let tod=NOW.hour as u64*60+NOW.min as u64;let day_start=now-tod;let wd_today=weekday(NOW.year,NOW.month,NOW.day);let mut i=0;while i<MAX_ALARMS{if ALARMS[i].used{if ALARMS[i].snooze_due!=0&&now>=ALARMS[i].snooze_due{let late=now-ALARMS[i].snooze_due;ALARMS[i].snooze_due=0;if late<=CATCHUP_MIN{RING=i;return;}}if ALARMS[i].enabled{let sched_tod=ALARMS[i].hour as u64*60+ALARMS[i].min as u64;let mut back=0;while back<2{let sched=day_start-back*1440+sched_tod;let wd=(wd_today+7-back as u8)%7;let day_ok=ALARMS[i].days==0||(ALARMS[i].days&(1u8<<wd))!=0;if sched<=now&&now-sched<=CATCHUP_MIN&&sched>ALARMS[i].last_fired&&day_ok{ALARMS[i].last_fired=sched;if ALARMS[i].days==0{ALARMS[i].enabled=false;}RING=i;return;}back+=1;}}}i+=1;}}}
fn poll_rtc(ms:u64)->bool{let mut redraw=false;unsafe{LAST_POLL_MS=ms;POLLED=true;match rtc_read(){Some(n)=>{if !RTC_OK{redraw=true;}NOW=n;NOW_ABS=abs_minutes(&n);if !RTC_OK{RTC_OK=true;rebase_alarms();}if RING==NO_RING{check_alarms();if RING!=NO_RING{RING_START_MS=ms;RING_START_ABS=NOW_ABS;BLINK=false;redraw=true;}}}None=>{if RTC_OK{redraw=true;}RTC_OK=false;}}}redraw}
pub fn alarm_init(){unsafe{let mut a=EMPTY;a.used=true;a.enabled=false;a.hour=7;a.min=0;a.days=31;set_label(&mut a,b"Wake up");ALARMS[0]=a;SEL=0;RTC_OK=false;POLLED=false;RING=NO_RING;BEEPING=false;LABEL_EDIT=false;}poll_rtc(0);}
pub fn alarm_ringing()->bool{unsafe{RING!=NO_RING}}
pub fn alarm_tick(ms:u64)->bool{let mut redraw=false;unsafe{if poll_rtc(ms){redraw=true;}if RING!=NO_RING{if NOW_ABS>=RING_START_ABS+5{stop_ring();return true;}let sec=NOW.sec as u64;let on=sec%2==0;if on!=BEEPING{if on{snd_start(880)}else{snd_stop()}BEEPING=on;}let bl=sec%2==1;if bl!=BLINK{BLINK=bl;}}}redraw}
pub fn alarm_draw(ox:i32,oy:i32){unsafe{OX=ox;OY=oy;}r(0,0,ALARM_W,ALARM_H,FACE);draw_header();draw_list();draw_edit();t(12,292,b"Space:on/off N:new Del:delete",C_DIM);t(12,306,b"H/M:+- +/-:5min 1-7:days E:label",C_DIM);if alarm_ringing(){draw_ring();}}
pub fn alarm_mouse_move(lx:i32,ly:i32)->bool{let id=hit_button(lx,ly);unsafe{if id!=HOVER{HOVER=id;return true;}}false}
pub fn alarm_click(lx:i32,ly:i32)->bool{unsafe{let id=hit_button(lx,ly);if RING!=NO_RING{if id==B_SNOOZE{snooze_ring();return true;}if id==B_DISMISS{stop_ring();return true;}return false;}if id!=B_LABEL{LABEL_EDIT=false;}match id{B_HUP=>shift_hour(1),B_HDN=>shift_hour(-1),B_MUP=>shift_min(1),B_MDN=>shift_min(-1),5..=11=>toggle_day((id-5)as usize),B_NEW=>{add_alarm();},B_DEL=>delete_sel(),B_ON=>toggle_enabled(),B_LABEL=>{if sel_ok(){LABEL_EDIT=true;}},_=>{if lx>=14&&lx<260&&ly>=106&&ly<106+(MAX_ALARMS as i32)*22{let row=((ly-106)/22)as usize;if row<MAX_ALARMS&&ALARMS[row].used{SEL=row;if lx>=22&&lx<37{toggle_enabled();}}}else{return false;}}}true}}
pub fn alarm_key(k:u8)->bool{unsafe{if RING!=NO_RING{if k==b' '||k==13{stop_ring();return true;}if k==b's'||k==b'S'{snooze_ring();return true;}return false;}if LABEL_EDIT{if !sel_ok(){LABEL_EDIT=false;return true;}if k==13||k==10||k==27{LABEL_EDIT=false;}else if k==8||k==127{if ALARMS[SEL].label_len>0{ALARMS[SEL].label_len-=1;}}else if k>=32&&k<127&&ALARMS[SEL].label_len<LABEL_LEN{let n=ALARMS[SEL].label_len;ALARMS[SEL].label[n]=k;ALARMS[SEL].label_len=n+1;}return true;}match k{KEY_UP=>select_step(-1),KEY_DOWN=>select_step(1),b' '=>toggle_enabled(),b'n'|b'N'=>{add_alarm();},8|127=>delete_sel(),b'h'=>shift_hour(1),b'H'=>shift_hour(-1),b'm'=>shift_min(1),b'M'=>shift_min(-1),b'+'=>shift_min(5),b'-'=>shift_min(-5),b'1'..=b'7'=>toggle_day((k-b'1')as usize),b'e'|b'E'=>{if sel_ok(){LABEL_EDIT=true;}},_=>return false}true}}
