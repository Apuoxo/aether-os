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
const IWL2030_FW: &[u8] = include_bytes!("../../build/iwlwifi-2030-6.ucode");
const IWLAGN_RTC_INST_LOWER_BOUND: u32 = 0x000000;
const IWLAGN_RTC_DATA_LOWER_BOUND: u32 = 0x800000;
const HBUS_TARG_MEM_WADDR: usize = 0x410;
const HBUS_TARG_MEM_WDAT: usize = 0x418;

const CSR_INT: usize = 0x008;
const CSR_INT_MASK: usize = 0x00C;
const CSR_FH_INT_STATUS: usize = 0x010;
const CSR_INT_BIT_FH_TX: u32 = 1 << 27;
const CSR_FH_INT_TX_MASK: u32 = 0x0000_0003;
const FH_MEM_LOWER_BOUND: usize = 0x1000;
const FH_TFDIB_CTRL0_SRVC: usize = FH_MEM_LOWER_BOUND + 0x900 + 8 * 9;
const FH_TFDIB_CTRL1_SRVC: usize = FH_TFDIB_CTRL0_SRVC + 4;
const FH_TCSR_CONFIG_SRVC: usize = FH_MEM_LOWER_BOUND + 0xD00 + 0x20 * 9;
const FH_TCSR_BUF_STS_SRVC: usize = FH_TCSR_CONFIG_SRVC + 8;
const FH_SRVC_SRAM_ADDR: usize = FH_MEM_LOWER_BOUND + 0x9C8;
const FH_MEM_TFDIB_REG1_ADDR_BITSHIFT: u32 = 28;
const FH_MEM_TB_MAX_LENGTH: usize = 0x0002_0000;
const FH_TCSR_DMA_ENABLE: u32 = 0x8000_0000;
const FH_TCSR_CIRQ_HOST_ENDTFD: u32 = 0x0010_0000;
const FH_TCSR_TFDB_VALID: u32 = 0x0000_0003;
const FH_TCSR_TB_NUM: u32 = 1 << 20;
const FH_TCSR_TB_IDX: u32 = 1 << 12;

#[repr(align(4096))]
struct FirmwareDmaBuf([u8; FH_MEM_TB_MAX_LENGTH]);
static mut FW_DMA_BUF: FirmwareDmaBuf = FirmwareDmaBuf([0; FH_MEM_TB_MAX_LENGTH]);

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
const CSR_GP_CNTRL: usize = 0x024;
const CSR_GP_CNTRL_INIT_DONE: u32 = 0x0000_0004;
const CSR_GP_CNTRL_MAC_CLOCK_READY: u32 = 0x0000_0001;

static mut ACTIVATE_ATTEMPTED: bool = false;
static mut ACTIVATE_OK: bool = false;
static mut ACTIVATE_BEFORE: u32 = 0;
static mut ACTIVATE_AFTER: u32 = 0;
static mut FW_ATTEMPTED: bool = false;
static mut FW_LOADED: bool = false;
static mut FW_VER: u32 = 0;
static mut FW_INST_SIZE: u32 = 0;
static mut FW_DATA_SIZE: u32 = 0;
static mut FW_EXEC_ATTEMPTED: bool = false;
static mut FW_EXEC_STARTED: bool = false;



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
pub fn activate_attempted() -> bool { unsafe { ACTIVATE_ATTEMPTED } }
pub fn activate_ok() -> bool { unsafe { ACTIVATE_OK } }
pub fn activate_before() -> u32 { unsafe { ACTIVATE_BEFORE } }
pub fn activate_after() -> u32 { unsafe { ACTIVATE_AFTER } }
pub fn firmware_attempted() -> bool { unsafe { FW_ATTEMPTED } }
pub fn firmware_loaded() -> bool { unsafe { FW_LOADED } }
pub fn firmware_version() -> u32 { unsafe { FW_VER } }
pub fn firmware_inst_size() -> u32 { unsafe { FW_INST_SIZE } }
pub fn firmware_data_size() -> u32 { unsafe { FW_DATA_SIZE } }
pub fn firmware_exec_attempted() -> bool { unsafe { FW_EXEC_ATTEMPTED } }
pub fn firmware_exec_started() -> bool { unsafe { FW_EXEC_STARTED } }

