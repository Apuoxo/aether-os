//! Intel Sandy Bridge (Gen6) display driver for Aether OS.
//!
//! The driver is deliberately layered:
//!  1. PCI identification / BAR decoding.
//!  2. Guarded MMIO mapping of the programmed graphics BAR.
//!  3. Read-only display-engine snapshot.
//!  4. Existing Multiboot framebuffer remains the presentation surface.
//!
//! No PCI configuration writes, BAR sizing writes, display register writes,
//! GMBUS transactions, power-state changes or ring submissions are performed
//! automatically. This keeps the AH532 recovery path safe while giving the
//! driver enough hardware knowledge for the next active display stage.

use crate::graphics;
use crate::mm::paging;
use crate::serial;

const PCI_CONFIG_ADDR: u16 = 0x0CF8;
const PCI_CONFIG_DATA: u16 = 0x0CFC;
const INTEL_VENDOR: u16 = 0x8086;
const DISPLAY_CLASS: u8 = 0x03;
const HD3000_DID: u16 = 0x0116;

const PAGE_SIZE: usize = 4096;

// Sandy Bridge display registers (Gen6).
const PIPEACONF: usize = 0x70008;
const PIPEBCONF: usize = 0x71008;
const DSPACNTR: usize = 0x70180;
const DSPASTRIDE: usize = 0x70188;
const DSPASURF: usize = 0x7019C;
const DSPBCNTR: usize = 0x71180;
const DSPBSTRIDE: usize = 0x71188;
const DSPBSURF: usize = 0x7119C;
const GFX_MODE: usize = 0x70000;

// GMBUS block. Read-only probing is limited to register reads; starting a
// transaction requires writes and is intentionally deferred until validated.
const GMBUS0: usize = 0x5100;
const GMBUS1: usize = 0x5104;
const GMBUS2: usize = 0x5108;
const GMBUS3: usize = 0x510C;
const GMBUS4: usize = 0x5110;

#[derive(Copy, Clone)]
struct Bar {
    base: u64,
    is_io: bool,
    is_64: bool,
    prefetch: bool,
}

impl Bar {
    const fn empty() -> Self {
        Bar { base: 0, is_io: false, is_64: false, prefetch: false }
    }
}

#[derive(Copy, Clone)]
struct Gpu {
    bus: u8,
    dev: u8,
    func: u8,
    bar0: Bar,
    mmio: usize,
}

static mut GPU: Gpu = Gpu {
    bus: 0, dev: 0, func: 0, bar0: Bar::empty(), mmio: 0,
};

static mut GPU_READY: bool = false;
static mut TARGET_FOUND: bool = false;

unsafe fn pci_addr(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x8000_0000 | ((bus as u32) << 16) | ((dev as u32) << 11)
        | ((func as u32) << 8) | ((off as u32) & 0xFC)
}

unsafe fn pci_read32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let addr = pci_addr(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") PCI_CONFIG_ADDR, in("eax") addr,
        options(nostack, preserves_flags));
    let value: u32;
    core::arch::asm!("in eax, dx", in("dx") PCI_CONFIG_DATA, out("eax") value,
        options(nostack, preserves_flags));
    value
}

fn log_hex64(v: u64) {
    serial::write_hex((v >> 32) as usize);
    serial::write_str("_");
    serial::write_hex(v as usize);
}

fn decode_bar(raw: u32, upper: u32) -> Bar {
    if raw == 0 {
        return Bar::empty();
    }
    if raw & 1 != 0 {
        return Bar { base: (raw & !0x3) as u64, is_io: true, is_64: false, prefetch: false };
    }
    let kind = (raw >> 1) & 3;
    let base_lo = (raw & !0xF) as u64;
    let base = if kind == 2 {
        base_lo | ((upper as u64) << 32)
    } else {
        base_lo
    };
    Bar {
        base,
        is_io: false,
        is_64: kind == 2,
        prefetch: raw & 8 != 0,
    }
}

fn print_bar(index: u8, raw: u32, upper: u32) {
    let b = decode_bar(raw, upper);
    serial::write_str("[VIDEO/PCI] BAR");
    serial::write_usize(index as usize);
    serial::write_str(" raw=");
    serial::write_hex(raw as usize);
    if b.base == 0 {
        serial::write_str(" UNUSED\n");
        return;
    }
    if b.is_io {
        serial::write_str(" IO base=");
        serial::write_hex(b.base as usize);
    } else {
        serial::write_str(" MMIO base=");
        log_hex64(b.base);
        serial::write_str(if b.is_64 { " 64bit" } else { " 32bit" });
        serial::write_str(if b.prefetch { " prefetch" } else { " nonprefetch" });
    }
    serial::write_str("\n");
}

