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

// Intel 2230 uses the 2030 firmware family. Keep the firmware contract
// explicit; Aether does not embed or claim to load an arbitrary blob.
const IWL2030_FW_API_MIN: u32 = 5;
const IWL2030_FW_API_MAX: u32 = 6;
const IWL2030_FW_PREFIX: &str = "iwlwifi-2030-";

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
static mut MMIO_MAPPED: bool = false;
static mut PCI_COMMAND: u16 = 0;
static mut PCI_STATUS: u16 = 0;
static mut IRQ_LINE: u8 = 0;
static mut IRQ_PIN: u8 = 0;
static mut PREREQS_READ: bool = false;
static mut CAP_PTR: u8 = 0;
static mut CAP_MSI: bool = false;
static mut CAP_MSIX: bool = false;
static mut CAP_PM: bool = false;
static mut CAP_CHAIN_READ: bool = false;
static mut MSI_CTRL: u16 = 0;
static mut PCIE_CAP: bool = false;
static mut PCIE_LINK_STATUS: u16 = 0;
static mut PCIE_DEVICE_STATUS: u16 = 0;
static mut PCIE_LINK_SPEED: u8 = 0;
static mut PCIE_LINK_WIDTH: u8 = 0;
static mut PCIE_CAP_OFF: u8 = 0;
static mut PCIE_FLR_SUPPORTED: bool = false;
// Device-specific reset state. PCIe FLR is absent on the AH532 2230, so
// bring-up uses the Intel iwlwifi CSR software-reset path instead of inventing
// a PCI config reset. No firmware, interrupts, or DMA are started here.
static mut RESET_ATTEMPTED: bool = false;
static mut RESET_OK: bool = false;
static mut RESET_BEFORE: u32 = 0;
static mut RESET_AFTER: u32 = 0;

const CSR_RESET: usize = 0x020;
const CSR_RESET_SW_RESET: u32 = 0x0000_0080;



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
    if phys == 0 || size == 0 || (phys & 0xFFF) != 0 {
        return false;
    }
    let cr3 = unsafe { paging::read_cr3() };
    let start = (phys as usize) & !0xFFF;
    let end = (phys as usize + size + 0xFFF) & !0xFFF;
    let mut va = start;
    while va < end {
        if va >= 0x4000_0000 {
            let ok = unsafe {
                paging::map_page(cr3, va, va, paging::PAGE_PRESENT | paging::PAGE_WRITE)
            };
            if !ok { return false; }
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

pub fn mmio_ready() -> bool {
    unsafe { MMIO_MAPPED }
}

/// Firmware filename contract for the supported Intel 2230 API range.
/// This does not load firmware; the native loader is a separate stage.
pub fn firmware_name(api: u32) -> Option<&'static str> {
    if api < IWL2030_FW_API_MIN || api > IWL2030_FW_API_MAX { return None; }
    if api == 5 { Some("iwlwifi-2030-5.ucode") } else { Some("iwlwifi-2030-6.ucode") }
}

pub fn firmware_prefix() -> &'static str {
    IWL2030_FW_PREFIX
}
pub fn bar0() -> u64 {
    unsafe { BAR0 }
}
pub fn bus_dev_func() -> (u8, u8, u8) {
    unsafe { (BUS, DEV, FUNC) }
}

pub fn pci_command() -> u16 { unsafe { PCI_COMMAND } }
pub fn pci_status() -> u16 { unsafe { PCI_STATUS } }
pub fn irq_line() -> u8 { unsafe { IRQ_LINE } }
pub fn irq_pin() -> u8 { unsafe { IRQ_PIN } }
pub fn prerequisites_read() -> bool { unsafe { PREREQS_READ } }
pub fn cap_ptr() -> u8 { unsafe { CAP_PTR } }
pub fn cap_msi() -> bool { unsafe { CAP_MSI } }
pub fn cap_msix() -> bool { unsafe { CAP_MSIX } }
pub fn cap_pm() -> bool { unsafe { CAP_PM } }
pub fn cap_chain_read() -> bool { unsafe { CAP_CHAIN_READ } }
pub fn msi_ctrl() -> u16 { unsafe { MSI_CTRL } }
pub fn pcie_cap() -> bool { unsafe { PCIE_CAP } }
pub fn pcie_link_status() -> u16 { unsafe { PCIE_LINK_STATUS } }
pub fn pcie_device_status() -> u16 { unsafe { PCIE_DEVICE_STATUS } }
pub fn pcie_link_speed() -> u8 { unsafe { PCIE_LINK_SPEED } }
pub fn pcie_link_width() -> u8 { unsafe { PCIE_LINK_WIDTH } }
pub fn pcie_flr_supported() -> bool { unsafe { PCIE_FLR_SUPPORTED } }
pub fn reset_attempted() -> bool { unsafe { RESET_ATTEMPTED } }
pub fn reset_ok() -> bool { unsafe { RESET_OK } }
pub fn reset_before() -> u32 { unsafe { RESET_BEFORE } }
pub fn reset_after() -> u32 { unsafe { RESET_AFTER } }

