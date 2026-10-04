//! Clean SMP-1 bootstrap for the AH532.
//!
//! Phase 1 deliberately stops at AP -> Rust entry:
//!   topology -> broadcast INIT/SIPI -> 32-bit -> long mode -> per-AP stack
//!   -> ap_kernel_entry -> ONLINE.
//!
//! There is no scheduler, run queue, process dispatch, migration token or
//! mailbox job path in the AP bootstrap.

use core::sync::atomic::{AtomicU32, Ordering};
use crate::serial;

pub const LAPIC_BASE: usize = 0xFEE0_0000;
pub const SHARED: usize = 0x7000;
pub const TRAMPOLINE: usize = 0x8000;
pub const AP_PML4: usize = 0xA000;
pub const AP_PDPT: usize = 0xB000;
pub const AP_PD: usize = 0xC000;
pub const AP_QUANTUM: u32 = 10; // legacy shell compatibility; unused by SMP-1
const MAX_AP: usize = 8;
const AP_PT_LIMIT: usize = 0x4000_0000; // 1 GiB identity map
const GDTR: usize = SHARED + 0x20;
const GDT: usize = SHARED + 0x30;
const STACK_COUNTER: usize = SHARED + 0x300;
const STACK_TABLE: usize = SHARED + 0x400;
const APIC_TABLE: usize = SHARED + 0xC00;
const READY_TABLE: usize = SHARED + 0xD00;
const STATUS_MAGIC: usize = SHARED + 0x14;

static mut DETECTED: usize = 1;
static ONLINE: AtomicU32 = AtomicU32::new(1);
static RQ_LOCK: AtomicU32 = AtomicU32::new(0);

