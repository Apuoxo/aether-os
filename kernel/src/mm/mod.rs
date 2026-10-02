//! Physical Memory Manager + paging re-export
use core::sync::atomic::{AtomicUsize, Ordering};
pub mod paging;
pub const PAGE_SIZE: usize = 4096;
const LOW_PHYS_LIMIT: usize = 1 << 30;
const FREE: u8 = 1;
const USED: u8 = 2;
static mut MAP: usize = 0;
static PAGES: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE_PAGES: AtomicUsize = AtomicUsize::new(0);
static HINT: AtomicUsize = AtomicUsize::new(0);
static MAX_PHYS: AtomicUsize = AtomicUsize::new(0);

#[inline] fn au(v: usize) -> usize { (v + PAGE_SIZE - 1) & !(PAGE_SIZE - 1) }
#[inline] fn ad(v: usize) -> usize { v & !(PAGE_SIZE - 1) }
unsafe fn get(p: usize) -> u8 {
    if p >= PAGES.load(Ordering::Relaxed) { return 0; }
    let b = *((MAP as *const u8).add(p >> 2));
    (b >> ((p & 3) * 2)) & 3
}
unsafe fn set(p: usize, s: u8) {
    if p >= PAGES.load(Ordering::Relaxed) { return; }
    let q = (MAP as *mut u8).add(p >> 2);
    let sh = (p & 3) * 2;
    *q = (*q & !(3 << sh)) | ((s & 3) << sh);
}
fn total(mbi: usize) -> usize {
    if mbi == 0 { 0 } else { unsafe { core::ptr::read_unaligned(mbi as *const u32) as usize } }
}
fn mmap<F: FnMut(u64,u64,u32)>(mbi: usize, mut f: F) {
    let t = total(mbi); if t < 16 || t > 0x100000 { return; }
    unsafe {
        let mut o=8;
        while o+8<=t {
            let ty=core::ptr::read_unaligned((mbi+o) as *const u32);
            let sz=core::ptr::read_unaligned((mbi+o+4) as *const u32) as usize;
            if sz<8 || o+sz>t { break; } if ty==0 { break; }
            if ty==6 && sz>=16 {
                let es=core::ptr::read_unaligned((mbi+o+8) as *const u32) as usize;
                if es<24 { break; }
                let mut p=o+16;
                while p+es<=o+sz {
                    let b=core::ptr::read_unaligned((mbi+p) as *const u64);
                    let l=core::ptr::read_unaligned((mbi+p+8) as *const u64);
                    let k=core::ptr::read_unaligned((mbi+p+16) as *const u32);
                    f(b,l,k); p+=es;
                }
                return;
            }
            o=(o+sz+7)&!7;
        }
    }
}
fn reserve_tags<F: FnMut(u64,u64)>(mbi: usize, mut f:F) {
    let t=total(mbi); if t<16 || t>0x100000{return;}
    unsafe {
        let mut o=8;
        while o+8<=t {
            let ty=core::ptr::read_unaligned((mbi+o) as *const u32);
            let sz=core::ptr::read_unaligned((mbi+o+4) as *const u32) as usize;
            if sz<8 || o+sz>t {break;} if ty==0 {break;}
            if ty==3 && sz>=16 {
                let b=core::ptr::read_unaligned((mbi+o+8) as *const u32) as u64;
                let e=core::ptr::read_unaligned((mbi+o+12) as *const u32) as u64;
                if e>b {f(b,e-b);}
            } else if ty==8 && sz>=32 {
                let b=core::ptr::read_unaligned((mbi+o+8) as *const u64);
                let pitch=core::ptr::read_unaligned((mbi+o+16) as *const u32) as u64;
                let h=core::ptr::read_unaligned((mbi+o+24) as *const u32) as u64;
                let l=pitch.saturating_mul(h); if l!=0 {f(b,l);}
            }
            o=(o+sz+7)&!7;
        }
    }
}
fn reserve(a:usize,l:usize) {
    let mut p=ad(a)/PAGE_SIZE; let e=au(a.saturating_add(l))/PAGE_SIZE;
    unsafe { while p<e {set(p,0);p+=1;} }
}
pub fn init_from_multiboot(mbi:usize,kernel_end:usize)->bool {
    let t=total(mbi); if t<16 || t>0x100000{return false;}
    let mut max=0u64;
    mmap(mbi,|b,l,_|{if let Some(e)=b.checked_add(l){if e>max{max=e;}}});
    if max==0{return false;}
    let pages64=(max+4095)/4096;
    let pages=match usize::try_from(pages64){Ok(v) if v>0=>v,_=>return false};
    let bytes=match pages.checked_add(3){Some(v)=>v/4,None=>return false};
    let map_bytes=au(bytes);
    let mut bitmap=0usize;
    mmap(mbi,|b,l,k|{
        if bitmap!=0 || k!=1{return;}
        let mut s=match usize::try_from(b){Ok(v)=>au(v),Err(_)=>return;};
        let e=match b.checked_add(l).and_then(|v|usize::try_from(v).ok()){Some(v)=>ad(v),None=>return;};
        if s<0x100000{s=0x100000;} if s<kernel_end{s=au(kernel_end);}
        if s<LOW_PHYS_LIMIT && map_bytes<=e.saturating_sub(s){bitmap=s;}
    });
    if bitmap==0{return false;}
    unsafe {
        MAP=bitmap; PAGES.store(pages,Ordering::SeqCst);
        TOTAL.store(0,Ordering::SeqCst); FREE_PAGES.store(0,Ordering::SeqCst);
        HINT.store(0,Ordering::SeqCst); MAX_PHYS.store(usize::try_from(max).unwrap_or(usize::MAX),Ordering::SeqCst);
        core::ptr::write_bytes(bitmap as *mut u8,0,map_bytes);
    }
    mmap(mbi,|b,l,k|{
        if k!=1{return;}
        let s=match usize::try_from(b).ok().map(au){Some(v)=>v,None=>return};
        let e=match b.checked_add(l).and_then(|v|usize::try_from(v).ok()).map(ad){Some(v)=>v,None=>return};
        let mut p=s/PAGE_SIZE; let last=e/PAGE_SIZE;
        unsafe{while p<last{set(p,FREE);p+=1;}}
    });
    reserve(0,0x100000);
    reserve(0x100000,kernel_end.saturating_sub(0x100000));
    reserve(mbi,t);
    reserve_tags(mbi,|b,l|{if let (Ok(a),Ok(z))=(usize::try_from(b),usize::try_from(l)){reserve(a,z);}});
    reserve(bitmap,map_bytes);
    let mut managed=0; let mut free=0; let mut p=0;
    unsafe{while p<pages{if get(p)==FREE{managed+=1;free+=1;}p+=1;}}
    TOTAL.store(managed,Ordering::SeqCst); FREE_PAGES.store(free,Ordering::SeqCst);
    crate::serial::write_str("[PMM] Multiboot2 map initialized; dynamic physical page state\n");
    crate::serial::write_str("[PMM] bitmap="); crate::serial::write_hex(bitmap);
    crate::serial::write_str(" bytes="); crate::serial::write_usize(map_bytes);
    crate::serial::write_str(" managed_mib="); crate::serial::write_usize(managed/256);
    crate::serial::write_str("\n");
    true
}
pub fn alloc_page()->Option<usize>{alloc_page_below(usize::MAX)}
pub fn alloc_page_below(limit:usize)->Option<usize>{
    let n=PAGES.load(Ordering::Acquire); if FREE_PAGES.load(Ordering::Acquire)==0{return None;}
    let max=core::cmp::min(n,limit/PAGE_SIZE); let start=core::cmp::min(HINT.load(Ordering::Relaxed),max);
    unsafe{
        let mut pass=0; while pass<2 {
            let mut p=if pass==0{start}else{0}; let end=if pass==0{max}else{start};
            while p<end {if get(p)==FREE{set(p,USED);FREE_PAGES.fetch_sub(1,Ordering::SeqCst);HINT.store(if p+1<n{p+1}else{0},Ordering::Relaxed);return Some(p*PAGE_SIZE);}p+=1;}
            pass+=1;
        }
    } None
}
pub fn free_page(a:usize){if a&(PAGE_SIZE-1)!=0{return;}let p=a/PAGE_SIZE;unsafe{if get(p)==USED{set(p,FREE);FREE_PAGES.fetch_add(1,Ordering::SeqCst);}}}
pub fn free_count()->usize{FREE_PAGES.load(Ordering::SeqCst)}
pub fn total_count()->usize{TOTAL.load(Ordering::SeqCst)}
pub fn max_physical_address()->usize{MAX_PHYS.load(Ordering::Acquire)}
pub fn alloc_pages(count:usize)->Option<usize>{
    if count==0||FREE_PAGES.load(Ordering::Acquire)<count{return None;}let n=PAGES.load(Ordering::Acquire);
    unsafe{let mut s=0;while s+count<=n{let mut ok=true;let mut i=0;while i<count{if get(s+i)!=FREE{ok=false;break;}i+=1;}if ok{let mut j=0;while j<count{set(s+j,USED);j+=1;}FREE_PAGES.fetch_sub(count,Ordering::SeqCst);return Some(s*PAGE_SIZE);}s+=1;}}None
}
pub fn zero_pages(phys:usize,count:usize){
    let mut p=phys;let mut left=count;
    while left!=0{
        if p<LOW_PHYS_LIMIT && p+PAGE_SIZE<=LOW_PHYS_LIMIT{unsafe{core::ptr::write_bytes(p as *mut u8,0,PAGE_SIZE);}}
        else{let c=paging::kernel_cr3();if c!=0 && unsafe{paging::map_page(c,paging::TEMP_PHYS_MAP,p,paging::PAGE_PRESENT|paging::PAGE_WRITE)}{unsafe{core::ptr::write_bytes(paging::TEMP_PHYS_MAP as *mut u8,0,PAGE_SIZE);paging::unmap_page(c,paging::TEMP_PHYS_MAP);}}}
        p+=PAGE_SIZE;left-=1;
    }
}
#[repr(C)] struct PageListNode{next:usize,count:usize,pages:[usize;510]}
#[derive(Clone,Copy)] pub struct PageList{head:usize,tail:usize,count:usize}
impl PageList{
    pub const fn empty()->Self{Self{head:0,tail:0,count:0}}
    pub fn count(&self)->usize{self.count}
    pub fn push(&mut self,page:usize)->bool{unsafe{
        if self.tail==0{let n=match alloc_page_below(LOW_PHYS_LIMIT){Some(v)=>v,None=>return false};zero_pages(n,1);self.head=n;self.tail=n;}
        let mut node=self.tail as *mut PageListNode;
        if (*node).count==510{let n=match alloc_page_below(LOW_PHYS_LIMIT){Some(v)=>v,None=>return false};zero_pages(n,1);(*node).next=n;self.tail=n;node=n;}
        let i=(*node).count;(*node).pages[i]=page;(*node).count=i+1;self.count+=1;true
    }}
}
pub fn free_page_list(list:PageList){unsafe{let mut n=list.head;while n!=0{let node=n as *mut PageListNode;let next=(*node).next;let mut i=0;while i<(*node).count{free_page((*node).pages[i]);i+=1;}free_page(n);n=next;}}}