fn fw_le32(b: &[u8], off: usize) -> u32 {
    (b[off] as u32) | ((b[off + 1] as u32) << 8) |
    ((b[off + 2] as u32) << 16) | ((b[off + 3] as u32) << 24)
}

pub fn load_firmware() -> bool {
    unsafe {
        FW_ATTEMPTED = true;
        FW_LOADED = false;
        if !ACTIVATE_OK || MMIO == 0 || !MMIO_MAPPED {
            serial::write_str("[WIFI] FW=NOT-ATTEMPTED activation prerequisite missing\n");
            return false;
        }
        if IWL2030_FW.len() < 88 {
            serial::write_str("[WIFI] FW=INVALID file too small\n");
            return false;
        }

        let magic = fw_le32(IWL2030_FW, 4);
        const IWL_TLV_UCODE_MAGIC: u32 = 0x0A4C5749;
        if fw_le32(IWL2030_FW, 0) != 0 || magic != IWL_TLV_UCODE_MAGIC {
            serial::write_str("[WIFI] FW=UNSUPPORTED_FORMAT MAGIC=");
            serial::write_hex(magic as usize);
            serial::write_str("\n");
            return false;
        }

        let ver = fw_le32(IWL2030_FW, 72);
        let build = fw_le32(IWL2030_FW, 76);
        let api = (ver >> 8) & 0xFF;
        FW_VER = ver;
        FW_INST_SIZE = 0;
        FW_DATA_SIZE = 0;
        serial::write_str("[WIFI] FW FORMAT=TLV MAGIC=");
        serial::write_hex(magic as usize);
        serial::write_str(" VER=");
        serial::write_hex(ver as usize);
        serial::write_str(" API=");
        serial::write_usize(api as usize);
        serial::write_str(" BUILD=");
        serial::write_usize(build as usize);
        serial::write_str("\n");

        let dma_base = (&FW_DMA_BUF.0 as *const u8) as u64;
        serial::write_str("[WIFI] FW DMA_BUFFER=");
        serial::write_hex(dma_base as usize);
        serial::write_str("\n");

        let load_chunk = |dst: u32, src: &[u8]| -> bool {
            if src.is_empty() || src.len() > FH_MEM_TB_MAX_LENGTH || (src.len() & 3) != 0 {
                return false;
            }
            let mut i = 0usize;
            while i < src.len() {
                FW_DMA_BUF.0[i] = src[i];
                i += 1;
            }

            // Intel's gen1/gen2 iwlwifi transport uses FH service DMA channel 9.
            // We poll the same completion interrupt that Linux handles, avoiding
            // dependence on an Aether PCI ISR while bringing the firmware up.
            core::ptr::write_volatile((MMIO + CSR_INT_MASK) as *mut u32, CSR_INT_BIT_FH_TX);
            core::ptr::write_volatile((MMIO + CSR_FH_INT_STATUS) as *mut u32, CSR_FH_INT_TX_MASK);
            core::ptr::write_volatile((MMIO + CSR_INT) as *mut u32, CSR_INT_BIT_FH_TX);
            core::ptr::write_volatile((MMIO + FH_TCSR_CONFIG_SRVC) as *mut u32, 0);
            core::ptr::write_volatile((MMIO + FH_SRVC_SRAM_ADDR) as *mut u32, dst);
            core::ptr::write_volatile((MMIO + FH_TFDIB_CTRL0_SRVC) as *mut u32, dma_base as u32);
            core::ptr::write_volatile(
                (MMIO + FH_TFDIB_CTRL1_SRVC) as *mut u32,
                (((dma_base >> 32) as u32) << FH_MEM_TFDIB_REG1_ADDR_BITSHIFT) | src.len() as u32
            );
            core::ptr::write_volatile(
                (MMIO + FH_TCSR_BUF_STS_SRVC) as *mut u32,
                FH_TCSR_TB_NUM | FH_TCSR_TB_IDX | FH_TCSR_TFDB_VALID
            );
            core::ptr::write_volatile(
                (MMIO + FH_TCSR_CONFIG_SRVC) as *mut u32,
                FH_TCSR_DMA_ENABLE | FH_TCSR_CIRQ_HOST_ENDTFD
            );

            let mut n = 0usize;
            while n < 50_000_000 {
                let inta = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
                if (inta & CSR_INT_BIT_FH_TX) != 0 {
                    core::ptr::write_volatile((MMIO + CSR_FH_INT_STATUS) as *mut u32, CSR_FH_INT_TX_MASK);
                    core::ptr::write_volatile((MMIO + CSR_INT) as *mut u32, CSR_INT_BIT_FH_TX);
                    return true;
                }
                core::hint::spin_loop();
                n += 1;
            }
            false
        };

        let mut pos = 88usize;
        let mut inst_seen = false;
        let mut data_seen = false;
        let mut inst_size = 0usize;
        let mut data_size = 0usize;

        while pos + 8 <= IWL2030_FW.len() {
            let tlv_type = fw_le32(IWL2030_FW, pos);
            let tlv_len = fw_le32(IWL2030_FW, pos + 4) as usize;
            let data_start = pos + 8;
            let data_end = match data_start.checked_add(tlv_len) {
                Some(v) => v,
                None => return false,
            };
            if data_end > IWL2030_FW.len() { return false; }

            let aligned_len = (tlv_len + 3) & !3usize;
            let next = match data_start.checked_add(aligned_len) {
                Some(v) => v,
                None => return false,
            };
            if next > IWL2030_FW.len() { return false; }

            let (dst, seen) = match tlv_type {
                1 if !inst_seen => {
                    inst_seen = true;
                    inst_size = tlv_len;
                    (IWLAGN_RTC_INST_LOWER_BOUND, true)
                }
                2 if !data_seen => {
                    data_seen = true;
                    data_size = tlv_len;
                    (IWLAGN_RTC_DATA_LOWER_BOUND, true)
                }
                _ => (0, false),
            };

            if seen {
                let mut off = 0usize;
                while off < tlv_len {
                    let chunk = core::cmp::min(FH_MEM_TB_MAX_LENGTH, tlv_len - off) & !3usize;
                    if chunk == 0 || !load_chunk(
                        dst + off as u32,
                        &IWL2030_FW[data_start + off..data_start + off + chunk]
                    ) {
                        serial::write_str("[WIFI] FW SERVICE-DMA=TIMEOUT dst=");
                        serial::write_hex((dst + off) as usize);
                        serial::write_str("\n");
                        return false;
                    }
                    off += chunk;
                }
                serial::write_str("[WIFI] FW SERVICE-DMA section dst=");
                serial::write_hex(dst as usize);
                serial::write_str(" bytes=");
                serial::write_usize(tlv_len);
                serial::write_str(" OK\n");
            }

            pos = next;
        }

        if !inst_seen || !data_seen {
            serial::write_str("[WIFI] FW TLV=RUNTIME_SECTIONS_MISSING\n");
            return false;
        }

        FW_INST_SIZE = inst_size as u32;
        FW_DATA_SIZE = data_size as u32;
        FW_LOADED = true;
        NEEDS_FW = false;
        serial::write_str("[WIFI] FW LOAD=OK via Intel FH service DMA\n");
        true
    }
}