/// Map a physical MMIO range into the current identity address space.
/// Existing mappings are overwritten only with the same physical address.
unsafe fn map_mmio(phys: u64, len: usize) -> bool {
    if phys == 0 || len == 0 || phys > usize::MAX as u64 {
        return false;
    }
    let start = (phys as usize) & !(PAGE_SIZE - 1);
    let end = match start.checked_add(len.saturating_add(PAGE_SIZE - 1) & !(PAGE_SIZE - 1)) {
        Some(v) => v,
        None => return false,
    };
    let cr3 = paging::kernel_cr3();
    let mut va = start;
    while va < end {
        if !paging::map_page(
            cr3, va, va, paging::PAGE_PRESENT | paging::PAGE_WRITE
        ) {
            return false;
        }
        match va.checked_add(PAGE_SIZE) {
            Some(v) => va = v,
            None => return false,
        }
    }
    paging::load_cr3(cr3);
    true
}

unsafe fn mmio_read32(off: usize) -> u32 {
    core::ptr::read_volatile((GPU.mmio + off) as *const u32)
}

fn log_reg(name: &str, off: usize) {
    unsafe {
        let v = mmio_read32(off);
        serial::write_str("[VIDEO/MMIO] ");
        serial::write_str(name);
        serial::write_str(" @");
        serial::write_hex(off);
        serial::write_str("=");
        serial::write_hex(v as usize);
        serial::write_str("\n");
    }
}

/// Decode only stable/read-only information from the display engine.
fn snapshot_display() {
    serial::write_str("\n======== SANDY BRIDGE DISPLAY SNAPSHOT ========\n");
    unsafe {
        log_reg("GFX_MODE", GFX_MODE);
        log_reg("PIPEACONF", PIPEACONF);
        log_reg("PIPEBCONF", PIPEBCONF);
        log_reg("DSPACNTR", DSPACNTR);
        log_reg("DSPASTRIDE", DSPASTRIDE);
        log_reg("DSPASURF", DSPASURF);
        log_reg("DSPBCNTR", DSPBCNTR);
        log_reg("DSPBSTRIDE", DSPBSTRIDE);
        log_reg("DSPBSURF", DSPBSURF);

        serial::write_str("[VIDEO/GMBUS] GMBUS0="); serial::write_hex(mmio_read32(GMBUS0) as usize);
        serial::write_str(" GMBUS1="); serial::write_hex(mmio_read32(GMBUS1) as usize);
        serial::write_str(" GMBUS2="); serial::write_hex(mmio_read32(GMBUS2) as usize);
        serial::write_str(" GMBUS3="); serial::write_hex(mmio_read32(GMBUS3) as usize);
        serial::write_str(" GMBUS4="); serial::write_hex(mmio_read32(GMBUS4) as usize);
        serial::write_str("\n");
    }
}

