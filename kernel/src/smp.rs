//! Minimal opt-in SMP bring-up for Sandy Bridge AH532.
//! BSP stays single-core at boot. CPU6 explicitly starts APs with one SIPI.

use core::sync::atomic::{AtomicU32, Ordering};
use crate::serial;

pub const LAPIC_BASE: usize = 0xFEE0_0000;
pub const SHARED: usize = 0x7000;
pub const TRAMPOLINE: usize = 0x8000;
pub const AP_PML4: usize = 0xA000;
pub const AP_PDPT: usize = 0xB000;
pub const AP_PD: usize = 0xC000;
pub const AP_QUANTUM: u32 = 10;
const MAX_AP: usize = 8;
const RQ_CAP: usize = 8;

static mut DETECTED: usize = 1;
static mut AP_IDS: [u8; MAX_AP] = [0; MAX_AP];
static mut ONLINE: u32 = 1;
static RQ_LOCK: AtomicU32 = AtomicU32::new(0);
static mut RQ: [[usize; RQ_CAP]; MAX_AP] = [[0; RQ_CAP]; MAX_AP];
static mut RQ_LEN: [usize; MAX_AP] = [0; MAX_AP];

#[inline(always)]
unsafe fn mmio_read32(addr: usize) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}
#[inline(always)]
unsafe fn mmio_write32(addr: usize, value: u32) {
    core::ptr::write_volatile(addr as *mut u32, value);
}
#[inline(always)]
fn mb_read32(off: usize) -> u32 {
    unsafe { core::ptr::read_volatile((SHARED + off) as *const u32) }
}
#[inline(always)]
fn mb_write32(off: usize, value: u32) {
    unsafe { core::ptr::write_volatile((SHARED + off) as *mut u32, value) }
}
#[inline(always)]
fn mb_write64(off: usize, value: u64) {
    unsafe { core::ptr::write_volatile((SHARED + off) as *mut u64, value) }
}
#[inline(always)]
fn mb_read64(off: usize) -> u64 {
    unsafe { core::ptr::read_volatile((SHARED + off) as *const u64) }
}

#[inline(always)]
fn cpuid(leaf: u32, subleaf: u32) -> [u32; 4] {
    let r = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
    [r.eax, r.ebx, r.ecx, r.edx]
}
fn bsp_apic_id() -> u8 { ((cpuid(1,0)[1] >> 24) & 0xff) as u8 }

fn detect_topology() -> usize {
    let max = cpuid(0,0)[0];
    if max >= 0xB {
        let r = cpuid(0xB,0);
        if r[2] != 0 && r[1] != 0 {
            return (r[1] as usize).min(MAX_AP);
        }
    }
    ((cpuid(1,0)[1] >> 16) as usize & 0xff).min(MAX_AP)
}

pub fn init() {
    let n = detect_topology();
    unsafe {
        DETECTED = if n == 0 { 1 } else { n };
        AP_IDS = [0; MAX_AP];
        AP_IDS[0] = bsp_apic_id();
        let mut i = 1usize;
        while i < DETECTED { AP_IDS[i] = i as u8; i += 1; }
        ONLINE = 1;
    }
    mb_write32(0x10, 1);
    // Detect only. CPU6 owns AP startup.
    if probe_lapic_only() {
        serial::write_str("[SMP] detect logical CPUs=");
        serial::write_usize(unsafe { DETECTED });
        serial::write_str(" BSP=");
        serial::write_usize(bsp_apic_id() as usize);
        serial::write_str(" ONLINE=1 (opt-in)\n");
    } else {
        serial::write_str("[SMP] LAPIC probe failed\n");
    }
}

pub fn probe_lapic_only() -> bool {
    let cr3 = unsafe { crate::mm::paging::read_cr3() };
    let mapped = unsafe {
        crate::mm::paging::map_page(
            cr3,
            LAPIC_BASE,
            LAPIC_BASE,
            crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE,
        )
    };
    if !mapped { return false; }
    unsafe { crate::mm::paging::load_cr3(cr3); }
    let id = unsafe { mmio_read32(LAPIC_BASE + 0x20) };
    id != 0xffff_ffff
}

pub fn online_count() -> u32 { let n = mb_read32(0x10); if n == 0 { 1 } else { n } }