/// Release the Intel 2000/2030 runtime uCode from host reset after its
/// runtime instruction/data sections have been written to device SRAM.
/// This is deliberately a boot-only step: Aether does not claim ALIVE until
/// the firmware notification is received through the future RX/interrupt path.
pub fn start_firmware() -> bool {
    unsafe {
        FW_EXEC_ATTEMPTED = false;
        FW_EXEC_STARTED = false;
        if !FW_LOADED || !ACTIVATE_OK || MMIO == 0 || !MMIO_MAPPED {
            serial::write_str("[WIFI] FW EXEC=NOT-ATTEMPTED prerequisite missing\n");
            return false;
        }
        FW_EXEC_ATTEMPTED = true;
        let gp1_clr = (MMIO + 0x05C) as *mut u32;
        let csr = (MMIO + CSR_RESET) as *mut u32;
        core::ptr::write_volatile(gp1_clr, 0x0000_0006);
        core::ptr::write_volatile(csr, 0);
        let after = core::ptr::read_volatile(csr);
        FW_EXEC_STARTED = after == 0;
        serial::write_str("[WIFI] FW EXEC RELEASE_RESET=0 READBACK=");
        serial::write_hex(after as usize);
        serial::write_str(" RESULT=");
        serial::write_str(if FW_EXEC_STARTED { "STARTED" } else { "FAILED" });
        serial::write_str(" ALIVE=NOT-YET RX/IRQ=NOT-STARTED\n");
        FW_EXEC_STARTED
    }
}

