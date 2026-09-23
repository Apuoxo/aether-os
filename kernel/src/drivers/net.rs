//! Aether network — Ethernet phase 2 + WiFi handoff
//! AH532: Realtek RTL8111/8168 (10ec:8168) on PCIe
//! Phase 2: multi-bus PCI, MMIO map, soft reset attempt, read MAC from ID registers.
//! TX/RX DMA rings = phase 3 (not claimed here).

use crate::serial;
use crate::mm::paging;

static mut READY: bool = false;
static mut ETH_FOUND: bool = false;
static mut WIFI_FOUND: bool = false;
static mut ETH_BUS: u8 = 0;
static mut ETH_DEV: u8 = 0;
static mut ETH_FUNC: u8 = 0;
static mut ETH_VID: u16 = 0;
static mut ETH_DID: u16 = 0;
static mut ETH_BAR0: u64 = 0;
static mut ETH_MMIO: usize = 0;
static mut ETH_MAC: [u8; 6] = [0; 6];
static mut ETH_MAC_OK: bool = false;
static mut ETH_RTL: bool = false;
static mut WIFI_VID: u16 = 0;
static mut WIFI_DID: u16 = 0;
static mut LINK_UP: bool = false;

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

unsafe fn r8(b: usize, o: usize) -> u8 {
    core::ptr::read_volatile((b + o) as *const u8)
}
unsafe fn r32(b: usize, o: usize) -> u32 {
    core::ptr::read_volatile((b + o) as *const u32)
}
unsafe fn w8(b: usize, o: usize, v: u8) {
    core::ptr::write_volatile((b + o) as *mut u8, v);
}

fn delay(n: u32) {
    let mut i = 0u32;
    while i < n {
        i += 1;
        core::hint::spin_loop();
    }
}

pub fn eth_found() -> bool {
    unsafe { ETH_FOUND }
}
pub fn wifi_found() -> bool {
    unsafe { WIFI_FOUND }
}
pub fn ready() -> bool {
    unsafe { READY }
}
pub fn link_up() -> bool {
    unsafe { LINK_UP }
}
pub fn eth_vid() -> u16 {
    unsafe { ETH_VID }
}
pub fn eth_did() -> u16 {
    unsafe { ETH_DID }
}
pub fn eth_bar0() -> u32 {
    unsafe { ETH_BAR0 as u32 }
}
pub fn eth_mac_ok() -> bool {
    unsafe { ETH_MAC_OK }
}
pub fn eth_is_rtl() -> bool {
    unsafe { ETH_RTL }
}
pub fn eth_mac(out: &mut [u8; 6]) {
    unsafe {
        let mut i = 0usize;
        while i < 6 {
            out[i] = ETH_MAC[i];
            i += 1;
        }
    }
}

/// RTL8168/8111: soft-reset + read station address (IDR0..5)
fn rtl_init_and_mac(mmio: usize) -> bool {
    unsafe {
        // ChipCmd 0x37: bit 4 = Reset
        w8(mmio, 0x37, 0x10);
        let mut t = 0u32;
        while t < 10000 {
            if r8(mmio, 0x37) & 0x10 == 0 {
                break;
            }
            delay(100);
            t += 1;
        }
        // Read MAC from ID registers 0x00..0x05
        let mut i = 0usize;
        let mut nonzero = false;
        while i < 6 {
            ETH_MAC[i] = r8(mmio, i);
            if ETH_MAC[i] != 0 && ETH_MAC[i] != 0xFF {
                nonzero = true;
            }
            i += 1;
        }
        ETH_MAC_OK = nonzero;
        serial::write_str("[NET] RTL ChipCmd after reset=");
        serial::write_hex(r8(mmio, 0x37) as usize);
        serial::write_str(" MAC=");
        i = 0;
        while i < 6 {
            serial::write_hex(ETH_MAC[i] as usize);
            if i + 1 < 6 {
                serial::write_str(":");
            }
            i += 1;
        }
        serial::write_str(if ETH_MAC_OK { " OK\n" } else { " (empty)\n" });
        ETH_MAC_OK
    }
}