fn build_ap_page_tables() {
    unsafe {
        let pml4 = AP_PML4 as *mut u64;
        let pdpt = AP_PDPT as *mut u64;
        let pd = AP_PD as *mut u64;
        let mut i = 0usize;
        while i < 512 {
            core::ptr::write_volatile(pml4.add(i), 0);
            core::ptr::write_volatile(pdpt.add(i), 0);
            core::ptr::write_volatile(pd.add(i), 0);
            i += 1;
        }
        core::ptr::write_volatile(pml4, (AP_PDPT as u64) | 0x03);
        core::ptr::write_volatile(pdpt, (AP_PD as u64) | 0x03);
        i = 0;
        while i < 4 {
            core::ptr::write_volatile(pd.add(i), ((i * 0x20_0000) as u64) | 0x83);
            i += 1;
        }
    }
    mb_write64(0x00, AP_PML4 as u64);
}

fn write_gdt() {
    unsafe {
        let gdt = (SHARED + 0x30) as *mut u64;
        core::ptr::write_volatile(gdt.add(0), 0);
        core::ptr::write_volatile(gdt.add(1), 0x00CF9A000000FFFF);
        core::ptr::write_volatile(gdt.add(2), 0x00CF92000000FFFF);
        core::ptr::write_volatile(gdt.add(3), 0x00AF9A000000FFFF);
        core::ptr::write_volatile(gdt.add(4), 0x00AF92000000FFFF);
        core::ptr::write_volatile((SHARED + 0x20) as *mut u16, 39);
        core::ptr::write_volatile((SHARED + 0x22) as *mut u32, (SHARED + 0x30) as u32);
    }
}

fn build_trampoline() {
    const CODE: [u8; 106] = [
        0xFA,0x0F,0x01,0x16,0x20,0x70,0x0F,0x20,0xC0,0x83,0xC8,0x01,0x0F,0x22,0xC0,
        0xE9,0x0C,0x80,0xFF,0xFF,0x0F,0x20,0xE0,0x83,0xC8,0x20,0x0F,0x22,0xE0,
        0xA1,0x00,0x70,0x00,0x00,0x0F,0x22,0xD8,0xB9,0x80,0x00,0x00,0xC0,0x0F,0x32,
        0x0D,0x00,0x01,0x00,0x00,0x0F,0x30,0x0F,0x20,0xC0,0x0D,0x00,0x00,0x00,0x80,
        0x0F,0x22,0xC0,0xEA,0x45,0x80,0x00,0x00,0x18,0x00,0x48,0x8B,0x04,0x25,
        0x08,0x70,0x00,0x00,0x48,0x89,0xC4,0xF0,0xFF,0x04,0x25,0x10,0x70,0x00,0x00,
        0x48,0x8B,0x04,0x25,0x28,0x70,0x00,0x00,0x48,0x85,0xC0,0x74,0x02,0xFF,0xE0,
        0xF4,0xEB,0xFD
    ];
    unsafe {
        let dst = TRAMPOLINE as *mut u8;
        let mut i = 0usize;
        while i < CODE.len() { core::ptr::write_volatile(dst.add(i), CODE[i]); i += 1; }
    }
}

fn delay() { let mut n=0usize; while n<50_000 { core::hint::spin_loop(); n+=1; } }

unsafe fn lapic_icr(apic: u8, command: u32) {
    mmio_write32(LAPIC_BASE + 0x310, (apic as u32) << 24);
    mmio_write32(LAPIC_BASE + 0x300, command);
    delay();
}

fn start_one(apic: u8) {
    mb_write32(0x80 + (apic as usize) * 4, 0);
    mb_write32(0xA0 + (apic as usize) * 4, 0);
    mb_write32(0xC0 + (apic as usize) * 4, 0);
    mb_write32(0xE0 + (apic as usize) * 4, 0);
    let stack = 0x8F000usize.saturating_sub((apic as usize).saturating_sub(1) * 0x1000);
    mb_write64(0x08, stack as u64);
    unsafe {
        lapic_icr(apic, 0x0000_C500);
        lapic_icr(apic, 0x0000_8500);
        // Exactly one SIPI. Vector 8 == physical 0x8000.
        lapic_icr(apic, 0x0000_0608);
    }
    let mut n=0usize;
    while n<2_000_000 {
        if mb_read32(0x80 + (apic as usize)*4) != 0 { break; }
        core::hint::spin_loop(); n+=1;
    }
}