/// Wake the pre-8000 Intel MAC after reset, matching iwlwifi's gen1/gen2
/// activate_nic stage: set INIT_DONE and wait for MAC_CLOCK_READY.
pub fn activate_nic() -> bool {
    unsafe {
        ACTIVATE_ATTEMPTED = false;
        ACTIVATE_OK = false;
        ACTIVATE_BEFORE = 0;
        ACTIVATE_AFTER = 0;
        if !RESET_OK || MMIO == 0 || !MMIO_MAPPED {
            serial::write_str("[WIFI] ACTIVATE=NOT-ATTEMPTED reset prerequisite missing\n");
            return false;
        }
        let gp = (MMIO + CSR_GP_CNTRL) as *mut u32;
        let before = core::ptr::read_volatile(gp);
        ACTIVATE_BEFORE = before;
        ACTIVATE_ATTEMPTED = true;
        core::ptr::write_volatile(gp, before | CSR_GP_CNTRL_INIT_DONE);
        let mut ready = 0u32;
        let mut n = 0usize;
        while n < 50_000_000 {
            ready = core::ptr::read_volatile(gp);
            if (ready & CSR_GP_CNTRL_MAC_CLOCK_READY) != 0 { break; }
            core::hint::spin_loop();
            n += 1;
        }
        ACTIVATE_AFTER = ready;
        ACTIVATE_OK = (ready & CSR_GP_CNTRL_MAC_CLOCK_READY) != 0;
        serial::write_str("[WIFI] ACTIVATE INIT_DONE=SET BEFORE=");
        serial::write_hex(before as usize);
        serial::write_str(" AFTER=");
        serial::write_hex(ready as usize);
        serial::write_str(" RESULT=");
        serial::write_str(if ACTIVATE_OK { "MAC_CLOCK_READY" } else { "TIMEOUT" });
        serial::write_str("\n");
        ACTIVATE_OK
    }
}


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
            serial::write_str("[WIFI] RESET=NOT-ATTEMPTED prerequisite missing\n");
            return false;
        }

        let csr = (MMIO + CSR_RESET) as *mut u32;
        let before = core::ptr::read_volatile(csr);
        RESET_BEFORE = before;
        RESET_ATTEMPTED = true;
        serial::write_str("[WIFI] RESET path=INTEL_CSR_RESET SW_RESET=0x80 BEFORE=");
        serial::write_hex(before as usize);
        serial::write_str("\n");

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
        serial::write_str("\n");
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
            serial::write_str("[WIFI] CAPS=NO-DEVICE\n");
            return;
        }

        let p = pci_r32(BUS, DEV, FUNC, 0x34);
        let mut ptr = (p & 0xFF) as u8;
        CAP_PTR = ptr;
        let mut count = 0usize;

        serial::write_str("[WIFI] CAPS PTR=");
        serial::write_hex(ptr as usize);
        serial::write_str("\n");

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
            serial::write_str("\n");

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
        serial::write_str(" READ=YES\n");
    }
}

/// Read-only PCI prerequisite snapshot for the Intel 2230 bring-up stage.
/// No device reset, firmware load, interrupt enable, DMA, TX/RX, or association.
pub fn probe_prerequisites() {
    unsafe {
        if !FOUND {
            serial::write_str("[WIFI] PREREQ=NO-DEVICE\n");
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
        serial::write_str("\n");
        serial::write_str("[WIFI] PREREQ RESET=NOT-TOUCHED INTERRUPTS=NOT-ENABLED DMA=NOT-STARTED\n");
        serial::write_str("[WIFI] PREREQ firmware loader=NOT-IMPLEMENTED (contract only)\n");
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
                    serial::write_str(" (READ-ONLY)\n");
                    return;
                }
            }
        }
        FOUND = false;
        READY = false;
        NEEDS_FW = true;
        serial::write_str("[WIFI] SURVEY no Intel WLAN on buses 0..31\n");
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
