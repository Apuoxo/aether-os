//! Minimal audio stack for Aether:
//! - PC speaker (always available on classic x86)
//! - HDA probe (PCI class 04.03) — detect only for now

use crate::serial;

static mut SPEAKER_OK: bool = true;
static mut HDA_FOUND: bool = false;

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

fn delay(n: u32) {
    let mut i = 0u32;
    while i < n {
        i += 1;
        core::hint::spin_loop();
    }
}

pub fn speaker_ok() -> bool {
    unsafe { SPEAKER_OK }
}
pub fn hda_found() -> bool {
    unsafe { HDA_FOUND }
}

/// Default short beep
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
        delay(duration_units.saturating_mul(40_000));
        let t2 = inb(0x61);
        outb(0x61, t2 & !3);
    }
}

pub fn play_startup_chime() {
    beep_hz(523, 6);
    beep_hz(659, 6);
    beep_hz(784, 10);
}

fn probe_hda() {
    unsafe {
        HDA_FOUND = false;
        for bus in 0u8..=0 {
            for dev in 0u8..32 {
                for func in 0u8..8 {
                    let id = pci_r32(bus, dev, func, 0);
                    if id == 0xFFFF_FFFF || id == 0 {
                        continue;
                    }
                    let cr = pci_r32(bus, dev, func, 0x08);
                    let class = ((cr >> 24) & 0xFF) as u8;
                    let sub = ((cr >> 16) & 0xFF) as u8;
                    // Multimedia audio controller
                    if class == 0x04 && (sub == 0x03 || sub == 0x01) {
                        HDA_FOUND = true;
                        serial::write_str("[AUDIO] HDA/AC97 PCI found ");
                        serial::write_usize(bus as usize);
                        serial::write_str(":");
                        serial::write_usize(dev as usize);
                        serial::write_str(".");
                        serial::write_usize(func as usize);
                        serial::write_str(" (probe only)\n");
                        return;
                    }
                }
            }
        }
        serial::write_str("[AUDIO] HDA not found on bus0\n");
    }
}

pub fn init() {
    unsafe {
        SPEAKER_OK = true;
    }
    serial::write_str("[AUDIO] PC speaker OK\n");
    probe_hda();
}
