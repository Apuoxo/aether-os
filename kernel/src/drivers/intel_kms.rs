//! Intel Gen6 Sandy Bridge display MMIO bring-up — Stage 3.
//! MMIO identity mapping + bounded forcewake + read-only register dump.
//! No GGTT/GSM, no modeset, no EDID, no infinite waits.

use crate::{mm, serial};
use crate::drivers::intel_igpu;

const PCI_ADDR: u16 = 0x0CF8;
const PCI_DATA: u16 = 0x0CFC;

const FORCEWAKE_MT: usize = 0x0A188;
const FORCEWAKE_MT_ACK: usize = 0x130040;
const FORCEWAKE: usize = 0x0A18C;
const FORCEWAKE_ACK: usize = 0x130090;

const PIPEASRC: usize = 0x6001C;
const PIPEACONF: usize = 0x70008;
const PIPEASTAT: usize = 0x70024;
const PIPEADSL: usize = 0x70000;

static mut MMIO_BASE: usize = 0;
static mut MMIO_READY: bool = false;
static mut FORCEWAKE_READY: bool = false;

unsafe fn pci_read32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") PCI_ADDR, in("eax") a,
        options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") PCI_DATA, out("eax") v,
        options(nostack, preserves_flags));
    v
}

unsafe fn pci_write32(bus: u8, dev: u8, func: u8, off: u8, value: u32) {
    let a = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") PCI_ADDR, in("eax") a,
        options(nostack, preserves_flags));
    core::arch::asm!("out dx, eax", in("dx") PCI_DATA, in("eax") value,
        options(nostack, preserves_flags));
}

unsafe fn mmio_read32(off: usize) -> u32 {
    core::ptr::read_volatile((MMIO_BASE + off) as *const u32)
}

unsafe fn mmio_write32(off: usize, value: u32) {
    core::ptr::write_volatile((MMIO_BASE + off) as *mut u32, value);
}

fn log_reg(name: &str, off: usize, value: u32) {
    serial::write_str("[KMS] ");
    serial::write_str(name);
    serial::write_str("=");
    serial::write_hex(value as usize);
    serial::write_str(" @");
    serial::write_hex(off);
    serial::write_str("\n");
}

/// Identity-map the MMIO BAR through the current kernel page tables.
/// The MMIO window is deliberately limited to 2 MiB: enough for the Gen6
/// display registers used in this stage, without introducing an aperture/GGTT.
unsafe fn map_mmio(base: usize) -> bool {
    if base == 0 || (base & 0xFFF) != 0 {
        return false;
    }

    let cr3 = crate::mm::paging::kernel_cr3();
    if cr3 == 0 {
        return false;
    }

    let mut off = 0usize;
    while off < 0x20_0000 {
        if !crate::mm::paging::map_page(
            cr3,
            base + off,
            base + off,
            crate::mm::paging::PAGE_PRESENT
                | crate::mm::paging::PAGE_WRITE
                | crate::mm::paging::PAGE_PCD
                | crate::mm::paging::PAGE_PWT,
        ) {
            return false;
        }
        off += crate::mm::paging::PAGE_SIZE;
    }
    crate::mm::paging::load_cr3(cr3);
    true
}

/// Enable PCI memory decoding if firmware left it disabled.
unsafe fn enable_memory_space() {
    let g = intel_igpu::info();
    let old = pci_read32(g.bus, g.dev, g.func, 0x04);
    let new = old | (1 << 1);
    if new != old {
        pci_write32(g.bus, g.dev, g.func, 0x04, new);
        serial::write_str("[KMS] PCI MEM=EN\n");
    } else {
        serial::write_str("[KMS] PCI MEM=already\n");
    }
}

/// Gen6 forcewake with a hard upper bound. No CLI and no unbounded polling.
pub fn forcewake_get() -> bool {
    unsafe {
        if !MMIO_READY {
            return false;
        }

        mmio_write32(FORCEWAKE_MT, 0x0001_0001);
        let mut i = 0usize;
        while i < 50_000 {
            if mmio_read32(FORCEWAKE_MT_ACK) & 1 != 0 {
                FORCEWAKE_READY = true;
                return true;
            }
            core::hint::spin_loop();
            i += 1;
        }

        // Fall back to legacy Gen6 forcewake path.
        mmio_write32(FORCEWAKE, 0x0000_0001);
        i = 0;
        while i < 50_000 {
            if mmio_read32(FORCEWAKE_ACK) & 1 != 0 {
                FORCEWAKE_READY = true;
                return true;
            }
            core::hint::spin_loop();
            i += 1;
        }

        FORCEWAKE_READY = false;
        false
    }
}

pub fn forcewake_put() {
    unsafe {
        if !MMIO_READY {
            return;
        }
        mmio_write32(FORCEWAKE_MT, 0x0001_0000);
        mmio_write32(FORCEWAKE, 0);
        FORCEWAKE_READY = false;
    }
}

/// Stage 3: map display MMIO, bounded-forcewake, and dump the Gen6 Pipe A state.
/// This function never programs timing or plane registers.
pub fn init() -> bool {
    unsafe {
        MMIO_READY = false;
        FORCEWAKE_READY = false;

        if !intel_igpu::ready() || intel_igpu::gen() != 6 {
            serial::write_str("[KMS] SKIP: Intel Gen6 iGPU not ready\n");
            return false;
        }

        let base = intel_igpu::mmio_bar() as usize;
        serial::write_str("[KMS] MMIO BAR0=");
        serial::write_hex(base);
        serial::write_str("\n");

        enable_memory_space();

        if !map_mmio(base) {
            serial::write_str("[KMS] MMIO MAP=FAIL (LFB path preserved)\n");
            return false;
        }

        MMIO_BASE = base;
        MMIO_READY = true;
        serial::write_str("[KMS] MMIO MAP=READY size=0x200000\n");

        if !forcewake_get() {
            serial::write_str("[KMS] FORCEWAKE=TIMEOUT\n");
            return false;
        }
        serial::write_str("[KMS] FORCEWAKE=OK\n");

        log_reg("PIPEASRC", PIPEASRC, mmio_read32(PIPEASRC));
        log_reg("PIPEACONF", PIPEACONF, mmio_read32(PIPEACONF));
        log_reg("PIPEASTAT", PIPEASTAT, mmio_read32(PIPEASTAT));
        log_reg("PIPEADSL", PIPEADSL, mmio_read32(PIPEADSL));

        forcewake_put();
        serial::write_str("[KMS] STAGE3=READY\n");
        true
    }
}

pub fn ready() -> bool { unsafe { MMIO_READY } }
pub fn forcewake_ready() -> bool { unsafe { FORCEWAKE_READY } }
pub fn mmio_base() -> usize { unsafe { MMIO_BASE } }
