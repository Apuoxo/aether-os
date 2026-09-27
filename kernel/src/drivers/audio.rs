//! Aether native audio hardware foundation.
//!
//! Final media playback is HDA/PCM DMA based. The PC speaker remains only as a
//! diagnostic fallback; it is not the media-player output path.
//!
//! This stage deliberately performs discovery only. It does not reset or touch
//! HDA MMIO until BAR mapping and the controller/codec bring-up path are ready.

use crate::serial;

static mut SPEAKER_OK: bool = true;
static mut HDA_FOUND: bool = false;
static mut HDA_BDF: u16 = 0;
static mut HDA_BAR0: u64 = 0;
static mut HDA_BAR0_SIZE_HINT: u32 = 0;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nostack, preserves_flags));
    v
}

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

pub fn speaker_ok() -> bool {
    unsafe { SPEAKER_OK }
}

pub fn hda_found() -> bool {
    unsafe { HDA_FOUND }
}

pub fn hda_bar0() -> u64 {
    unsafe { HDA_BAR0 }
}

/// Default short diagnostic beep. This is intentionally not the media path.
pub fn beep() {
    beep_hz(880, 12);
}

pub fn beep_hz(freq_hz: u32, duration_units: u32) {
    if freq_hz < 20 || freq_hz > 20000 {
        return;
    }
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
        let t2 = inb(0x61);
        outb(0x61, t2 & !3);
    }
}

fn print_hex(v: u64) {
    serial::write_hex(v as usize);
}

fn probe_hda() {
    unsafe {
        HDA_FOUND = false;
        HDA_BDF = 0;
        HDA_BAR0 = 0;
        HDA_BAR0_SIZE_HINT = 0;

        for dev in 0u8..32 {
            for func in 0u8..8 {
                let id = pci_r32(0, dev, func, 0);
                if id == 0xFFFF_FFFF || id == 0 {
                    continue;
                }

                let cr = pci_r32(0, dev, func, 0x08);
                let class = ((cr >> 24) & 0xFF) as u8;
                let sub = ((cr >> 16) & 0xFF) as u8;

                if class != 0x04 || sub != 0x03 {
                    continue;
                }

                let vendor = (id & 0xFFFF) as u16;
                let device = (id >> 16) as u16;
                let bar0 = pci_r32(0, dev, func, 0x10);
                let bar1 = pci_r32(0, dev, func, 0x14);
                let bar_is_io = (bar0 & 1) != 0;

                // HDA uses a memory BAR. Do not access it until the MMIO
                // mapping layer explicitly owns the physical address.
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
                HDA_BAR0_SIZE_HINT = 0x1000;

                serial::write_str("[AUDIO] HDA controller discovered BDF=00:");
                serial::write_usize(dev as usize);
                serial::write_str(".");
                serial::write_usize(func as usize);
                serial::write_str(" VID=");
                serial::write_hex(vendor as usize);
                serial::write_str(" DID=");
                serial::write_hex(device as usize);
                serial::write_str(" BAR0=");
                print_hex(base);
                serial::write_str(bar_is_io.then_some(" IO").unwrap_or(" MMIO"));
                serial::write_str("\n");

                if base == 0 || bar_is_io {
                    serial::write_str("[AUDIO] HDA BAR0 is not usable yet; MMIO bring-up deferred\n");
                } else {
                    serial::write_str("[AUDIO] HDA BAR0 candidate recorded; no MMIO access performed\n");
                }
                return;
            }
        }

        serial::write_str("[AUDIO] HDA controller not found on bus0\n");
    }
}

pub fn init() {
    unsafe {
        SPEAKER_OK = true;
    }
    serial::write_str("[AUDIO] PC speaker diagnostic path OK\n");
    probe_hda();
}
