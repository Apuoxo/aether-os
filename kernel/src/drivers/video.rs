//! Aether Intel graphics driver — Stage 0: PCI identification only.
//!
//! This is intentionally a read-only first stage.
//! No MMIO access, BAR mapping, register writes, GMBUS, or mode programming
//! is performed here. The goal is to establish the GPU identity safely before
//! touching any graphics-engine or display registers.
//
// Target for the reference AH532:
//!   Intel Sandy Bridge integrated graphics, DID 0x0116 (HD Graphics 3000).
//!
//! Next stages will be added only after the PCI identity and BAR layout are
//! confirmed on real hardware.

use crate::serial;

const PCI_CONFIG_ADDR: u16 = 0x0CF8;
const PCI_CONFIG_DATA: u16 = 0x0CFC;
const INTEL_VENDOR: u16 = 0x8086;
const DISPLAY_CLASS: u8 = 0x03;
const HD3000_DID: u16 = 0x0116;

unsafe fn pci_addr(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC)
}

unsafe fn pci_read32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let addr = pci_addr(bus, dev, func, off);
    core::arch::asm!(
        "out dx, eax",
        in("dx") PCI_CONFIG_ADDR,
        in("eax") addr,
        options(nostack, preserves_flags)
    );

    let value: u32;
    core::arch::asm!(
        "in eax, dx",
        in("dx") PCI_CONFIG_DATA,
        out("eax") value,
        options(nostack, preserves_flags)
    );
    value
}

unsafe fn pci_read16(bus: u8, dev: u8, func: u8, off: u8) -> u16 {
    let value = pci_read32(bus, dev, func, off & !3);
    let shift = ((off & 2) as u32) * 8;
    ((value >> shift) & 0xFFFF) as u16
}

/// Read-only PCI probe for the display controller on bus 0.
///
/// Returns true only for the expected Intel HD Graphics 3000 device.
/// Unknown display controllers are reported but never touched beyond PCI
/// configuration-space reads.
pub fn probe() -> bool {
    serial::write_str("\n======== INTEL VIDEO PROBE ========\n");

    let mut found_display = false;
    let mut found_target = false;

    for dev in 0u8..32 {
        for func in 0u8..8 {
            unsafe {
                let id = pci_read32(0, dev, func, 0x00);
                if id == 0 || id == 0xFFFF_FFFF {
                    continue;
                }

                let vendor = (id & 0xFFFF) as u16;
                let device = ((id >> 16) & 0xFFFF) as u16;

                let class_reg = pci_read32(0, dev, func, 0x08);
                let class = (class_reg >> 24) as u8;
                let subclass = (class_reg >> 16) as u8;
                let prog_if = (class_reg >> 8) as u8;
                let revision = class_reg as u8;

                if class != DISPLAY_CLASS {
                    continue;
                }

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
                serial::write_str("\n");

                if vendor == INTEL_VENDOR && device == HD3000_DID {
                    found_target = true;
                    serial::write_str(
                        "[VIDEO/PCI] Intel Sandy Bridge HD Graphics 3000 detected\n"
                    );
                } else {
                    serial::write_str("[VIDEO/PCI] display device is not target DID\n");
                }

                // Only read BARs. Do not enable memory space, bus mastering,
                // or write any PCI configuration registers in Stage 0.
                let mut bar = 0u8;
                while bar < 6 {
                    let value = pci_read32(0, dev, func, 0x10 + bar * 4);
                    serial::write_str("[VIDEO/PCI] BAR");
                    serial::write_usize(bar as usize);
                    serial::write_str("=");
                    serial::write_hex(value as usize);
                    serial::write_str("\n");
                    bar += 1;
                }
            }
        }
    }

    if !found_display {
        serial::write_str("[VIDEO/PCI] no display controller found\n");
        return false;
    }

    if found_target {
        serial::write_str("[VIDEO/PCI] STAGE0 PASS — read-only identity\n");
    } else {
        serial::write_str("[VIDEO/PCI] STAGE0 PASS — display found, target absent\n");
    }

    found_target
}

pub fn init() -> bool {
    probe()
}