pub fn init() {
    serial::write_str("[NET] force probe PCI network (buses 0..31)...\n");
    unsafe {
        ETH_FOUND = false;
        WIFI_FOUND = false;
        READY = false;
        LINK_UP = false;
        ETH_MAC_OK = false;
        ETH_RTL = false;
        ETH_MMIO = 0;
        let mut best_rtl = false;
        let mut saved = false;

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
                    if class != 0x02 {
                        continue;
                    }
                    let cmd = (pci_r32(bus, dev, func, 0x04) & 0xFFFF) as u16;
                    pci_w16(bus, dev, func, 0x04, cmd | 0x0006);
                    let mut bar0 = pci_r32(bus, dev, func, 0x10) as u64;
                    if bar0 & 0x4 != 0 {
                        let bar1 = pci_r32(bus, dev, func, 0x14) as u64;
                        bar0 = (bar0 & !0xF) | (bar1 << 32);
                    } else {
                        bar0 &= !0xF;
                    }

                    if sub == 0x00 {
                        let is_rtl = vid == 0x10EC
                            && (did == 0x8168 || did == 0x8169 || did == 0x8136 || did == 0x8129);
                        // Prefer Realtek (AH532); else first ethernet
                        if !saved || (is_rtl && !best_rtl) {
                            ETH_FOUND = true;
                            ETH_BUS = bus;
                            ETH_DEV = dev;
                            ETH_FUNC = func;
                            ETH_VID = vid;
                            ETH_DID = did;
                            ETH_BAR0 = bar0;
                            ETH_RTL = is_rtl;
                            saved = true;
                            if is_rtl {
                                best_rtl = true;
                            }
                        }
                        serial::write_str("[NET] ETH ");
                        serial::write_usize(bus as usize);
                        serial::write_str(":");
                        serial::write_usize(dev as usize);
                        serial::write_str(".");
                        serial::write_usize(func as usize);
                        serial::write_str(" VID=");
                        serial::write_hex(vid as usize);
                        serial::write_str(" DID=");
                        serial::write_hex(did as usize);
                        serial::write_str(" BAR0=");
                        serial::write_hex(bar0 as usize);
                        if is_rtl {
                            serial::write_str(" RTL81xx\n");
                        } else {
                            serial::write_str("\n");
                        }
                    } else if sub == 0x80 {
                        WIFI_FOUND = true;
                        WIFI_VID = vid;
                        WIFI_DID = did;
                        serial::write_str("[NET] WIFI class ");
                        serial::write_usize(bus as usize);
                        serial::write_str(":");
                        serial::write_usize(dev as usize);
                        serial::write_str(".");
                        serial::write_usize(func as usize);
                        serial::write_str(" VID=");
                        serial::write_hex(vid as usize);
                        serial::write_str(" DID=");
                        serial::write_hex(did as usize);
                        serial::write_str("\n");
                    }
                }
            }
        }

        if ETH_FOUND && ETH_BAR0 != 0 {
            if map_mmio(ETH_BAR0, 0x1000) {
                ETH_MMIO = ETH_BAR0 as usize;
                serial::write_str("[NET] ETH MMIO mapped\n");
                if ETH_RTL {
                    rtl_init_and_mac(ETH_MMIO);
                } else {
                    // Generic: try read 6 bytes at BAR0 (works on some NICs)
                    let mut i = 0usize;
                    let mut nz = false;
                    while i < 6 {
                        ETH_MAC[i] = r8(ETH_MMIO, i);
                        if ETH_MAC[i] != 0 && ETH_MAC[i] != 0xFF {
                            nz = true;
                        }
                        i += 1;
                    }
                    ETH_MAC_OK = nz;
                    serial::write_str("[NET] non-RTL MAC probe ");
                    serial::write_str(if ETH_MAC_OK { "data\n" } else { "empty\n" });
                }
            } else {
                serial::write_str("[NET] ETH MMIO map skip/fail\n");
            }
        }

        READY = ETH_FOUND || WIFI_FOUND;
        if !ETH_FOUND {
            serial::write_str("[NET] no Ethernet\n");
        }
        serial::write_str("[NET] phase2 DONE (MAC try; no TX/RX DMA)\n");
    }
}