/// Locate the Intel display controller and decode all programmed BARs.
/// This function is read-only with respect to PCI configuration space.
fn discover() -> bool {
    serial::write_str("\n======== INTEL VIDEO DISCOVERY ========\n");
    unsafe {
        GPU_READY = false;
        TARGET_FOUND = false;
    }

    for dev in 0u8..32 {
        for func in 0u8..8 {
            unsafe {
                let id = pci_read32(0, dev, func, 0);
                if id == 0 || id == 0xFFFF_FFFF {
                    continue;
                }
                let vendor = id as u16;
                let device = (id >> 16) as u16;
                let class_reg = pci_read32(0, dev, func, 0x08);
                if ((class_reg >> 24) as u8) != DISPLAY_CLASS {
                    continue;
                }

                let subclass = (class_reg >> 16) as u8;
                let prog_if = (class_reg >> 8) as u8;
                let command = pci_read32(0, dev, func, 0x04) as u16;

                serial::write_str("[VIDEO/PCI] 00:");
                serial::write_usize(dev as usize);
                serial::write_str(".");
                serial::write_usize(func as usize);
                serial::write_str(" VID=");
                serial::write_hex(vendor as usize);
                serial::write_str(" DID=");
                serial::write_hex(device as usize);
                serial::write_str(" class=");
                serial::write_hex(subclass as usize);
                serial::write_str("/");
                serial::write_hex(prog_if as usize);
                serial::write_str(" CMD=");
                serial::write_hex(command as usize);
                serial::write_str("\n");

                let mut bars = [Bar::empty(); 6];
                let mut i = 0u8;
                while i < 6 {
                    let raw = pci_read32(0, dev, func, 0x10 + i * 4);
                    let kind = if raw & 1 == 0 { (raw >> 1) & 3 } else { 0 };
                    let upper = if raw != 0 && raw & 1 == 0 && kind == 2 && i < 5 {
                        pci_read32(0, dev, func, 0x10 + (i + 1) * 4)
                    } else {
                        0
                    };
                    print_bar(i, raw, upper);
                    bars[i as usize] = decode_bar(raw, upper);
                    if kind == 2 && i < 5 {
                        bars[(i + 1) as usize] = Bar::empty();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }

                if vendor == INTEL_VENDOR && device == HD3000_DID {
                    TARGET_FOUND = true;
                    // Intel graphics normally exposes its device registers through
                    // a memory BAR. Do not assume which BAR is valid; select the
                    // first non-zero memory BAR and map only a guarded register window.
                    let mut selected = Bar::empty();
                    let mut bi = 0usize;
                    while bi < 6 {
                        if bars[bi].base != 0 && !bars[bi].is_io {
                            selected = bars[bi];
                            break;
                        }
                        bi += 1;
                    }

                    if selected.base != 0 && selected.base <= usize::MAX as u64 {
                        serial::write_str("[VIDEO/MMIO] candidate BAR base=");
                        log_hex64(selected.base);
                        serial::write_str("\n");
                        // Gen6 display registers used below are below 0x72000.
                        // Map only the required 1 MiB, not the whole BAR.
                        if map_mmio(selected.base, 0x100000) {
                            GPU = Gpu {
                                bus: 0, dev, func,
                                bar0: selected,
                                mmio: selected.base as usize,
                            };
                            GPU_READY = true;
                            serial::write_str("[VIDEO/MMIO] mapping PASS (1 MiB guarded window)\n");
                            snapshot_display();
                        } else {
                            serial::write_str("[VIDEO/MMIO] mapping FAIL — no register access\n");
                        }
                    } else {
                        serial::write_str("[VIDEO/MMIO] no usable memory BAR — no register access\n");
                    }
                }
            }
        }
    }

    unsafe { TARGET_FOUND }
}

/// Initialize the hardware probe while preserving the existing software FB.
pub fn init() -> bool {
    let found = discover();
    if graphics::ready() {
        serial::write_str("[VIDEO] software framebuffer remains active; hardware snapshot is diagnostic only\n");
    }
    found
}

pub fn ready() -> bool { graphics::ready() }
pub fn width() -> u32 { graphics::width() as u32 }
pub fn height() -> u32 { graphics::height() as u32 }

/// True only after a validated Intel BAR has been mapped and the MMIO snapshot
/// completed without being skipped. This is not a claim that the display is
/// driven by Aether yet.
pub fn hardware_ready() -> bool {
    unsafe { GPU_READY }
}

/// Explicitly requested hardware snapshot; no PCI/GPU writes are performed.
pub fn snapshot() {
    unsafe {
        if GPU_READY {
            snapshot_display();
        } else {
            serial::write_str("[VIDEO] snapshot skipped — MMIO not ready\n");
        }
    }
}


unsafe fn mmio_write32(off: usize, value: u32) {
    core::ptr::write_volatile((GPU.mmio + off) as *mut u32, value);
}

const GMBUS_PIN_DISABLED: u32 = 0;
const GMBUS_PIN_VGADDC: u32 = 2;
const GMBUS_PIN_PANEL: u32 = 3;
const GMBUS_PIN_DPC: u32 = 4;
const GMBUS_PIN_DPB: u32 = 5;
const GMBUS_PIN_DPD: u32 = 6;
const GMBUS_RATE_100KHZ: u32 = 0 << 8;
const GMBUS_SW_RDY: u32 = 1 << 30;
const GMBUS_CYCLE_STOP: u32 = 4 << 25;
const GMBUS_CYCLE_INDEX: u32 = 2 << 25;
const GMBUS_HW_RDY: u32 = 1 << 11;
const GMBUS_SATOER: u32 = 1 << 10;
const GMBUS_ACTIVE: u32 = 1 << 9;
const GMBUS_BYTE_COUNT_SHIFT: u32 = 16;
const GMBUS_SLAVE_ADDR_SHIFT: u32 = 1;
const GMBUS_SLAVE_READ: u32 = 1;
const GMBUS_SLAVE_EDID: u32 = 0x50;

unsafe fn wait_gmbus(mask: u32, want_set: bool, limit: u32) -> bool {
    let mut n = 0u32;
    while n < limit {
        let v = mmio_read32(GMBUS2);
        if want_set {
            if v & mask != 0 { return true; }
        } else if v & mask == 0 {
            return true;
        }
        core::hint::spin_loop();
        n += 1;
    }
    false
}

/// Explicit EDID read. Normal boot does not invoke this transaction.
pub fn read_edid(port: u32, out: &mut [u8; 128]) -> bool {
    unsafe {
        if !GPU_READY || !TARGET_FOUND {
            serial::write_str("[VIDEO/EDID] unavailable — MMIO not ready\n");
            return false;
        }
        if port > GMBUS_PIN_DPD || port == GMBUS_PIN_DISABLED {
            serial::write_str("[VIDEO/EDID] invalid GMBUS port\n");
            return false;
        }
        if !wait_gmbus(GMBUS_ACTIVE, false, 100_000) {
            serial::write_str("[VIDEO/EDID] controller busy\n");
            return false;
        }

        let mut ok = false;
        mmio_write32(GMBUS0, port | GMBUS_RATE_100KHZ);
        let command = GMBUS_CYCLE_INDEX
            | (128u32 << GMBUS_BYTE_COUNT_SHIFT)
            | (GMBUS_SLAVE_EDID << GMBUS_SLAVE_ADDR_SHIFT)
            | GMBUS_SLAVE_READ
            | GMBUS_SW_RDY;
        mmio_write32(GMBUS1, command);

        let mut pos = 0usize;
        while pos < 128 {
            if !wait_gmbus(GMBUS_HW_RDY, true, 200_000) {
                serial::write_str("[VIDEO/EDID] HW_RDY timeout\n");
                break;
            }
            let word = mmio_read32(GMBUS3);
            let mut b = 0usize;
            while b < 4 && pos < 128 {
                out[pos] = (word >> (b * 8)) as u8;
                pos += 1;
                b += 1;
            }
        }

        if pos == 128 {
            let status = mmio_read32(GMBUS2);
            if status & GMBUS_SATOER == 0 {
                let header_ok =
                    out[0] == 0x00 && out[1] == 0xFF && out[2] == 0xFF &&
                    out[3] == 0xFF && out[4] == 0xFF && out[5] == 0xFF &&
                    out[6] == 0xFF && out[7] == 0x00;
                let mut sum = 0u8;
                let mut i = 0usize;
                while i < 128 {
                    sum = sum.wrapping_add(out[i]);
                    i += 1;
                }
                ok = header_ok && sum == 0;
                serial::write_str(if ok {
                    "[VIDEO/EDID] block0 checksum PASS\n"
                } else {
                    "[VIDEO/EDID] block0 validation FAIL\n"
                });
            }
        }

        mmio_write32(GMBUS1, GMBUS_CYCLE_STOP | GMBUS_SW_RDY);
        let _ = wait_gmbus(GMBUS_ACTIVE, false, 100_000);
        mmio_write32(GMBUS0, GMBUS_PIN_DISABLED);
        ok
    }
}

/// Try the Sandy Bridge DDC pins until one returns a valid EDID block.
pub fn probe_edid(out: &mut [u8; 128]) -> u32 {
    let ports = [
        GMBUS_PIN_VGADDC, GMBUS_PIN_PANEL, GMBUS_PIN_DPC,
        GMBUS_PIN_DPB, GMBUS_PIN_DPD,
    ];
    let mut found = 0u32;
    let mut i = 0usize;
    while i < ports.len() {
        serial::write_str("[VIDEO/EDID] probing pin=");
        serial::write_usize(ports[i] as usize);
        serial::write_str("\n");
        if read_edid(ports[i], out) {
            found += 1;
            return found;
        }
        i += 1;
    }
    found
}
