//! Aether Intel integrated GPU identity layer — Stage 2.
//! Read-only PCI discovery. No MMIO programming, GGTT/GSM, modeset or EDID.

use crate::serial;

const PCI_ADDR: u16 = 0x0CF8;
const PCI_DATA: u16 = 0x0CFC;
const INTEL: u16 = 0x8086;
const DISPLAY_CLASS: u8 = 0x03;

#[derive(Copy, Clone)]
pub struct IntelGpu {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vid: u16,
    pub did: u16,
    pub rev: u8,
    pub cmd: u16,
    pub bars: [u64; 6],
    pub irq: u8,
    pub gen: u8,
}

static mut GPU: IntelGpu = IntelGpu {
    bus: 0, dev: 0, func: 0, vid: 0, did: 0, rev: 0, cmd: 0,
    bars: [0; 6], irq: 0, gen: 0,
};
static mut READY: bool = false;

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

fn gen_from_did(did: u16) -> u8 {
    match did {
        0x0116 | 0x0112 | 0x011A | 0x011E => 6,
        _ => 0,
    }
}

fn name_from_did(did: u16) -> &'static str {
    match did {
        0x0116 => "SNB HD Graphics 3000",
        _ => "Intel integrated graphics",
    }
}

fn log_bar(i: usize, value: u64) {
    serial::write_str("[IGPU] BAR");
    serial::write_usize(i);
    serial::write_str("=");
    serial::write_hex((value >> 32) as usize);
    serial::write_str("_");
    serial::write_hex(value as usize);
    serial::write_str("\n");
}

pub fn init() -> bool {
    unsafe {
        READY = false;
        for dev in 0u8..32 {
            for func in 0u8..8 {
                let id = pci_read32(0, dev, func, 0);
                if id == 0 || id == 0xFFFF_FFFF {
                    continue;
                }
                let vid = id as u16;
                let did = (id >> 16) as u16;
                let class = (pci_read32(0, dev, func, 0x08) >> 24) as u8;
                if class != DISPLAY_CLASS {
                    continue;
                }

                let cmd = pci_read32(0, dev, func, 0x04) as u16;
                let rev = pci_read32(0, dev, func, 0x08) as u8;
                let irq = pci_read32(0, dev, func, 0x3C) as u8;
                let mut bars = [0u64; 6];
                let mut i = 0usize;
                while i < 6 {
                    let raw = pci_read32(0, dev, func, 0x10 + (i as u8) * 4);
                    if raw & 1 != 0 {
                        bars[i] = (raw & !0x3) as u64;
                        i += 1;
                        continue;
                    }
                    let kind = (raw >> 1) & 3;
                    let low = (raw & !0xF) as u64;
                    if kind == 2 && i < 5 {
                        let hi = pci_read32(0, dev, func, 0x14 + (i as u8) * 4);
                        bars[i] = low | ((hi as u64) << 32);
                        i += 2;
                    } else {
                        bars[i] = low;
                        i += 1;
                    }
                }

                if vid != INTEL {
                    continue;
                }

                GPU = IntelGpu {
                    bus: 0, dev, func, vid, did, rev, cmd, bars, irq,
                    gen: gen_from_did(did),
                };
                READY = true;

                serial::write_str("[IGPU] PCI ");
                serial::write_usize(dev as usize);
                serial::write_str(":");
                serial::write_usize(func as usize);
                serial::write_str(" ");
                serial::write_hex(vid as usize);
                serial::write_str(":");
                serial::write_hex(did as usize);
                serial::write_str(" REV=");
                serial::write_hex(rev as usize);
                serial::write_str(" CMD=");
                serial::write_hex(cmd as usize);
                serial::write_str(" IRQ=");
                serial::write_usize(irq as usize);
                serial::write_str(" GEN=");
                serial::write_usize(GPU.gen as usize);
                serial::write_str(" NAME=");
                serial::write_str(name_from_did(did));
                serial::write_str("\n");

                i = 0;
                while i < 6 {
                    log_bar(i, bars[i]);
                    i += 1;
                }
                return true;
            }
        }
    }
    serial::write_str("[IGPU] no Intel display controller\n");
    false
}

pub fn ready() -> bool { unsafe { READY } }
pub fn info() -> IntelGpu { unsafe { GPU } }
pub fn mmio_bar() -> u64 { unsafe { GPU.bars[0] } }
pub fn aperture_bar() -> u64 { unsafe { GPU.bars[2] } }
pub fn gen() -> u8 { unsafe { GPU.gen } }
pub fn did() -> u16 { unsafe { GPU.did } }
pub fn name() -> &'static str { unsafe { name_from_did(GPU.did) } }
