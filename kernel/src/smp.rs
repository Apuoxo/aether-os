//! Aether SMP foundation.
//!
//! This module intentionally contains only the safe BSP-side foundation.
//! AP startup is not attempted until the memory, paging, interrupt and per-CPU
//! contracts are established. Keeping this boundary small prevents the old
//! experimental AP trampoline/scheduler code from affecting normal boot.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::serial;

pub const LAPIC_BASE: usize = 0xFEE0_0000;
const MAX_LOGICAL_CPUS: usize = 8;

static DETECTED_CPUS: AtomicU32 = AtomicU32::new(1);

#[inline(always)]
fn cpuid(leaf: u32, subleaf: u32) -> [u32; 4] {
    let r = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
    [r.eax, r.ebx, r.ecx, r.edx]
}

/// Return the BSP's xAPIC ID as reported by CPUID.01H.
#[inline(always)]
pub fn bsp_apic_id() -> u8 {
    ((cpuid(1, 0)[1] >> 24) & 0xff) as u8
}

/// Detect logical processors without starting any AP.
///
/// CPUID.0BH is preferred when available. Leaf 1 is retained as the
/// compatibility fallback needed by older x86 hardware.
pub fn detect_topology() -> usize {
    let max_basic = cpuid(0, 0)[0];

    if max_basic >= 0x0B {
        let r = cpuid(0x0B, 1);
        let logical = r[1] as usize;
        if logical != 0 {
            return logical.clamp(1, MAX_LOGICAL_CPUS);
        }
    }

    let logical = ((cpuid(1, 0)[1] >> 16) & 0xff) as usize;
    logical.clamp(1, MAX_LOGICAL_CPUS)
}

/// Verify that the current page tables can access the local APIC MMIO page.
///
/// This is deliberately a read-only probe. It does not program the LAPIC and
/// therefore cannot send INIT/SIPI or otherwise change CPU execution state.
pub fn probe_lapic() -> bool {
    let cr3 = unsafe { crate::mm::paging::read_cr3() };

    let mapped = unsafe {
        crate::mm::paging::map_page(
            cr3,
            LAPIC_BASE,
            LAPIC_BASE,
            crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE,
        )
    };

    if !mapped {
        return false;
    }

    unsafe { crate::mm::paging::load_cr3(cr3); }

    let version = unsafe {
        core::ptr::read_volatile((LAPIC_BASE + 0x30) as *const u32)
    };

    version != 0xffff_ffff
}

/// Initialize only the BSP-side SMP foundation.
///
/// No AP is started here. The normal Aether boot path therefore remains
/// unchanged while the new SMP implementation is built and validated.
pub fn init() {
    let detected = detect_topology();
    DETECTED_CPUS.store(detected as u32, Ordering::Release);

    serial::write_str("[SMP] FOUNDATION logical_cpus=");
    serial::write_usize(detected);
    serial::write_str(" bsp_apic=");
    serial::write_usize(bsp_apic_id() as usize);
    serial::write_str("\n");

    if probe_lapic() {
        serial::write_str("[SMP] FOUNDATION LAPIC=READY\n");
    } else {
        serial::write_str("[SMP] FOUNDATION LAPIC=FAIL\n");
    }
}

#[inline]
pub fn detected_cpus() -> u32 {
    DETECTED_CPUS.load(Ordering::Acquire)
}

#[inline]
pub fn online_cpus() -> u32 {
    // Until AP bootstrap exists, only the BSP is legitimately online.
    1
}
