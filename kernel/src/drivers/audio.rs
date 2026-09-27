//! Aether native audio hardware foundation.
//!
//! PC speaker is diagnostic only. The media path is HDA/PCM DMA.
//! This stage discovers the HDA controller, records BAR0, maps one MMIO page,
//! and reads capability/status registers. It deliberately does not reset HDA,
//! start DMA, or send codec verbs yet.

use crate::serial;

static mut SPEAKER_OK: bool = true;
static mut HDA_FOUND: bool = false;
static mut HDA_BDF: u16 = 0;
static mut HDA_BAR0: u64 = 0;
static mut HDA_MMIO_READY: bool = false;
static mut HDA_CORB_PHYS: usize = 0;
static mut HDA_RIRB_PHYS: usize = 0;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nostack, preserves_flags));
    v
}
unsafe fn pci_addr(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x8000_0000u32 | ((bus as u32) << 16) | ((dev as u32) << 11)
        | ((func as u32) << 8) | ((off as u32) & 0xFC)
}
unsafe fn pci_r32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = pci_addr(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") v, options(nostack, preserves_flags));
    v
}
unsafe fn pci_w32(bus: u8, dev: u8, func: u8, off: u8, value: u32) {
    let a = pci_addr(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    core::arch::asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") value, options(nostack, preserves_flags));
}
unsafe fn hda_r8(base: usize, off: usize) -> u8 {
    core::ptr::read_volatile((base + off) as *const u8)
}
unsafe fn hda_r16(base: usize, off: usize) -> u16 {
    core::ptr::read_volatile((base + off) as *const u16)
}
unsafe fn hda_r32(base: usize, off: usize) -> u32 {
    core::ptr::read_volatile((base + off) as *const u32)
}
unsafe fn hda_w8(base: usize, off: usize, v: u8) {
    core::ptr::write_volatile((base + off) as *mut u8, v);
}
unsafe fn hda_w32(base: usize, off: usize, v: u32) {
    core::ptr::write_volatile((base + off) as *mut u32, v);
}
fn wait_short() {
    let mut i = 0u32;
    while i < 100_000 { core::hint::spin_loop(); i += 1; }
}

pub fn speaker_ok() -> bool { unsafe { SPEAKER_OK } }
pub fn hda_found() -> bool { unsafe { HDA_FOUND } }
pub fn hda_bar0() -> u64 { unsafe { HDA_BAR0 } }
pub fn hda_mmio_ready() -> bool { unsafe { HDA_MMIO_READY } }

pub fn beep() { beep_hz(880, 12); }

pub fn beep_hz(freq_hz: u32, duration_units: u32) {
    if freq_hz < 20 || freq_hz > 20000 { return; }
    unsafe {
        let div = 1193182u32 / freq_hz;
        outb(0x43, 0xB6);
        outb(0x42, (div & 0xFF) as u8);
        outb(0x42, ((div >> 8) & 0xFF) as u8);
        let t = inb(0x61);
        outb(0x61, t | 3);
        let mut i = 0u32;
        while i < duration_units.saturating_mul(40_000) {
            i += 1;
            core::hint::spin_loop();
        }
        outb(0x61, inb(0x61) & !3);
    }
}

fn probe_hda() {
    unsafe {
        HDA_FOUND = false;
        HDA_BDF = 0;
        HDA_BAR0 = 0;
        HDA_MMIO_READY = false;

        for dev in 0u8..32 {
            for func in 0u8..8 {
                let id = pci_r32(0, dev, func, 0);
                if id == 0xFFFF_FFFF || id == 0 { continue; }
                let cr = pci_r32(0, dev, func, 0x08);
                let class = ((cr >> 24) & 0xFF) as u8;
                let sub = ((cr >> 16) & 0xFF) as u8;
                if class != 0x04 || sub != 0x03 { continue; }

                let vendor = (id & 0xFFFF) as u16;
                let device = (id >> 16) as u16;
                let bar0 = pci_r32(0, dev, func, 0x10);
                let bar1 = pci_r32(0, dev, func, 0x14);
                let bar_is_io = (bar0 & 1) != 0;
                let base = if bar_is_io {
                    0
                } else if (bar0 & 0x6) == 0x4 {
                    ((bar1 as u64) << 32) | ((bar0 as u64) & 0xFFFF_FFF0)
                } else {
                    (bar0 as u64) & 0xFFFF_FFF0
                };

                HDA_FOUND = true;
                HDA_BDF = ((dev as u16) << 3) | func as u16;
                HDA_BAR0 = base;

                serial::write_str("[AUDIO] HDA BDF=00:");
                serial::write_usize(dev as usize);
                serial::write_str(".");
                serial::write_usize(func as usize);
                serial::write_str(" VID=");
                serial::write_hex(vendor as usize);
                serial::write_str(" DID=");
                serial::write_hex(device as usize);
                serial::write_str(" BAR0=");
                serial::write_hex(base as usize);
                serial::write_str("\n");

                if base == 0 || bar_is_io {
                    serial::write_str("[AUDIO] HDA BAR0 unusable; MMIO deferred\n");
                    return;
                }

                let cmd = pci_r32(0, dev, func, 0x04);
                let new_cmd = cmd | 0x0000_0006;
                if new_cmd != cmd { pci_w32(0, dev, func, 0x04, new_cmd); }

                let phys = base as usize & !0xFFF;
                let cr3 = crate::mm::paging::read_cr3();
                let mapped = crate::mm::paging::map_page(
                    cr3, phys, phys,
                    crate::mm::paging::PAGE_PRESENT
                        | crate::mm::paging::PAGE_WRITE
                        | crate::mm::paging::PAGE_PCD
                        | crate::mm::paging::PAGE_PWT,
                );
                if !mapped {
                    serial::write_str("[AUDIO] HDA MMIO map failed; untouched\n");
                    return;
                }
                crate::mm::paging::load_cr3(cr3);

                let mmio = base as usize;
                let gcap = hda_r16(mmio, 0x00);
                let gctl = hda_r32(mmio, 0x08);
                let statests = hda_r16(mmio, 0x0E);
                let intsts = hda_r32(mmio, 0x24);
                let walclk = hda_r32(mmio, 0x30);
                let corb_size = hda_r8(mmio, 0x4E);
                let rirb_size = hda_r8(mmio, 0x5E);
                HDA_MMIO_READY = true;

                serial::write_str("[AUDIO] HDA MMIO READY GCAP=");
                serial::write_hex(gcap as usize);
                serial::write_str(" GCTL=");
                serial::write_hex(gctl as usize);
                serial::write_str(" STATESTS=");
                serial::write_hex(statests as usize);
                serial::write_str(" INTSTS=");
                serial::write_hex(intsts as usize);
                serial::write_str(" WALCLK=");
                serial::write_hex(walclk as usize);
                serial::write_str(" CORBSIZE=");
                serial::write_hex(corb_size as usize);
                serial::write_str(" RIRBSIZE=");
                serial::write_hex(rirb_size as usize);
                serial::write_str("\n");
                hda_w32(mmio, 0x08, gctl & !1);
                let mut reset_ok = false;
                let mut n = 0u32;
                while n < 1000 {
                    if hda_r32(mmio, 0x08) & 1 == 0 { reset_ok = true; break; }
                    n += 1;
                    core::hint::spin_loop();
                }
                if !reset_ok {
                    serial::write_str("[AUDIO] HDA RESET_ASSERT_TIMEOUT\n");
                    return;
                }

                hda_w32(mmio, 0x08, hda_r32(mmio, 0x08) | 1);
                let mut running = false;
                n = 0;
                while n < 5000 {
                    if hda_r32(mmio, 0x08) & 1 != 0 { running = true; break; }
                    n += 1;
                    core::hint::spin_loop();
                }
                if !running {
                    serial::write_str("[AUDIO] HDA RESET_RELEASE_TIMEOUT\n");
                    return;
                }

                // Codec enumeration may take up to 25 HDA frames after CRST=1.
                let mut d = 0;
                while d < 6 { wait_short(); d += 1; }

                let states = hda_r16(mmio, 0x0E);
                serial::write_str("[AUDIO] HDA RESET=OK STATESTS=");
                serial::write_hex(states as usize);
                serial::write_str("\n");
                if states == 0 {
                    serial::write_str("[AUDIO] HDA CODEC_NONE\n");
                    return;
                }
                serial::write_str("[AUDIO] HDA CODEC_ADDRS=");
                serial::write_hex(states as usize);
                serial::write_str("\n");
                // Allocate physically contiguous, naturally aligned DMA rings.
                // CORB: 256 x 4-byte commands = 1 KiB.
                // RIRB: 256 x 8-byte responses = 2 KiB.
                let corb_phys = match crate::mm::alloc_pages(1) {
                    Some(p) => p,
                    None => {
                        serial::write_str("[AUDIO] CORB_ALLOC_FAIL\n");
                        return;
                    }
                };
                let rirb_phys = match crate::mm::alloc_pages(2) {
                    Some(p) => p,
                    None => {
                        crate::mm::free_page(corb_phys);
                        serial::write_str("[AUDIO] RIRB_ALLOC_FAIL\n");
                        return;
                    }
                };
                crate::mm::zero_pages(corb_phys, 1);
                crate::mm::zero_pages(rirb_phys, 2);

                let cr3_dma = crate::mm::paging::read_cr3();
                let mut p = corb_phys;
                while p < corb_phys + 0x1000 {
                    if !crate::mm::paging::map_page(
                        cr3_dma, p, p,
                        crate::mm::paging::PAGE_PRESENT
                            | crate::mm::paging::PAGE_WRITE
                            | crate::mm::paging::PAGE_PCD
                            | crate::mm::paging::PAGE_PWT,
                    ) { serial::write_str("[AUDIO] CORB_MAP_FAIL\n"); return; }
                    p += 0x1000;
                }
                p = rirb_phys;
                while p < rirb_phys + 0x2000 {
                    if !crate::mm::paging::map_page(
                        cr3_dma, p, p,
                        crate::mm::paging::PAGE_PRESENT
                            | crate::mm::paging::PAGE_WRITE
                            | crate::mm::paging::PAGE_PCD
                            | crate::mm::paging::PAGE_PWT,
                    ) { serial::write_str("[AUDIO] RIRB_MAP_FAIL\n"); return; }
                    p += 0x1000;
                }
                crate::mm::paging::load_cr3(cr3_dma);

                HDA_CORB_PHYS = corb_phys;
                HDA_RIRB_PHYS = rirb_phys;

                // Stop engines before programming base addresses.
                hda_w8(mmio, 0x4C, 0);
                hda_w8(mmio, 0x5C, 0);
                hda_w16(mmio, 0x48, 0x8000);
                hda_w16(mmio, 0x48, 0);
                hda_w16(mmio, 0x58, 0x8000);
                hda_w16(mmio, 0x58, 0);

                // Select 256-entry rings when the controller advertises that size.
                if hda_r8(mmio, 0x4E) & 0x40 != 0 {
                    hda_w8(mmio, 0x4E, (hda_r8(mmio, 0x4E) & 0xFC) | 0x02);
                }
                if hda_r8(mmio, 0x5E) & 0x40 != 0 {
                    hda_w8(mmio, 0x5E, (hda_r8(mmio, 0x5E) & 0xFC) | 0x02);
                }

                hda_w32(mmio, 0x40, corb_phys as u32);
                hda_w32(mmio, 0x44, (corb_phys >> 32) as u32);
                hda_w32(mmio, 0x50, rirb_phys as u32);
                hda_w32(mmio, 0x54, (rirb_phys >> 32) as u32);

                // One response is enough for synchronous codec discovery.
                hda_w16(mmio, 0x5A, 1);
                hda_w8(mmio, 0x4C, 0x02);
                hda_w8(mmio, 0x5C, 0x02);

                serial::write_str("[AUDIO] HDA CORB/RIRB READY CORB=");
                serial::write_hex(corb_phys);
                serial::write_str(" RIRB=");
                serial::write_hex(rirb_phys);
                serial::write_str("\n");

                // Get Parameter(Vendor ID) from root node 0 of codec address 0.
                let codec = states.trailing_zeros() as u8;
                let cmd = ((codec as u32) << 28) | (0xF00u32 << 8);
                let wp = hda_r16(mmio, 0x48) & 0x00FF;
                let next = (wp.wrapping_add(1)) & 0x00FF;
                core::ptr::write_volatile((corb_phys + (next as usize) * 4) as *mut u32, cmd);
                hda_w16(mmio, 0x48, next);

                let mut got = false;
                let mut tries = 0u32;
                while tries < 200_000 {
                    if (hda_r16(mmio, 0x58) & 0x00FF) == next { got = true; break; }
                    tries += 1;
                    core::hint::spin_loop();
                }
                if !got {
                    serial::write_str("[AUDIO] HDA VERB_TIMEOUT\n");
                    return;
                }

                let resp = core::ptr::read_volatile((rirb_phys + (next as usize) * 8) as *const u32);
                serial::write_str("[AUDIO] HDA CODEC=");
                serial::write_usize(codec as usize);
                serial::write_str(" VID_DID=");
                serial::write_hex(resp as usize);
                serial::write_str("\n");
                serial::write_str("[AUDIO] HDA CORB/RIRB transport PASS; next: AFG/node enumeration\n");
                return;
            }
        }
        serial::write_str("[AUDIO] HDA controller not found on bus0\n");
    }
}

pub fn init() {
    unsafe { SPEAKER_OK = true; }
    serial::write_str("[AUDIO] native HDA foundation init\n");
    probe_hda();
}