/// Execute the device-specific Intel 2000/2030-family software reset used by
/// iwlwifi for pre-8000 devices. The Linux driver writes SW_RESET to CSR_RESET
/// and waits 5-6 ms. We deliberately do not touch PCI MSI/MSI-X, DMA rings,
/// firmware, or association state in this stage.
pub fn software_reset() -> bool {
    unsafe {
        RESET_ATTEMPTED = false;
        RESET_OK = false;
        RESET_BEFORE = 0;
        RESET_AFTER = 0;
        if !FOUND || !MMIO_MAPPED || MMIO == 0 {
            serial::write_str("[WIFI] RESET=NOT-ATTEMPTED prerequisite missing\\n");
            return false;
        }

        let csr = (MMIO + CSR_RESET) as *mut u32;
        let before = core::ptr::read_volatile(csr);
        RESET_BEFORE = before;
        RESET_ATTEMPTED = true;
        serial::write_str("[WIFI] RESET path=INTEL_CSR_RESET SW_RESET=0x80 BEFORE=");
        serial::write_hex(before as usize);
        serial::write_str("\\n");

        // Exact device-family operation documented in iwlwifi gen1/gen2:
        // set CSR_RESET_REG_FLAG_SW_RESET, then wait 5-6 ms.
        core::ptr::write_volatile(csr, before | CSR_RESET_SW_RESET);

        // Aether currently has no calibrated microsecond delay primitive.
        // This bounded PAUSE loop is intentionally conservative and is only
        // used for this hardware-reset settling interval.
        let mut n = 0usize;
        while n < 10_000_000 {
            core::hint::spin_loop();
            n += 1;
        }

        let after = core::ptr::read_volatile(csr);
        RESET_AFTER = after;
        RESET_OK = after != 0xFFFF_FFFF && after != 0xFFFF_FFFE;
        serial::write_str("[WIFI] RESET AFTER=");
        serial::write_hex(after as usize);
        serial::write_str(" RESULT=");
        serial::write_str(if RESET_OK { "READABLE" } else { "MMIO-FAIL" });
        serial::write_str("\\n");
        RESET_OK
    }
}


/// Read-only PCI capability-chain snapshot for the Intel 2230.
/// No capability is modified and no interrupt mode is enabled.
pub fn probe_capabilities() {
    unsafe {
        CAP_PTR = 0;
        CAP_MSI = false;
        CAP_MSIX = false;
        CAP_PM = false;
        CAP_CHAIN_READ = false;
        MSI_CTRL = 0;
        PCIE_CAP = false;
        PCIE_LINK_STATUS = 0;
        PCIE_DEVICE_STATUS = 0;
        PCIE_LINK_SPEED = 0;
        PCIE_LINK_WIDTH = 0;
        PCIE_CAP_OFF = 0;
        PCIE_FLR_SUPPORTED = false;
        if !FOUND {
            serial::write_str("[WIFI] CAPS=NO-DEVICE\\n");
            return;
        }

        let p = pci_r32(BUS, DEV, FUNC, 0x34);
        let mut ptr = (p & 0xFF) as u8;
        CAP_PTR = ptr;
        let mut count = 0usize;

        serial::write_str("[WIFI] CAPS PTR=");
        serial::write_hex(ptr as usize);
        serial::write_str("\\n");

        while ptr >= 0x40 && count < 48 {
            let off = ptr & 0xFC;
            let word = pci_r32(BUS, DEV, FUNC, off);
            let cap_id = (word & 0xFF) as u8;
            let next = ((word >> 8) & 0xFF) as u8;
            serial::write_str("[WIFI] CAP off=");
            serial::write_hex(ptr as usize);
            serial::write_str(" ID=");
            serial::write_hex(cap_id as usize);
            serial::write_str(" NEXT=");
            serial::write_hex(next as usize);
            serial::write_str("\\n");

            match cap_id {
                0x01 => { CAP_PM = true; }
                0x05 => {
                    CAP_MSI = true;
                    MSI_CTRL = ((word >> 16) & 0xFFFF) as u16;
                }
                0x10 => {
                    PCIE_CAP = true;
                    PCIE_CAP_OFF = off;
                    let pcie = pci_r32(BUS, DEV, FUNC, off.wrapping_add(0x0C));
                    PCIE_LINK_STATUS = (pcie & 0xFFFF) as u16;
                    PCIE_LINK_SPEED = (PCIE_LINK_STATUS & 0x000F) as u8;
                    PCIE_LINK_WIDTH = ((PCIE_LINK_STATUS >> 4) & 0x003F) as u8;
                    let pcie2 = pci_r32(BUS, DEV, FUNC, off.wrapping_add(0x08));
                    PCIE_DEVICE_STATUS = ((pcie2 >> 16) & 0xFFFF) as u16;
                    let devcap2 = pci_r32(BUS, DEV, FUNC, off.wrapping_add(0x24));
                    PCIE_FLR_SUPPORTED = (devcap2 & (1u32 << 28)) != 0;
                }
                0x11 => { CAP_MSIX = true; }
                _ => {}
            }

            if next == 0 || next == ptr { break; }
            ptr = next & 0xFC;
            count += 1;
        }
        CAP_CHAIN_READ = true;
        serial::write_str("[WIFI] CAPS PM=");
        serial::write_str(if CAP_PM { "YES" } else { "NO" });
        serial::write_str(" MSI=");
        serial::write_str(if CAP_MSI { "YES" } else { "NO" });
        serial::write_str(" MSIX=");
        serial::write_str(if CAP_MSIX { "YES" } else { "NO" });
        serial::write_str(" READ=YES\\n");
    }
}