pub fn stage_all_aps() -> bool {
    let detected = unsafe { DETECTED };
    if online_count() >= detected as u32 { return true; }
    if !probe_lapic_only() { return false; }
    build_ap_page_tables();
    write_gdt();
    build_trampoline();
    mb_write32(0x14, 0x4150_4552);
    mb_write32(0x18, 0);
    mb_write32(0x1C, 0);
    mb_write64(0x28, ap_kernel_entry as usize as u64);
    let mut i=1usize;
    while i < unsafe { DETECTED } {
        let apic = unsafe { AP_IDS[i] };
        if apic != bsp_apic_id() { start_one(apic); }
        i+=1;
    }
    online_count() == detected as u32
}

fn rq_lock() {
    while RQ_LOCK.compare_exchange(0,1,Ordering::Acquire,Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}
fn rq_unlock() { RQ_LOCK.store(0,Ordering::Release); }

pub fn rq_len(cpu: usize) -> usize {
    if cpu >= MAX_AP { return 0; }
    rq_lock();
    let n=unsafe{RQ_LEN[cpu]};
    rq_unlock(); n
}

pub fn rq_enqueue(cpu: usize, pid: usize) -> bool {
    if cpu == 0 || cpu >= MAX_AP || pid == 0 { return false; }
    rq_lock();
    let ok=unsafe {
        if RQ_LEN[cpu] >= RQ_CAP { false } else {
            RQ[cpu][RQ_LEN[cpu]]=pid; RQ_LEN[cpu]+=1; true
        }
    };
    rq_unlock(); ok
}

fn rq_dequeue(cpu: usize) -> Option<usize> {
    if cpu >= MAX_AP { return None; }
    rq_lock();
    let out=unsafe {
        if RQ_LEN[cpu] == 0 { None } else {
            let p=RQ[cpu][0]; let mut i=1usize;
            while i<RQ_LEN[cpu] { RQ[cpu][i-1]=RQ[cpu][i]; i+=1; }
            RQ_LEN[cpu]-=1; Some(p)
        }
    };
    rq_unlock(); out
}

pub fn migrate_token(apic: usize, pid: usize) -> bool {
    if apic == 0 || apic >= MAX_AP || pid == 0 || online_count() < 4 { return false; }
    let done_before = mb_read32(0x80 + apic*4);
    mb_write32(0xE0 + apic*4, pid as u32);
    mb_write32(0xA0 + apic*4, 1);
    core::sync::atomic::fence(Ordering::SeqCst);
    let mut n=0usize;
    while n<5_000_000 {
        if mb_read32(0x80 + apic*4) != done_before {
            return mb_read32(0xC0 + apic*4) == pid as u32;
        }
        core::hint::spin_loop(); n+=1;
    }
    false
}

pub fn balance_enqueue(pid: usize) -> usize {
    if pid == 0 { return 0; }
    let detected = unsafe { DETECTED };
    let limit = if detected > MAX_AP { MAX_AP } else { detected };
    let mut best=0usize; let mut best_len=usize::MAX;
    let mut cpu=1usize;
    while cpu<limit {
        let n=rq_len(cpu);
        if n<best_len { best_len=n; best=cpu; }
        cpu+=1;
    }
    if best!=0 && rq_enqueue(best,pid) { best } else { 0 }
}

#[no_mangle]
pub extern "C" fn ap_kernel_entry() -> ! {
    let apic = ((cpuid(1,0)[1] >> 24) & 7) as usize;
    loop {
        let job = mb_read32(0xA0 + apic*4);
        if job != 0 {
            let pid=mb_read32(0xE0 + apic*4) as usize;
            if pid != 0 {
                mb_write32(0xA0 + apic*4, 0);
                mb_write32(0xC0 + apic*4, pid as u32);
                crate::process::account_run(pid, AP_QUANTUM as u64);
                mb_write32(0x80 + apic*4, mb_read32(0x80 + apic*4).wrapping_add(1));
            } else {
                mb_write32(0xA0 + apic*4, 0);
            }
        } else if let Some(pid)=rq_dequeue(apic) {
            crate::process::account_run(pid, AP_QUANTUM as u64);
        } else {
            core::hint::spin_loop();
        }
    }
}
