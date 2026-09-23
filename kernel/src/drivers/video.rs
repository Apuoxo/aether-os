//! Aether Intel graphics driver.
//!
//! Stages 0/0.1: read-only PCI identification and BAR characterization.
//! No PCI writes, no BAR sizing writes, no MMIO mapping and no GPU register
//! access are performed until a later guarded stage.

use crate::graphics;
use crate::serial;

const PCI_CONFIG_ADDR: u16 = 0x0CF8;
const PCI_CONFIG_DATA: u16 = 0x0CFC;
const INTEL_VENDOR: u16 = 0x8086;
const DISPLAY_CLASS: u8 = 0x03;
const HD3000_DID: u16 = 0x0116;

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

fn print_bar(index: u8, value: u32) {
    serial::write_str("[VIDEO/PCI] BAR");
    serial::write_usize(index as usize);
    serial::write_str("=");
    serial::write_hex(value as usize);

    if value == 0 {
        serial::write_str(" UNUSED\n");
        return;
    }

    if value & 1 != 0 {
        // I/O BAR.
        let base = value & !0x3;
        serial::write_str(" IO base=");
        serial::write_hex(base as usize);
    } else {
        // Memory BAR. Bit 2 selects 64-bit BARs.
        let kind = (value >> 1) & 0x3;
        let prefetch = (value >> 3) & 1;
        let base = value & !0xF;
        serial::write_str(" MMIO base=");
        serial::write_hex(base as usize);
        serial::write_str(" type=");
        serial::write_usize(kind as usize);
        serial::write_str(if kind == 2 { " 64bit" } else { " 32bit" });
        serial::write_str(if prefetch != 0 { " prefetch" } else { " nonprefetch" });
    }
    serial::write_str("\n");
}

pub fn probe() -> bool {
    serial::write_str("\n======== INTEL VIDEO PROBE ========\n");
    let mut found_display = false;
    let mut found_target = false;

    for dev in 0u8..32 {
        for func in 0u8..8 {
            unsafe {
                let id = pci_read32(0, dev, func, 0);
                if id == 0 || id == 0xFFFF_FFFF {
                    continue;
                }

                let vendor = (id & 0xFFFF) as u16;
                let device = (id >> 16) as u16;
                let class_reg = pci_read32(0, dev, func, 0x08);
                let class = (class_reg >> 24) as u8;
                if class != DISPLAY_CLASS {
                    continue;
                }

                let subclass = (class_reg >> 16) as u8;
                let prog_if = (class_reg >> 8) as u8;
                let revision = class_reg as u8;
                let command_status = pci_read32(0, dev, func, 0x04);
                let command = command_status as u16;
                found_display = true;

                serial::write_str("[VIDEO/PCI] display 00:");
                serial::write_usize(dev as usize);
                serial::write_str(".");
                serial::write_usize(func as usize);
                serial::write_str(" VID=");
                serial::write_hex(vendor as usize);
                serial::write_str(" DID=");
                serial::write_hex(device as usize);
                serial::write_str(" class=");
                serial::write_hex(class as usize);
                serial::write_str("/");
                serial::write_hex(subclass as usize);
                serial::write_str("/");
                serial::write_hex(prog_if as usize);
                serial::write_str(" rev=");
                serial::write_hex(revision as usize);
                serial::write_str(" CMD=");
                serial::write_hex(command as usize);
                serial::write_str(if command & 0x2 != 0 { " MEM" } else { " mem-off" });
                serial::write_str(if command & 0x4 != 0 { " BUSMASTER" } else { " busmaster-off" });
                serial::write_str("\n");

                if vendor == INTEL_VENDOR && device == HD3000_DID {
                    found_target = true;
                    serial::write_str(
                        "[VIDEO/PCI] Intel Sandy Bridge HD Graphics 3000 detected\n"
                    );
                }

                // Characterize the currently programmed BARs only. We deliberately
                // do NOT write 0xFFFF_FFFF to size them because that changes PCI state.
                let mut bar = 0u8;
                while bar < 6 {
                    let value = pci_read32(0, dev, func, 0x10 + bar * 4);
                    print_bar(bar, value);

                    // A 64-bit memory BAR consumes the next slot.
                    if value != 0 && value & 1 == 0 && ((value >> 1) & 0x3) == 2 {
                        if bar < 5 {
                            let upper = pci_read32(0, dev, func, 0x10 + (bar + 1) * 4);
                            serial::write_str("[VIDEO/PCI] BAR");
                            serial::write_usize((bar + 1) as usize);
                            serial::write_str(" upper=");
                            serial::write_hex(upper as usize);
                            serial::write_str("\n");
                            bar += 2;
                            continue;
                        }
                    }
                    bar += 1;
                }
            }
        }
    }

    if !found_display {
        serial::write_str("[VIDEO/PCI] no display controller found\n");
        return false;
    }

    serial::write_str(if found_target {
        "[VIDEO/PCI] STAGE0.1 PASS — identity + programmed BARs read\n"
    } else {
        "[VIDEO/PCI] STAGE0.1 PASS — display found, target absent\n"
    });
    found_target
}

/// Compatibility software-framebuffer API. Hardware acceleration is not
/// claimed here; graphics::Framebuffer remains the active display surface.
pub fn init() -> bool {
    let gpu = probe();
    if graphics::ready() {
        serial::write_str("[VIDEO] software framebuffer remains active\n");
    }
    gpu
}

pub fn ready() -> bool { graphics::ready() }
pub fn width() -> u32 { graphics::width() as u32 }
pub fn height() -> u32 { graphics::height() as u32 }