/// Read-only PCI prerequisite snapshot for the Intel 2230 bring-up stage.
/// No device reset, firmware load, interrupt enable, DMA, TX/RX, or association.
pub fn probe_prerequisites() {
    unsafe {
        if !FOUND {
            serial::write_str("[WIFI] PREREQ=NO-DEVICE\\n");
            return;
        }
        let cmdstat = pci_r32(BUS, DEV, FUNC, 0x04);
        PCI_COMMAND = (cmdstat & 0xFFFF) as u16;
        PCI_STATUS = (cmdstat >> 16) as u16;
        let il = pci_r32(BUS, DEV, FUNC, 0x3C);
        IRQ_LINE = (il & 0xFF) as u8;
        IRQ_PIN = ((il >> 8) & 0xFF) as u8;
        PREREQS_READ = true;
        serial::write_str("[WIFI] PREREQ PCI_CMD=");
        serial::write_hex(PCI_COMMAND as usize);
        serial::write_str(" STATUS=");
        serial::write_hex(PCI_STATUS as usize);
        serial::write_str(" IRQ_LINE=");
        serial::write_usize(IRQ_LINE as usize);
        serial::write_str(" IRQ_PIN=");
        serial::write_usize(IRQ_PIN as usize);
        serial::write_str(" MMIO=");
        serial::write_str(if MMIO_MAPPED { "READY" } else { "NOT-MAPPED" });
        serial::write_str(" FW=");
        serial::write_str(if NEEDS_FW { "REQUIRED" } else { "LOADED" });
        serial::write_str("\\n");
        serial::write_str("[WIFI] PREREQ RESET=NOT-TOUCHED INTERRUPTS=NOT-ENABLED DMA=NOT-STARTED\\n");
        serial::write_str("[WIFI] PREREQ firmware loader=NOT-IMPLEMENTED (contract only)\\n");
    }
}

/// Fresh read-only PCI survey for WF. It never enables PCI command bits,
/// maps MMIO, resets the device, enables interrupts, starts DMA, or loads firmware.
pub fn survey() {
    unsafe {
        for bus in 0u8..=31 {
            for dev in 0u8..32 {
                for func in 0u8..8 {
                    let id = pci_r32(bus, dev, func, 0);
                    if id == 0xFFFF_FFFF || id == 0 { continue; }
                    let vid = (id & 0xFFFF) as u16;
                    let did = ((id >> 16) & 0xFFFF) as u16;
                    let cr = pci_r32(bus, dev, func, 0x08);
                    let class = ((cr >> 24) & 0xFF) as u8;
                    let sub = ((cr >> 16) & 0xFF) as u8;
                    if vid != INTEL_VID || class != 0x02 || sub != 0x80 { continue; }
                    let bar0 = (pci_r32(bus, dev, func, 0x10) as u64) & !0xF;
                    let subsys = (pci_r32(bus, dev, func, 0x2C) >> 16) as u16;
                    FOUND = true;
                    BUS = bus;
                    DEV = dev;
                    FUNC = func;
                    BAR0 = bar0;
                    SUBSYS = subsys;
                    NEEDS_FW = true;
                    serial::write_str("[WIFI] SURVEY FOUND ");
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
                    serial::write_str(" (READ-ONLY)\\n");
                    return;
                }
            }
        }
        FOUND = false;
        READY = false;
        NEEDS_FW = true;
        serial::write_str("[WIFI] SURVEY no Intel WLAN on buses 0..31\\n");
    }
}

/// Force probe Intel Centrino Wireless-N 2230 (AH532) and any 02:80 Intel WLAN
pub fn init() {
    serial::write_str("[WIFI] AH532 target: Centrino Wireless-N 2230 (8086:0887)\n");
    unsafe {
        FOUND = false;
        READY = false;
        NEEDS_FW = true;
        MMIO_MAPPED = false;
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
                        MMIO_MAPPED = true;
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
                    serial::write_str("[WIFI] firmware contract: iwlwifi-2030-5/6.ucode\n");
                    serial::write_str("[WIFI] assoc/TX requires firmware loader — NEEDS_FW\n");
                    return;
                }
            }
        }
        serial::write_str("[WIFI] no Intel WLAN on buses 0..31\n");
    }
}