#[inline(always)]
unsafe fn mmio_read32(addr: usize) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}
#[inline(always)]
unsafe fn mmio_write32(addr: usize, value: u32) {
    core::ptr::write_volatile(addr as *mut u32, value)
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
fn cpuid(leaf: u32, subleaf: u32) -> [u32; 4] {
    let r = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
    [r.eax, r.ebx, r.ecx, r.edx]
}
#[inline(always)]
fn bsp_apic_id() -> u8 {
    ((cpuid(1, 0)[1] >> 24) & 0xff) as u8
}

fn detect_topology() -> usize {
    let max = cpuid(0, 0)[0];
    if max >= 0xB {
        let r = cpuid(0xB, 1);
        if r[1] != 0 && r[2] != 0 {
            return (r[1] as usize).min(MAX_AP);
        }
    }
    let n = ((cpuid(1, 0)[1] >> 16) & 0xff) as usize;
    n.clamp(1, MAX_AP)
}

pub fn init() {
    let n = detect_topology();
    unsafe {
        DETECTED = n;
        ONLINE = 1;
    }
    mb_write32(0x10, 1);
    serial::write_str("[SMP] PHASE0 logical CPUs=");
    serial::write_usize(n);
    serial::write_str(" BSP APIC=");
    serial::write_usize(bsp_apic_id() as usize);
    serial::write_str(" ONLINE=1\n");
    if probe_lapic_only() {
        serial::write_str("[SMP] PHASE0 LAPIC=READY\n");
    } else {
        serial::write_str("[SMP] PHASE0 LAPIC=FAIL\n");
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

pub fn online_count() -> u32 {
    ONLINE.load(Ordering::Acquire)
}

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
        // PML4[0] -> PDPT, PDPT[0] -> PD. The PD contains 512 x 2 MiB
        // identity mappings, covering 0..1 GiB. This is deliberately wider
        // than the old 0..8 MiB map so the real kernel entry/allocated stack
        // cannot silently fall outside the AP address space.
        core::ptr::write_volatile(pml4, (AP_PDPT as u64) | 0x03);
        core::ptr::write_volatile(pdpt, (AP_PD as u64) | 0x03);
        i = 0;
        while i < 512 {
            core::ptr::write_volatile(pd.add(i), ((i * 0x20_0000) as u64) | 0x83);
            i += 1;
        }
    }
    mb_write64(0x00, AP_PML4 as u64);
}

fn write_gdt() {
    unsafe {
        let gdt = GDT as *mut u64;
        core::ptr::write_volatile(gdt.add(0), 0);
        core::ptr::write_volatile(gdt.add(1), 0x00CF9A000000FFFF); // 32-bit code
        core::ptr::write_volatile(gdt.add(2), 0x00CF92000000FFFF); // 32-bit data
        core::ptr::write_volatile(gdt.add(3), 0x00AF9A000000FFFF); // 64-bit code
        core::ptr::write_volatile(gdt.add(4), 0x00AF92000000FFFF); // 64-bit data
        core::ptr::write_volatile((GDTR) as *mut u16, 39);
        core::ptr::write_volatile((GDTR + 2) as *mut u32, GDT as u32);
    }
}

// Emit the trampoline from symbolic stages rather than maintaining hand-counted
// relative branches. All far-jump targets are patched from the actual labels.
fn build_trampoline() {
    let mut code = [0u8; 256];
    let mut p = 0usize;

    macro_rules! e8 { ($($x:expr),* $(,)?) => { $(code[p] = $x; p += 1;)* }; }
    macro_rules! e16 { ($x:expr) => {{
        let v = $x as u16;
        code[p] = v as u8; code[p+1] = (v >> 8) as u8; p += 2;
    }}; }
    macro_rules! e32 { ($x:expr) => {{
        let v = $x as u32;
        code[p] = v as u8; code[p+1] = (v >> 8) as u8;
        code[p+2] = (v >> 16) as u8; code[p+3] = (v >> 24) as u8; p += 4;
    }}; }

    e8!(0xFA);                         // CLI
    e8!(0x0F, 0x01, 0x16); e16!(GDTR); // LGDT [gdtr]
    e8!(0x0F, 0x20, 0xC0);             // MOV EAX,CR0
    e8!(0x83, 0xC8, 0x01);             // OR EAX,1 (PE)
    e8!(0x0F, 0x22, 0xC0);             // MOV CR0,EAX
    e8!(0xEA); let pm_fix = p; e32!(0); e16!(0x08);

    let pm32 = p;
    e8!(0x0F, 0x20, 0xE0);             // CR4
    e8!(0x83, 0xC8, 0x20);             // PAE
    e8!(0x0F, 0x22, 0xE0);
    e8!(0xA1); e32!(SHARED);            // EAX=[AP_PML4]
    e8!(0x0F, 0x22, 0xD8);             // CR3
    e8!(0xB9); e32!(0xC000_0080);       // EFER
    e8!(0x0F, 0x32);
    e8!(0x0D); e32!(0x0000_0100);       // LME
    e8!(0x0F, 0x30);
    e8!(0x0F, 0x20, 0xC0);             // CR0
    e8!(0x0D); e32!(0x8000_0000);       // PG
    e8!(0x0F, 0x22, 0xC0);
    e8!(0xEA); let lm_fix = p; e32!(0); e16!(0x18);

    let lm64 = p;
    e8!(0x66, 0xB8); e16!(0x20);       // data selector
    e8!(0x8E, 0xD8, 0x8E, 0xC0, 0x8E, 0xD0); // DS/ES/SS
    // Atomically claim one preallocated AP stack. This happens before Rust,
    // so APs do not share a stack and no APIC-ID ordering is assumed.
    e8!(0x31, 0xC0);                    // EAX=0
    e8!(0xF0, 0x0F, 0xC1, 0x04, 0x25); e32!(STACK_COUNTER); // LOCK XADD [counter],EAX
    e8!(0x48, 0xC1, 0xE0, 0x03);        // RAX *= 8
    e8!(0x48, 0x8B, 0x24, 0xC5); e32!(STACK_TABLE); // RSP=[table+RAX*8]
    e8!(0x48, 0x8B, 0x04, 0x25); e32!(SHARED + 0x28); // RAX=[entry]
    e8!(0x48, 0x85, 0xC0, 0x74, 0x02, 0xFF, 0xE0);
    e8!(0xF4, 0xEB, 0xFD);

    let pm_phys = TRAMPOLINE + pm32;
    let lm_phys = TRAMPOLINE + lm64;
    code[pm_fix..pm_fix+4].copy_from_slice(&(pm_phys as u32).to_le_bytes());
    code[lm_fix..lm_fix+4].copy_from_slice(&(lm_phys as u32).to_le_bytes());

    unsafe {
        let dst = TRAMPOLINE as *mut u8;
        let mut i = 0usize;
        while i < code.len() {
            core::ptr::write_volatile(dst.add(i), 0);
            i += 1;
        }
        i = 0;
        while i < p {
            core::ptr::write_volatile(dst.add(i), code[i]);
            i += 1;
        }
    }
    serial::write_str("[SMP] PHASE1 trampoline bytes=");
    serial::write_usize(p);
    serial::write_str(" PM32=");
    serial::write_hex(pm_phys);
    serial::write_str(" LM64=");
    serial::write_hex(lm_phys);
    serial::write_str("\n");
}

fn prepare_ap_stacks(count: usize) -> bool {
    unsafe { core::ptr::write_volatile(STACK_COUNTER as *mut u32, 0); }
    let mut i = 0usize;
    while i < count {
        let page = match crate::mm::alloc_page() {
            Some(p) if p < AP_PT_LIMIT => p,
            _ => {
                serial::write_str("[SMP] AP stack allocation FAIL\n");
                return false;
            }
        };
        unsafe {
            core::ptr::write_volatile((STACK_TABLE + i * 8) as *mut u64, (page + 4096) as u64);
        }
        i += 1;
    }
    true
}

unsafe fn lapic_broadcast(command: u32) {
    // Destination shorthand 11b = all APs excluding the BSP.
    mmio_write32(LAPIC_BASE + 0x310, 0);
    mmio_write32(LAPIC_BASE + 0x300, command);
    delay();
}

fn delay() {
    let mut n = 0usize;
    while n < 50_000 { core::hint::spin_loop(); n += 1; }
}

fn clear_boot_state() {
    let mut i = 0usize;
    while i < MAX_AP {
        unsafe {
            core::ptr::write_volatile((APIC_TABLE + i * 4) as *mut u32, 0);
        }
        i += 1;
    }
}

fn ap_entry_phys() -> usize {
    ap_kernel_entry as usize
}

fn wait_for_aps(target: usize, spins: usize) -> usize {
    let mut n = 0usize;
    while n < spins {
        let online = ONLINE.load(Ordering::Acquire) as usize
        if online >= target { return online; }
        core::hint::spin_loop();
        n += 1;
    }
    ONLINE.load(Ordering::Acquire) as usize
}

pub fn stage_all_aps() -> bool {
    let detected = unsafe { DETECTED };
    if detected <= 1 { return true; }
    if !probe_lapic_only() {
        serial::write_str("[SMP] PHASE1 LAPIC=FAIL\n");
        return false;
    }

    let entry = ap_entry_phys();
    if entry >= AP_PT_LIMIT {
        serial::write_str("[SMP] PHASE1 REFUSE ENTRY_UNMAPPED entry=");
        serial::write_hex(entry);
        serial::write_str("\n");
        return false;
    }

    build_ap_page_tables();
    write_gdt();
    clear_boot_state();

    // One stack per AP, not one stack per APIC-ID assumption.
    if !prepare_ap_stacks(detected - 1) {
        return false;
    }

    build_trampoline();
    mb_write32(STATUS_MAGIC - SHARED, 0x4150_4254); // APBT
    mb_write32(0x18, 0); // reserved legacy status fields
    mb_write32(0x1C, 0);
    mb_write64(0x28, entry as u64);

    unsafe {
        lapic_broadcast(0x000C_4500); // INIT assert, all APs
        lapic_broadcast(0x0000_C500); // INIT deassert, all APs
        lapic_broadcast(0x000C_0608); // SIPI #1, vector 8
    }

    let first = wait_for_aps(detected, 2_000_000);
    if first < detected {
        serial::write_str("[SMP] PHASE1 SIPI1 incomplete ONLINE=");
        serial::write_usize(first);
        serial::write_str(" -> SIPI2\n");
        unsafe { lapic_broadcast(0x000C_0608); }
    }

    let final_online = wait_for_aps(detected, 2_000_000);
    serial::write_str("[SMP] PHASE2 AP ENTRY ONLINE=");
    serial::write_usize(final_online);
    serial::write_str("/");
    serial::write_usize(detected);
    serial::write_str("\n");
    final_online == detected
}

#[no_mangle]
pub extern "C" fn ap_kernel_entry() -> ! {
    let r = cpuid(1, 0);
    let apic = ((r[1] >> 24) & 0xff) as usize;

    // The bootstrap stack is already unique. From here on every AP has a real
    // Rust stack and can safely publish its identity before entering idle.
    // APIC ID is the hardware identity. Do not derive it from startup order.
    // The APIC table has 256 four-byte slots and fits entirely in the shared
    // low-memory page below 0x8000. Store APIC_ID+1 so zero remains "not ready".
    if apic < 256 {
        unsafe {
            core::ptr::write_volatile((APIC_TABLE + apic * 4) as *mut u32, (apic + 1) as u32);
        }
        ONLINE.fetch_add(1, Ordering::AcqRel);
    }

    serial::write_str("[SMP] AP LONG64 READY APIC=");
    serial::write_usize(apic);
    serial::write_str("\n");

    loop {
        unsafe { core::arch::asm!("hlt", options(nostack, preserves_flags)); }
    }
}

// Phase 1 deliberately does not expose scheduler/RQ operations.
// Keep these compatibility shims so the old terminal commands cannot
// accidentally execute the old CPU19/CPU20 scheduler prototype.
pub fn migrate_token(_apic: usize, _pid: usize) -> bool { false }
pub fn balance_enqueue(_pid: usize) -> usize { 0 }
pub fn rq_len(_cpu: usize) -> usize { 0 }
pub fn rq_enqueue(_cpu: usize, _pid: usize) -> bool { false }

fn rq_lock() {
    while RQ_LOCK.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}
fn rq_unlock() {
    RQ_LOCK.store(0, Ordering::Release);
}
