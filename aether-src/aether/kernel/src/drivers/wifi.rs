//! Aether WiFi driver — phase 1 for Fujitsu LIFEBOOK AH532
//! Hardware: Intel Centrino Wireless-N 2230
//!   PCI ID: 8086:0887  Subsystem: 8086:4062  Class: 02:80
//!   Linux: iwlwifi + firmware iwlwifi-2030-*.ucode
//!   BAR0: 8 KiB MMIO
//!
//! Phase 1 (this file): PCI find (all buses), bus-master, MMIO map,
//! identify 2230, report status. NO full 802.11 without firmware.
//! Full assoc/TX is out of scope until firmware loader exists.

use crate::serial;
use crate::mm::paging;

const INTEL_VID: u16 = 0x8086;
const CENTRINO_2230_DID: u16 = 0x0887;
const SUBSYS_BGN: u16 = 0x4062;

static mut FOUND: bool = false;
static mut READY: bool = false; // phase1 ready = found + mapped
static mut NEEDS_FW: bool = true;
static mut BUS: u8 = 0;
static mut DEV: u8 = 0;
static mut FUNC: u8 = 0;
static mut BAR0: u64 = 0;
static mut MMIO: usize = 0;
static mut REV: u8 = 0;
static mut SUBSYS: u16 = 0;

unsafe fn pci_r32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") v, options(nostack, preserves_flags));
    v
}
unsafe fn pci_w16(bus: u8, dev: u8, func: u8, off: u8, val: u16) {
    let a = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    let mut old: u32;
    core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") old, options(nostack, preserves_flags));
    let shift = (off as u32 & 2) * 8;
    old = (old & !(0xFFFF << shift)) | ((val as u32) << shift);
    core::arch::asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") old, options(nostack, preserves_flags));
}

fn map_mmio(phys: u64, size: usize) -> bool {
    if phys == 0 {
        return false;
    }
    let cr3 = unsafe { paging::read_cr3() };
    let start = (phys as usize) & !0xFFF;
    let end = (phys as usize + size + 0xFFF) & !0xFFF;
    let mut va = start;
    while va < end {
        if va >= 0x4000_0000 {
            let _ = unsafe {
                paging::map_page(cr3, va, va, paging::PAGE_PRESENT | paging::PAGE_WRITE)
            };
        }
        va += 0x1000;
    }
    unsafe { paging::load_cr3(cr3) };
    true
}

pub fn found() -> bool {
    unsafe { FOUND }
}
pub fn ready() -> bool {
    unsafe { READY }
}
pub fn needs_firmware() -> bool {
    unsafe { NEEDS_FW }
}
pub fn bar0() -> u64 {
    unsafe { BAR0 }
}
pub fn bus_dev_func() -> (u8, u8, u8) {
    unsafe { (BUS, DEV, FUNC) }
}

/// Force probe Intel Centrino Wireless-N 2230 (AH532) and any 02:80 Intel WLAN
pub fn init() {
    serial::write_str("[WIFI] AH532 target: Centrino Wireless-N 2230 (8086:0887)\n");
    unsafe {
        FOUND = false;
        READY = false;
        NEEDS_FW = true;
        // BUGFIX: WiFi sits behind PCIe root port → secondary bus (often 8 or 9)
        for bus in 0u8..=31 {
            for dev in 0u8..32 {
                for func in 0u8..8 {
                    let id = pci_r32(bus, dev, func, 0);
                    if id == 0xFFFF_FFFF || id == 0 {
                        continue;
                    }
                    let vid = (id & 0xFFFF) as u16;
                    let did = ((id >> 16) & 0xFFFF) as u16;
                    let cr = pci_r32(bus, dev, func, 0x08);
                    let class = ((cr >> 24) & 0xFF) as u8;
                    let sub = ((cr >> 16) & 0xFF) as u8;
                    let rev = (cr & 0xFF) as u8;
                    if class != 0x02 || sub != 0x80 {
                        continue;
                    }
                    if vid != INTEL_VID {
                        continue;
                    }
                    // Prefer exact 2230, else any Intel WLAN
                    let is_2230 = did == CENTRINO_2230_DID;
                    let mut bar0 = pci_r32(bus, dev, func, 0x10) as u64;
                    if bar0 & 0x4 != 0 {
                        let bar1 = pci_r32(bus, dev, func, 0x14) as u64;
                        bar0 = (bar0 & !0xF) | (bar1 << 32);
                    } else {
                        bar0 &= !0xF;
                    }
                    let subsys = (pci_r32(bus, dev, func, 0x2C) >> 16) as u16;

                    // Enable Mem + Bus Master
                    let cmd = (pci_r32(bus, dev, func, 0x04) & 0xFFFF) as u16;
                    pci_w16(bus, dev, func, 0x04, cmd | 0x0006);

                    FOUND = true;
                    BUS = bus;
                    DEV = dev;
                    FUNC = func;
                    BAR0 = bar0;
                    REV = rev;
                    SUBSYS = subsys;

                    serial::write_str("[WIFI] FOUND ");
                    serial::write_usize(bus as usize);
                    serial::write_str(":");
                    serial::write_usize(dev as usize);
                    serial::write_str(".");
                    serial::write_usize(func as usize);
                    serial::write_str(" DID=");
                    serial::write_hex(did as usize);
                    serial::write_str(" SUB=");
                    serial::write_hex(subsys as usize);
                    serial::write_str(" BAR0=");
                    serial::write_hex(bar0 as usize);
                    if is_2230 {
                        serial::write_str(" Centrino-N-2230");
                        if subsys == SUBSYS_BGN {
                            serial::write_str(" BGN");
                        }
                    }
                    serial::write_str("\n");

                    if bar0 != 0 && map_mmio(bar0, 0x2000) {
                        MMIO = bar0 as usize;
                        READY = true;
                        serial::write_str("[WIFI] MMIO mapped 8K phase1 OK\n");
                        // Touch first dword (alive check) — may be 0 without FW
                        let v = core::ptr::read_volatile(MMIO as *const u32);
                        serial::write_str("[WIFI] MMIO[0]=");
                        serial::write_hex(v as usize);
                        serial::write_str("\n");
                    } else {
                        serial::write_str("[WIFI] MMIO map FAIL or BAR0=0\n");
                    }
                    serial::write_str("[WIFI] assoc/TX requires iwlwifi firmware — NEEDS_FW\n");
                    return;
                }
            }
        }
        serial::write_str("[WIFI] no Intel WLAN on buses 0..31\n");
    }
}
