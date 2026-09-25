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

// WF diagnostic sink. A single WF command renders the complete Wi-Fi
// bring-up trace in the desktop terminal; Serial is only a fallback.
static mut WF_GUI_OUTPUT: bool = false;

pub fn set_wf_gui_output(enabled: bool) {
    unsafe { WF_GUI_OUTPUT = enabled; }
}

fn diag_write_str(s: &str) {
    unsafe { if WF_GUI_OUTPUT { crate::desktop::terminal_write(s); return; } }
    serial::write_str(s);
}

fn diag_write_usize(v: usize) {
    unsafe {
        if WF_GUI_OUTPUT {
            if v == 0 { crate::desktop::terminal_write("0"); return; }
            let mut n=v; let mut b=[0u8;20]; let mut k=0usize;
            while n>0 { b[k]=b'0'+(n%10) as u8; n/=10; k+=1; }
            let mut o=[0u8;20]; let mut i=0usize;
            while i<k { o[i]=b[k-1-i]; i+=1; }
            if let Ok(x)=core::str::from_utf8(&o[..k]) { crate::desktop::terminal_write(x); }
            return;
        }
    }
    serial::write_usize(v);
}

fn diag_write_hex(v: usize) {
    unsafe {
        if WF_GUI_OUTPUT {
            if v == 0 { crate::desktop::terminal_write("0"); return; }
            let mut n=v; let mut b=[0u8;16]; let mut k=0usize;
            while n>0 { let x=(n&0xF) as u8; b[k]=if x<10 { b'0'+x } else { b'a'+x-10 }; n>>=4; k+=1; }
            let mut o=[0u8;16]; let mut i=0usize;
            while i<k { o[i]=b[k-1-i]; i+=1; }
            if let Ok(x)=core::str::from_utf8(&o[..k]) { crate::desktop::terminal_write(x); }
            return;
        }
    }
    serial::write_hex(v);
}

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
const CSR_INT_BIT_FH_RX: u32 = 1 << 31;
const CSR_INT_BIT_ALIVE: u32 = 1 << 0;
const CSR_FH_INT_RX_MASK: u32 = (1 << 17) | (1 << 16);
const FH_RSCSR_CHNL0: usize = FH_MEM_LOWER_BOUND + 0xBC0;
const FH_RSCSR_STTS_WPTR: usize = FH_RSCSR_CHNL0 + 0x0;
const FH_RSCSR_RBDCB_BASE: usize = FH_RSCSR_CHNL0 + 0x4;
const FH_RSCSR_RBDCB_WPTR: usize = FH_RSCSR_CHNL0 + 0x8;
const FH_RSCSR_RDPTR: usize = FH_RSCSR_CHNL0 + 0xC;
const FH_RCSR_CHNL0_CONFIG: usize = FH_MEM_LOWER_BOUND + 0xC00;
const FH_RCSR_CHNL0_FLUSH_RB_REQ: usize = FH_RCSR_CHNL0_CONFIG + 0x10;
const FH_RSSR_RX_STATUS: usize = FH_MEM_LOWER_BOUND + 0xC40;
const FH_RSCSR_FRAME_INVALID: u32 = 0x5555_0000;
const FH_RX_RBD_COUNT: usize = 256;
const FH_RX_BUF_SIZE: usize = 4096;
const FH_RX_IRQ_VECTOR: u8 = 0x27;
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
const FH_MEM_CBBC_CMD: usize = FH_MEM_LOWER_BOUND + 0x9D0 + 4 * 4;
const FH_TCSR_CONFIG_CMD: usize = FH_MEM_LOWER_BOUND + 0xD00 + 0x20 * IWL_CMD_FIFO_NUM as usize;
const FH_TCSR_BUF_STS_CMD: usize = FH_TCSR_CONFIG_CMD + 8;
const FH_TCSR_TX_CMD_DMA_ENABLE: u32 = 0x8000_0000;
const FH_TCSR_TX_CMD_CIRQ_HOST_ENDTFD: u32 = 0x0010_0000;
const FH_TCSR_TX_CMD_CREDIT_ENABLE: u32 = 0x0000_0008;
// DVM command queue #4 is a 256-entry circular TFD queue.
const FH_TFD_CMD_SLOTS: usize = 256;
const FH_TFD_SIZE: usize = 128;
const IWL_DEFAULT_CMD_QUEUE_NUM: usize = 4;
const IWL_CMD_FIFO_NUM: u32 = 7;
const SCD_BASE: u32 = 0x00A0_2C00;
const SCD_SRAM_BASE_ADDR: u32 = SCD_BASE + 0x00;
const SCD_DRAM_BASE_ADDR: u32 = SCD_BASE + 0x08;
const SCD_TXFACT: u32 = SCD_BASE + 0x10;
const SCD_QUEUECHAIN_SEL: u32 = SCD_BASE + 0xE8;
const SCD_QUEUE_WRPTR: u32 = SCD_BASE + 0x18 + (IWL_DEFAULT_CMD_QUEUE_NUM as u32 * 4);
const SCD_QUEUE_RDPTR: u32 = SCD_BASE + 0x68 + (IWL_DEFAULT_CMD_QUEUE_NUM as u32 * 4);
const SCD_CHAINEXT_EN: u32 = SCD_BASE + 0x244;
const SCD_QUEUE_STATUS_BITS: u32 = SCD_BASE + 0x10C + (IWL_DEFAULT_CMD_QUEUE_NUM as u32 * 4);
const SCD_QUEUE_CTX: u32 = 0x0600 + (IWL_DEFAULT_CMD_QUEUE_NUM as u32 * 8);
const SCD_GP_CTRL: u32 = SCD_BASE + 0x1A8;
const SCD_EN_CTRL: u32 = SCD_BASE + 0x254;
const SCD_GP_CTRL_ENABLE_31_QUEUES: u32 = 1 << 0;
const SCD_QUEUE_ACTIVE: u32 = 1 << 3;
const SCD_QUEUE_WSL: u32 = 1 << 4;
// Intel 2000/2030 uses the agn/5000 scheduler status layout.\nconst SCD_QUEUE_STATUS_MASK: u32 = 0x017F_0000;
const SCD_WIN_SIZE: u32 = 64;
const SCD_FRAME_LIMIT: u32 = 64;
const SCD_QUEUE_COUNT: usize = 11;
const SCD_QUEUE_BC_SIZE: usize = 320; // 256 TFD entries + 64 duplicate entries

#[repr(align(256))]
struct CmdTfdQueue([u8; FH_TFD_CMD_SLOTS * FH_TFD_SIZE]);
static mut CMD_TFD_QUEUE: CmdTfdQueue = CmdTfdQueue([0; FH_TFD_CMD_SLOTS * FH_TFD_SIZE]);
#[repr(align(256))]
struct ScdBcTable([u16; SCD_QUEUE_COUNT * SCD_QUEUE_BC_SIZE]);
static mut SCD_BC_TABLE: ScdBcTable = ScdBcTable([0; SCD_QUEUE_COUNT * SCD_QUEUE_BC_SIZE]);
static mut CMD_QUEUE_READY: bool = false;
static mut CMD_WRITE_PTR: usize = 0;
static mut CMD_SEQ: u8 = 0;


#[repr(align(4096))]
struct FirmwareDmaBuf([u8; FH_MEM_TB_MAX_LENGTH]);
static mut FW_DMA_BUF: FirmwareDmaBuf = FirmwareDmaBuf([0; FH_MEM_TB_MAX_LENGTH]);
#[repr(align(256))]
struct RxRbd([u32; FH_RX_RBD_COUNT]);
#[repr(align(4096))]
struct RxBuffers([[u8; FH_RX_BUF_SIZE]; FH_RX_RBD_COUNT]);
#[repr(align(16))]
struct RxStatus([u32; 3]);
static mut RX_RBD: RxRbd = RxRbd([0; FH_RX_RBD_COUNT]);
static mut RX_BUFFERS: RxBuffers = RxBuffers([[0; FH_RX_BUF_SIZE]; FH_RX_RBD_COUNT]);
static mut RX_STATUS: RxStatus = RxStatus([0; 3]);
static mut RX_READY: bool = false;
static mut RX_READ: usize = 0;
static mut RX_IRQ_COUNT: u32 = 0;
static mut ALIVE_SEEN: bool = false;
static mut ALIVE_VALID: u32 = 0;
static mut ALIVE_SUBTYPE: u8 = 0;
static mut SCAN_NOTIFICATION_SEEN: bool = false;
static mut SCAN_START_COUNT: u32 = 0;
static mut SCAN_RESULTS_COUNT: u32 = 0;
static mut SCAN_COMPLETE_COUNT: u32 = 0;
static mut SCAN_COMPLETE_CHANNELS: u8 = 0;
static mut SCAN_COMPLETE_STATUS: u8 = 0;
static mut SCAN_COMPLETE_LAST_CHANNEL: u8 = 0;

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
pub fn rx_ready() -> bool { unsafe { RX_READY } }
pub fn rx_irq_count() -> u32 { unsafe { RX_IRQ_COUNT } }
pub fn alive_seen() -> bool { unsafe { ALIVE_SEEN } }
pub fn alive_valid() -> u32 { unsafe { ALIVE_VALID } }
pub fn alive_subtype() -> u8 { unsafe { ALIVE_SUBTYPE } }
fn scan_notification_seen() -> bool { unsafe { SCAN_NOTIFICATION_SEEN } }
fn scan_start_count() -> u32 { unsafe { SCAN_START_COUNT } }
fn scan_results_count() -> u32 { unsafe { SCAN_RESULTS_COUNT } }
fn scan_complete_count() -> u32 { unsafe { SCAN_COMPLETE_COUNT } }

pub fn wf_post_scan_diagnostics() {
    unsafe {
        if !MMIO_MAPPED || MMIO == 0 {
            diag_write_str("[WIFI] WF_RESULT=NO-MMIO\n");
            return;
        }
        let csr_int = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
        let fh_int = core::ptr::read_volatile((MMIO + CSR_FH_INT_STATUS) as *const u32);
        let hw_rptr = core::ptr::read_volatile((MMIO + FH_RSCSR_RDPTR) as *const u32);
        let cb_wptr = core::ptr::read_volatile((MMIO + FH_RSCSR_RBDCB_WPTR) as *const u32);
        let stts_wptr = core::ptr::read_volatile((MMIO + FH_RSCSR_STTS_WPTR) as *const u32);
        let rx_status = core::ptr::read_volatile((MMIO + FH_RSSR_RX_STATUS) as *const u32);
        let rbd_base = core::ptr::read_volatile((MMIO + FH_RSCSR_RBDCB_BASE) as *const u32);
        let rx_cfg = core::ptr::read_volatile((MMIO + FH_RCSR_CHNL0_CONFIG) as *const u32);
        let flush = core::ptr::read_volatile((MMIO + FH_RCSR_CHNL0_FLUSH_RB_REQ) as *const u32);
        diag_write_str("[WIFI] WF-RX FINAL read="); diag_write_usize(RX_READ);
        diag_write_str(" hw_closed_rb="); diag_write_hex(hw_rptr as usize);
        diag_write_str(" cb_wptr="); diag_write_hex(cb_wptr as usize);
        diag_write_str(" stts_wptr="); diag_write_hex(stts_wptr as usize);
        diag_write_str(" rx_status="); diag_write_hex(rx_status as usize);
        diag_write_str(" rbd_base="); diag_write_hex(rbd_base as usize);
        diag_write_str(" cfg="); diag_write_hex(rx_cfg as usize);
        diag_write_str(" flush="); diag_write_hex(flush as usize);
        diag_write_str(" CSR_INT="); diag_write_hex(csr_int as usize);
        diag_write_str(" FH_INT="); diag_write_hex(fh_int as usize); diag_write_str("\n");

        diag_write_str("[WIFI] WF-CMD FINAL wrptr="); diag_write_hex(prph_read(SCD_QUEUE_WRPTR) as usize);
        diag_write_str(" rdptr="); diag_write_hex(prph_read(SCD_QUEUE_RDPTR) as usize);
        diag_write_str(" status="); diag_write_hex(prph_read(SCD_QUEUE_STATUS_BITS) as usize);
        diag_write_str(" dram="); diag_write_hex(prph_read(SCD_DRAM_BASE_ADDR) as usize);
        diag_write_str(" cbbc="); diag_write_hex(core::ptr::read_volatile((MMIO + FH_MEM_CBBC_CMD) as *const u32) as usize);
        diag_write_str(" tcsr="); diag_write_hex(core::ptr::read_volatile((MMIO + FH_TCSR_CONFIG_CMD) as *const u32) as usize);
        diag_write_str(" tsts="); diag_write_hex(core::ptr::read_volatile((MMIO + FH_TCSR_BUF_STS_CMD) as *const u32) as usize);
        diag_write_str(" hbus="); diag_write_hex(core::ptr::read_volatile((MMIO + 0x60) as *const u32) as usize); diag_write_str("\n");

        diag_write_str("[WIFI] WF-RX DESCRIPTORS");
        let mut i = 0usize;
        while i < FH_RX_RBD_COUNT {
            diag_write_str(" ["); diag_write_usize(i); diag_write_str("]=");
            diag_write_hex(RX_RBD.0[i] as usize); diag_write_str(":");
            diag_write_hex(core::ptr::read_volatile(&RX_BUFFERS.0[i][0] as *const u8 as *const u32) as usize);
            i += 1;
        }
        diag_write_str("\n");

        diag_write_str("[WIFI] WF-SCAN start="); diag_write_usize(scan_start_count() as usize);
        diag_write_str(" results="); diag_write_usize(scan_results_count() as usize);
        diag_write_str(" complete="); diag_write_usize(scan_complete_count() as usize);
        diag_write_str(" rx_irq="); diag_write_usize(RX_IRQ_COUNT as usize);
        diag_write_str(" alive="); diag_write_str(if ALIVE_SEEN { "SEEN" } else { "NOT-SEEN" }); diag_write_str("\n");

        if scan_complete_count() == 0 && scan_start_count() == 0 &&
           scan_results_count() == 0 && RX_READ == (hw_rptr as usize & (FH_RX_RBD_COUNT - 1)) {
            diag_write_str("[WIFI] WF-RESULT=RX_RING_NO_NEW_ENTRIES\n");
            diag_write_str("[WIFI] WF-CAUSE=SCAN_COMMAND_SUBMITTED_BUT_RX_PRODUCER_DID_NOT_ADVANCE\n");
            diag_write_str("[WIFI] WF-NEXT=VERIFY_RX_RING_PROGRAMMING_FH_CHANNEL_ENABLE_AND_BUFFER_ADDRESSING\n");
        } else if scan_complete_count() == 0 {
            diag_write_str("[WIFI] WF-RESULT=SCAN_NO_COMPLETE_NOTIFICATION\n");
            diag_write_str("[WIFI] WF-NEXT=CONTINUE_RX_NOTIFICATION_ANALYSIS\n");
        } else {
            diag_write_str("[WIFI] WF-RESULT=SCAN_COMPLETE_NOTIFICATION_SEEN\n");
        }
    }
}

fn prph_write(addr: u32, val: u32) {
    unsafe {
        core::ptr::write_volatile((MMIO + 0x444) as *mut u32, (addr & 0x000F_FFFF) | (3 << 24));
        core::ptr::write_volatile((MMIO + 0x44C) as *mut u32, val);
    }
}

fn prph_read(addr: u32) -> u32 {
    unsafe {
        core::ptr::write_volatile((MMIO + 0x448) as *mut u32, (addr & 0x000F_FFFF) | (3 << 24));
        core::ptr::read_volatile((MMIO + 0x450) as *const u32)
    }
}

/// Initialize the legacy gen1/gen2 scheduler and command queue (#4).
/// This is the transport prerequisite for host commands such as REPLY_SCAN_CMD.
pub fn init_command_queue() -> bool {
    unsafe {
        if !ALIVE_SEEN || !MMIO_MAPPED || MMIO == 0 { return false; }
        let tfd_base = (&CMD_TFD_QUEUE.0 as *const u8) as u64;
        let scd_sram = prph_read(SCD_SRAM_BASE_ADDR);
        if scd_sram == 0 || scd_sram == 0xFFFF_FFFF { 
            diag_write_str("[WIFI] CMDQ SCD_SRAM=INVALID\\n");
            return false;
        }
        core::ptr::write_bytes(CMD_TFD_QUEUE.0.as_mut_ptr(), 0, CMD_TFD_QUEUE.0.len());
        core::ptr::write_bytes(SCD_BC_TABLE.0.as_mut_ptr(), 0, SCD_BC_TABLE.0.len());
        core::ptr::write_volatile((MMIO + FH_MEM_CBBC_CMD) as *mut u32, (tfd_base >> 8) as u32);

        // Initialize the scheduler's SRAM/DRAM translation state before
        // activating queue #4.  The DVM transport expects the DRAM base and
        // queue read pointer to be valid even for the first host command.
        prph_write(SCD_CHAINEXT_EN, 0);
        prph_write(SCD_GP_CTRL, SCD_GP_CTRL_ENABLE_31_QUEUES);
        prph_write(SCD_EN_CTRL, 0);
        prph_write(SCD_QUEUECHAIN_SEL, 0);
        prph_write(SCD_TXFACT, 1u32 << IWL_CMD_FIFO_NUM);
        // SCD_DRAM_BASE_ADDR is the scheduler byte-count table base, not the
        // TFD ring base. Gen1/2 iwlwifi uses 320 u16 entries per queue and
        // indexes the table by queue number.
        let bc_base = (&SCD_BC_TABLE.0 as *const u16) as u64;
        prph_write(SCD_DRAM_BASE_ADDR, (bc_base >> 10) as u32);

        // Clear queue #4 read/write pointers and status before activation.
        prph_write(SCD_QUEUE_WRPTR, 0);
        prph_write(SCD_QUEUE_RDPTR, 0);
        prph_write(SCD_QUEUE_STATUS_BITS, 0);

        // Queue #4 is the default DVM command queue when PAN is disabled.
        // FIFO 7 is the command FIFO. Active + write-status-limit are required.
        prph_write(SCD_QUEUE_STATUS_BITS,
            SCD_QUEUE_STATUS_MASK | SCD_QUEUE_ACTIVE | (IWL_CMD_FIFO_NUM << 0) | SCD_QUEUE_WSL);

        // Context: window size and frame limit, matching the DVM scheduler.
        let ctx = scd_sram + SCD_QUEUE_CTX;
        core::ptr::write_volatile((MMIO + 0x410) as *mut u32, ctx);
        core::ptr::write_volatile((MMIO + 0x418) as *mut u32, 0);
        core::ptr::write_volatile((MMIO + 0x410) as *mut u32, ctx + 4);
        core::ptr::write_volatile((MMIO + 0x418) as *mut u32,
            (SCD_WIN_SIZE & 0x7F) | ((SCD_FRAME_LIMIT & 0x7F) << 16));

        core::ptr::write_volatile((MMIO + FH_TCSR_CONFIG_CMD) as *mut u32, 0);
        core::ptr::write_volatile((MMIO + FH_TCSR_BUF_STS_CMD) as *mut u32, 0);
        core::ptr::write_volatile((MMIO + FH_TCSR_CONFIG_CMD) as *mut u32,
            FH_TCSR_TX_CMD_DMA_ENABLE | FH_TCSR_TX_CMD_CREDIT_ENABLE |
            FH_TCSR_TX_CMD_CIRQ_HOST_ENDTFD);
        prph_write(SCD_QUEUE_WRPTR, 0);
        // Seed the command queue byte-count table entry as an invalid/empty
        // descriptor. send_command() replaces it with the actual frame size.
        CMD_WRITE_PTR = 0;
        CMD_SEQ = 0;
        CMD_QUEUE_READY = true;
        diag_write_str("[WIFI] CMDQ READY QUEUE=4 FIFO=7 SCD5000=YES STATUS=");\n        diag_write_hex(prph_read(SCD_QUEUE_STATUS_BITS) as usize);\n        diag_write_str(" TFD=256\\n");
        true
    }
}

fn cmd_tfd_set(buf: *mut u8, dma: u64, len: usize) {
    unsafe {
        core::ptr::write_bytes(buf, 0, FH_TFD_SIZE);
        *buf.add(3) = 1;
        core::ptr::write_volatile(buf.add(4) as *mut u32, dma as u32);
        core::ptr::write_volatile(buf.add(8) as *mut u16,
            ((len as u16) << 4) | (((dma >> 32) as u16) & 0xF));
    }
}

/// Send one legacy DVM host command through command queue #4.
/// The caller supplies the complete command header+payload.
pub fn send_command(cmd: u8, payload: &[u8]) -> bool {
    unsafe {
        if !CMD_QUEUE_READY || payload.len() > 4096 { return false; }
        let slot = CMD_WRITE_PTR & (FH_TFD_CMD_SLOTS - 1);
        let frame = (&mut FW_DMA_BUF.0[0]) as *mut u8;
        core::ptr::write_bytes(frame, 0, 512);
        // DVM command header: cmd, flags, little-endian sequence.
        // Sequence encodes command queue in bits 8..12 and TFD index in bits 0..7.
        *frame.add(0) = cmd;
        *frame.add(1) = 0;
        let seq = ((IWL_DEFAULT_CMD_QUEUE_NUM as u16) << 8) | (slot as u16);
        *frame.add(2) = seq as u8;
        *frame.add(3) = (seq >> 8) as u8;
        let mut i = 0usize;
        while i < payload.len() {
            *frame.add(4 + i) = payload[i];
            i += 1;
        }
        let len = 4 + payload.len();
        let tfd = (&mut CMD_TFD_QUEUE.0[slot * FH_TFD_SIZE]) as *mut u8;
        cmd_tfd_set(tfd, (&FW_DMA_BUF.0[0] as *const u8) as u64, len);
        // Gen1/2 DVM scheduler byte-count tables are expressed in DWORDs.
        // Keep the first 64 entries duplicated as required by the hardware.
        let bc = ((len + 3) / 4) as u16;
        let bc_off = IWL_DEFAULT_CMD_QUEUE_NUM * SCD_QUEUE_BC_SIZE + slot;
        SCD_BC_TABLE.0[bc_off] = bc;
        if slot < 64 {
            SCD_BC_TABLE.0[IWL_DEFAULT_CMD_QUEUE_NUM * SCD_QUEUE_BC_SIZE + FH_TFD_CMD_SLOTS + slot] = bc;
        }
        let next = (slot + 1) & (FH_TFD_CMD_SLOTS - 1);
        // The DVM PCIe transport publishes the host TFD through
        // HBUS_TARG_WRPTR (MMIO + 0x60). SCD_QUEUE_WRPTR is scheduler state,
        // not the host doorbell used to submit the command.
        core::ptr::write_volatile((MMIO + 0x60) as *mut u32,
            (next as u32 & 0xFF) | ((IWL_DEFAULT_CMD_QUEUE_NUM as u32) << 8));
        CMD_WRITE_PTR = next;
        CMD_SEQ = CMD_SEQ.wrapping_add(1);
        diag_write_str("[WIFI] CMD TX id=");
        diag_write_hex(cmd as usize);
        diag_write_str(" len=");
        diag_write_usize(len);
        diag_write_str(" Q=4 BC_DW=");
        diag_write_hex(bc as usize);
        diag_write_str("\\n");
        true
    }
}

pub fn command_queue_ready() -> bool { unsafe { CMD_QUEUE_READY } }


const REPLY_SCAN_CMD: u8 = 0x80;
const SCAN_START_NOTIFICATION: u8 = 0x82;
const SCAN_RESULTS_NOTIFICATION: u8 = 0x83;
const SCAN_COMPLETE_NOTIFICATION: u8 = 0x84;
const RXON_FLG_BAND_24G: u32 = 1 << 0;
const IWL_GOOD_CRC_TH_DEFAULT: u16 = 1;
const IWL_SCAN_CHANNEL_ACTIVE: u32 = 1;
const IWL_SCAN_CHANNEL_COUNT_24G: usize = 11;

fn put_le16(buf: &mut [u8], off: usize, v: u16) {
    buf[off] = v as u8;
    buf[off + 1] = (v >> 8) as u8;
}
fn put_le32(buf: &mut [u8], off: usize, v: u32) {
    buf[off] = v as u8;
    buf[off + 1] = (v >> 8) as u8;
    buf[off + 2] = (v >> 16) as u8;
    buf[off + 3] = (v >> 24) as u8;
}

/// First real DVM scan request: passive 2.4 GHz sweep over channels 1..11.
/// This intentionally omits probe TX until the command transport is proven.
pub fn scan_24ghz() -> bool {
    unsafe {
        if !CMD_QUEUE_READY || !ALIVE_SEEN { return false; }
        SCAN_NOTIFICATION_SEEN = false;

        // Legacy DVM iwl_scan_cmd for the 2030 firmware:
        // fixed header 28 + zeroed iwl_tx_cmd 52 + 20 SSID IEs (34 each),
        // followed by iwl_scan_channel entries of 12 bytes each.
        let fixed = 28usize + 52usize + 20usize * 34usize;
        let channels = IWL_SCAN_CHANNEL_COUNT_24G * 12usize;
        let total = fixed + channels;
        let mut p = [0u8; 1024];

        // scan.len excludes the common 4-byte iwl_cmd_header.
        put_le16(&mut p, 0, total as u16);
        p[2] = 0; // scan_flags
        p[3] = IWL_SCAN_CHANNEL_COUNT_24G as u8;
        put_le16(&mut p, 4, 0); // quiet_time
        put_le16(&mut p, 6, 0); // quiet_plcp_th
        put_le16(&mut p, 8, IWL_GOOD_CRC_TH_DEFAULT);
        put_le16(&mut p, 10, 0x0007); // RX chain: allow A/B/C
        put_le32(&mut p, 12, 0); // max_out_time: unassociated
        put_le32(&mut p, 16, 0); // suspend_time
        put_le32(&mut p, 20, RXON_FLG_BAND_24G);
        put_le32(&mut p, 24, 0); // filter_flags

        // TX header remains zero for this passive scan.
        // Direct SSID table remains zero.
        let mut off = fixed;
        let mut ch = 1u16;
        while ch <= 11 {
            // iwl_scan_channel:
            // type:u32, channel:u16, tx_gain:u8, dsp_atten:u8,
            // active_dwell:u16, passive_dwell:u16.
            put_le32(&mut p, off, 0); // passive; no directed SSID mask
            put_le16(&mut p, off + 4, ch);
            p[off + 6] = 0; // automatic TX gain
            p[off + 7] = 0; // automatic DSP attenuation
            put_le16(&mut p, off + 8, 0);   // active dwell
            put_le16(&mut p, off + 10, 100); // passive dwell, TU
            off += 12;
            ch += 1;
        }

        diag_write_str("[WIFI] SCAN24 CMD channels=11\n");
        if !send_command(REPLY_SCAN_CMD, &p[..total]) {
            diag_write_str("[WIFI] SCAN24 CMD=FAILED\n");
            return false;
        }
        diag_write_str("[WIFI] SCAN24 CMD=SUBMITTED\n");
        crate::desktop::terminal_write("[WIFI] SCAN24 CMD=SUBMITTED\n");
        // Do one non-blocking RX poll only. If the firmware responds later,
        // the next WF invocation will observe it. This keeps the shell alive
        // while we validate command transport on real hardware.
        irq_handler();
        diag_write_str("[WIFI] SCAN24 POST CSR_INT=");
        diag_write_hex(core::ptr::read_volatile((MMIO + CSR_INT) as *const u32) as usize);
        diag_write_str(" FH_INT=");
        diag_write_hex(core::ptr::read_volatile((MMIO + CSR_FH_INT_STATUS) as *const u32) as usize);
        diag_write_str(" SCD_WRPTR=");
        diag_write_hex(prph_read(SCD_QUEUE_WRPTR) as usize);
        diag_write_str(" SCD_RDPTR=");
        diag_write_hex(prph_read(SCD_QUEUE_RDPTR) as usize);
        diag_write_str(" SCD_STATUS=");
        diag_write_hex(prph_read(SCD_QUEUE_STATUS_BITS) as usize);
        diag_write_str(" SCD_DRAM=");
        diag_write_hex(prph_read(SCD_DRAM_BASE_ADDR) as usize);
        diag_write_str(" CBBC=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_MEM_CBBC_CMD) as *const u32) as usize);
        diag_write_str(" TCSR_CFG=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_TCSR_CONFIG_CMD) as *const u32) as usize);
        diag_write_str(" TCSR_STS=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_TCSR_BUF_STS_CMD) as *const u32) as usize);
        diag_write_str(" HBUS_WRPTR=");
        diag_write_hex(core::ptr::read_volatile((MMIO + 0x60) as *const u32) as usize);
        diag_write_str("\n");

        let mut line = [0u8; 192];
        let mut n = 0usize;
        fn push(dst: &mut [u8], n: &mut usize, s: &[u8]) {
            let mut i = 0usize;
            while i < s.len() && *n < dst.len() { dst[*n] = s[i]; *n += 1; i += 1; }
        }
        fn hex(dst: &mut [u8], n: &mut usize, mut v: u32) {
            let mut tmp = [0u8; 8];
            let mut i = 8usize;
            while i > 0 {
                i -= 1;
                let x = (v & 0xF) as u8;
                tmp[i] = if x < 10 { b'0' + x } else { b'a' + x - 10 };
                v >>= 4;
            }
            push(dst, n, &tmp);
        }
        push(&mut line, &mut n, b"[WIFI] TRANSPORT SCD_STATUS=");
        hex(&mut line, &mut n, prph_read(SCD_QUEUE_STATUS_BITS));
        push(&mut line, &mut n, b" SCD_DRAM=");
        hex(&mut line, &mut n, prph_read(SCD_DRAM_BASE_ADDR));
        push(&mut line, &mut n, b" CBBC=");
        hex(&mut line, &mut n, core::ptr::read_volatile((MMIO + FH_MEM_CBBC_CMD) as *const u32));
        push(&mut line, &mut n, b" TCSR_CFG=");
        hex(&mut line, &mut n, core::ptr::read_volatile((MMIO + FH_TCSR_CONFIG_CMD) as *const u32));
        push(&mut line, &mut n, b" TCSR_STS=");
        hex(&mut line, &mut n, core::ptr::read_volatile((MMIO + FH_TCSR_BUF_STS_CMD) as *const u32));
        push(&mut line, &mut n, b" HBUS_WRPTR=");
        hex(&mut line, &mut n, core::ptr::read_volatile((MMIO + 0x60) as *const u32));
        push(&mut line, &mut n, b"\n");
        crate::desktop::terminal_write(core::str::from_utf8(&line[..n]).unwrap_or("[WIFI] TRANSPORT SNAPSHOT ERROR\\n"));

        let mut polls = 0usize;
        while polls < 200_000 {
            let pending = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
            if pending != 0 {
                irq_handler();
                if scan_complete_count() != 0 { break; }
            }
            core::hint::spin_loop();
            polls += 1;
        }
        diag_write_str("[WIFI] SCAN RX start=");
        diag_write_usize(scan_start_count() as usize);
        diag_write_str(" results=");
        diag_write_usize(scan_results_count() as usize);
        diag_write_str(" complete=");
        diag_write_usize(scan_complete_count() as usize);
        diag_write_str(" polls=");
        diag_write_usize(polls);
        diag_write_str("\n");
        if scan_complete_count() == 0 {
            diag_write_str("[WIFI] SCAN TIMEOUT/NO-COMPLETE-NOTIFICATION\n");
            crate::desktop::terminal_write("[WIFI] SCAN TIMEOUT/NO-COMPLETE-NOTIFICATION\n");
        }
        true
    }
}



unsafe fn init_rx_queue() -> bool {
    if !MMIO_MAPPED || MMIO == 0 { return false; }
    RX_READ = 0;
    RX_IRQ_COUNT = 0;
    ALIVE_SEEN = false;
    ALIVE_VALID = 0;
    ALIVE_SUBTYPE = 0;
    let rbd_base = (&RX_RBD.0 as *const u32) as u64;
    let status_base = (&RX_STATUS.0 as *const u32) as u64;
    let mut i = 0usize;
    while i < FH_RX_RBD_COUNT {
        let buf = (&RX_BUFFERS.0[i][0] as *const u8) as u64;
        RX_RBD.0[i] = (buf >> 8) as u32;
        RX_BUFFERS.0[i][0] = 0;
        i += 1;
    }
    RX_STATUS.0[0] = 0;
    RX_STATUS.0[1] = 0;\n    RX_STATUS.0[2] = 0;
    core::ptr::write_volatile((MMIO + FH_RCSR_CHNL0_CONFIG) as *mut u32, 0);
    core::ptr::write_volatile((MMIO + FH_RSCSR_RBDCB_WPTR) as *mut u32, 0);
    core::ptr::write_volatile((MMIO + FH_RCSR_CHNL0_FLUSH_RB_REQ) as *mut u32, 0);
    core::ptr::write_volatile((MMIO + FH_RSCSR_RDPTR) as *mut u32, 0);
    core::ptr::write_volatile((MMIO + FH_RSCSR_RBDCB_BASE) as *mut u32, (rbd_base >> 8) as u32);
    core::ptr::write_volatile((MMIO + FH_RSCSR_STTS_WPTR) as *mut u32, (status_base >> 4) as u32);
    core::ptr::write_volatile((MMIO + FH_RSCSR_RBDCB_WPTR) as *mut u32, 248);
    core::ptr::write_volatile((MMIO + CSR_FH_INT_STATUS) as *mut u32, CSR_FH_INT_RX_MASK);
    core::ptr::write_volatile((MMIO + CSR_INT) as *mut u32, CSR_INT_BIT_FH_RX | CSR_INT_BIT_ALIVE);
    core::ptr::write_volatile((MMIO + CSR_INT_MASK) as *mut u32, CSR_INT_BIT_FH_RX | CSR_INT_BIT_ALIVE);
    // 256 RBDs, 4 KiB buffers, host IRQ destination, RB timeout 0x11, DMA enabled.
    let cfg = 0x8000_0000u32 | (8u32 << 20) | 0x0000_1000 | (0x11u32 << 4) | 0x0000_0004;
    core::ptr::write_volatile((MMIO + FH_RCSR_CHNL0_CONFIG) as *mut u32, cfg);
    RX_READY = true;
    diag_write_str("[WIFI] RX-RING READY RBD=32 BUF=4K IRQ=HOST\n");
    true
}

pub unsafe fn irq_handler() {
    if !MMIO_MAPPED || MMIO == 0 { return; }
    let inta = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
    if inta == 0 { return; }
    RX_IRQ_COUNT = RX_IRQ_COUNT.wrapping_add(1);
    let fh = core::ptr::read_volatile((MMIO + CSR_FH_INT_STATUS) as *const u32);
    if (inta & CSR_INT_BIT_FH_RX) != 0 || (fh & CSR_FH_INT_RX_MASK) != 0 {
        // DVM FH writes closed_rb_num into the first RX status word. This is\n        // the producer index; FH_RSCSR_RDPTR is not the notification producer.\n        let status_closed_rb = core::ptr::read_volatile((&RX_STATUS.0[0]) as *const u32) as usize;\n        let hw = status_closed_rb & (FH_RX_RBD_COUNT - 1);
        let mut same_rx_streak = 0usize;
        let mut last_cmd = 0u8;
        let mut last_subtype = 0u8;
        let mut rx_items = 0usize;
        diag_write_str("[WIFI] RX-STATE enter read=");
        diag_write_usize(RX_READ);
        diag_write_str(" hw_rptr=");
        diag_write_usize(hw);
        diag_write_str(" cb_wptr=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSCSR_RBDCB_WPTR) as *const u32) as usize);
        diag_write_str(" stts_wptr=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSCSR_STTS_WPTR) as *const u32) as usize);
        diag_write_str(" rx_status=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSSR_RX_STATUS) as *const u32) as usize);
        diag_write_str(" inta=");
        diag_write_hex(inta as usize);
        diag_write_str(" fh=");
        diag_write_hex(fh as usize);
        diag_write_str(" rbd_base=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSCSR_RBDCB_BASE) as *const u32) as usize);
        diag_write_str(" rbd_addr=");
        diag_write_hex((&RX_RBD.0[RX_READ] as *const u32) as usize);
        diag_write_str(" buf_addr=");
        diag_write_hex((&RX_BUFFERS.0[RX_READ][0] as *const u8) as usize);
        diag_write_str("\n");
        while RX_READ != hw {
            let p = &RX_BUFFERS.0[RX_READ][0] as *const u8;
            let len_flags = core::ptr::read_volatile(p as *const u32);
            if len_flags != FH_RSCSR_FRAME_INVALID {
                let len = (len_flags & 0x3FFF) as usize;
                if len >= 8 && len <= FH_RX_BUF_SIZE - 4 {
                    let cmd = core::ptr::read_volatile(p.add(4));
                    let subtype_probe = if cmd == 1 {
                        core::ptr::read_volatile(p.add(4 + 4 + 13))
                    } else { 0 };
                    if cmd == last_cmd && (cmd != 1 || subtype_probe == last_subtype) {
                        same_rx_streak = same_rx_streak.saturating_add(1);
                    } else {
                        same_rx_streak = 1;
                        last_cmd = cmd;
                        last_subtype = subtype_probe;
                    }
                    rx_items = rx_items.saturating_add(1);
                    diag_write_str("[WIFI] RX-ITEM idx=");
                    diag_write_usize(RX_READ);
                    diag_write_str(" len=");
                    diag_write_usize(len);
                    diag_write_str(" cmd=");
                    diag_write_hex(cmd as usize);
                    if cmd == 1 {
                        diag_write_str(" subtype=");
                        diag_write_usize(subtype_probe as usize);
                    }
                    diag_write_str(" repeat=");
                    diag_write_usize(same_rx_streak);
                    diag_write_str(" raw=");
                    let mut j = 0usize;
                    while j < 16 && j < len {
                        diag_write_hex(core::ptr::read_volatile(p.add(j)) as usize);
                        j += 1;
                    }
                    diag_write_str("\n");
                    if same_rx_streak > 50 {
                        diag_write_str("[WIFI] IRQ STORM SUSPECTED same RX cmd/subtype >50\n");
                        break;
                    }
                    if cmd == 1 {
                        let subtype = core::ptr::read_volatile(p.add(4 + 4 + 13));
                        let valid = core::ptr::read_volatile(p.add(4 + 4 + 28));
                        ALIVE_SEEN = true;
                        ALIVE_SUBTYPE = subtype;
                        ALIVE_VALID = valid as u32;
                        diag_write_str("[WIFI] ALIVE cmd=1 subtype=");
                        diag_write_usize(subtype as usize);
                        diag_write_str(" valid=");
                        diag_write_hex(valid as usize);
                        diag_write_str(if valid == 1 { " RESULT=VALID\n" } else { " RESULT=INVALID\n" });
                    } else {
                        if cmd == SCAN_START_NOTIFICATION ||
                           cmd == SCAN_RESULTS_NOTIFICATION ||
                           cmd == SCAN_COMPLETE_NOTIFICATION {
                            SCAN_NOTIFICATION_SEEN = true;
                        }
                        diag_write_str("[WIFI] RX cmd=");
                        diag_write_hex(cmd as usize);
                        diag_write_str(" len=");
                        diag_write_usize(len);
                        let payload = p.add(8);
                        if cmd == SCAN_START_NOTIFICATION && len >= 24 {
                            let channel = core::ptr::read_volatile(payload.add(12));
                            let band = core::ptr::read_volatile(payload.add(13));
                            let status = core::ptr::read_volatile(payload.add(16) as *const u32);
                            SCAN_START_COUNT = SCAN_START_COUNT.wrapping_add(1);
                            diag_write_str(" TYPE=SCAN_START ch=");
                            diag_write_usize(channel as usize);
                            diag_write_str(" band=");
                            diag_write_usize(band as usize);
                            diag_write_str(" status=");
                            diag_write_hex(status as usize);
                            diag_write_str("\n");
                        } else if cmd == SCAN_RESULTS_NOTIFICATION && len >= 20 {
                            let channel = core::ptr::read_volatile(payload);
                            let band = core::ptr::read_volatile(payload.add(1));
                            let probe = core::ptr::read_volatile(payload.add(2));
                            let not_sent = core::ptr::read_volatile(payload.add(3));
                            let stats = core::ptr::read_volatile(payload.add(12) as *const u32);
                            SCAN_RESULTS_COUNT = SCAN_RESULTS_COUNT.wrapping_add(1);
                            diag_write_str(" TYPE=SCAN_RESULTS ch=");
                            diag_write_usize(channel as usize);
                            diag_write_str(" band=");
                            diag_write_usize(band as usize);
                            diag_write_str(" probe=");
                            diag_write_hex(probe as usize);
                            diag_write_str(" not_sent=");
                            diag_write_usize(not_sent as usize);
                            diag_write_str(" good_crc=");
                            diag_write_usize(stats as usize);
                            diag_write_str("\n");
                        } else if cmd == SCAN_COMPLETE_NOTIFICATION && len >= 16 {
                            let channels = core::ptr::read_volatile(payload);
                            let status = core::ptr::read_volatile(payload.add(1));
                            let bt = core::ptr::read_volatile(payload.add(2));
                            let last = core::ptr::read_volatile(payload.add(3));
                            SCAN_COMPLETE_COUNT = SCAN_COMPLETE_COUNT.wrapping_add(1);
                            SCAN_COMPLETE_CHANNELS = channels;
                            SCAN_COMPLETE_STATUS = status;
                            SCAN_COMPLETE_LAST_CHANNEL = last;
                            diag_write_str(" TYPE=SCAN_COMPLETE channels=");
                            diag_write_usize(channels as usize);
                            diag_write_str(" status=");
                            diag_write_hex(status as usize);
                            diag_write_str(" bt=");
                            diag_write_usize(bt as usize);
                            diag_write_str(" last=");
                            diag_write_usize(last as usize);
                            diag_write_str("\n");
                        } else {
                            diag_write_str(" TYPE=OTHER RAW=");
                            let mut j = 0usize;
                            while j < 12 && (8 + j) < len {
                                diag_write_hex(core::ptr::read_volatile(p.add(8 + j)) as usize);
                                j += 1;
                            }
                            diag_write_str("\n");
                        }
                    }
                }
            }
            core::ptr::write_volatile(RX_BUFFERS.0[RX_READ].as_mut_ptr() as *mut u32, FH_RSCSR_FRAME_INVALID);
            RX_READ = (RX_READ + 1) & (FH_RX_RBD_COUNT - 1);
        }
        diag_write_str("[WIFI] RX-STATE exit items=");
        diag_write_usize(rx_items);
        diag_write_str(" read=");
        diag_write_usize(RX_READ);
        diag_write_str(" hw_rptr=");
        diag_write_usize(hw);
        diag_write_str(" cb_wptr=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSCSR_RBDCB_WPTR) as *const u32) as usize);
        diag_write_str(" stts_wptr=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSCSR_STTS_WPTR) as *const u32) as usize);
        diag_write_str(" rx_status=");
        diag_write_hex(core::ptr::read_volatile((MMIO + FH_RSSR_RX_STATUS) as *const u32) as usize);
        diag_write_str("\n");
        let new_cb_wptr = RX_READ as u32 + FH_RX_RBD_COUNT as u32 - 1;
        core::ptr::write_volatile((MMIO + FH_RSCSR_RBDCB_WPTR) as *mut u32, new_cb_wptr);
        core::ptr::write_volatile((MMIO + CSR_FH_INT_STATUS) as *mut u32, CSR_FH_INT_RX_MASK);
        let ack_fh = core::ptr::read_volatile((MMIO + CSR_FH_INT_STATUS) as *const u32);
        let ack_int = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
        diag_write_str("[WIFI] RX-ACK cb_wptr=");
        diag_write_hex(new_cb_wptr as usize);
        diag_write_str(" fh_after=");
        diag_write_hex(ack_fh as usize);
        diag_write_str(" inta_after=");
        diag_write_hex(ack_int as usize);
        diag_write_str("\n");
    }
    core::ptr::write_volatile((MMIO + CSR_INT) as *mut u32, inta);
    let int_after = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
    diag_write_str("[WIFI] RX-ACK CSR_INT final=");
    diag_write_hex(int_after as usize);
    diag_write_str("\n");
}


fn fw_le32(b: &[u8], off: usize) -> u32 {
    (b[off] as u32) | ((b[off + 1] as u32) << 8) |
    ((b[off + 2] as u32) << 16) | ((b[off + 3] as u32) << 24)
}

pub fn load_firmware() -> bool {
    unsafe {
        FW_ATTEMPTED = true;
        FW_LOADED = false;
        if !ACTIVATE_OK || MMIO == 0 || !MMIO_MAPPED {
            diag_write_str("[WIFI] FW=NOT-ATTEMPTED activation prerequisite missing\n");
            return false;
        }
        if IWL2030_FW.len() < 88 {
            diag_write_str("[WIFI] FW=INVALID file too small\n");
            return false;
        }

        let magic = fw_le32(IWL2030_FW, 4);
        const IWL_TLV_UCODE_MAGIC: u32 = 0x0A4C5749;
        if fw_le32(IWL2030_FW, 0) != 0 || magic != IWL_TLV_UCODE_MAGIC {
            diag_write_str("[WIFI] FW=UNSUPPORTED_FORMAT MAGIC=");
            diag_write_hex(magic as usize);
            diag_write_str("\n");
            return false;
        }

        let ver = fw_le32(IWL2030_FW, 72);
        let build = fw_le32(IWL2030_FW, 76);
        let api = (ver >> 8) & 0xFF;
        FW_VER = ver;
        FW_INST_SIZE = 0;
        FW_DATA_SIZE = 0;
        diag_write_str("[WIFI] FW FORMAT=TLV MAGIC=");
        diag_write_hex(magic as usize);
        diag_write_str(" VER=");
        diag_write_hex(ver as usize);
        diag_write_str(" API=");
        diag_write_usize(api as usize);
        diag_write_str(" BUILD=");
        diag_write_usize(build as usize);
        diag_write_str("\n");

        let dma_base = (&FW_DMA_BUF.0 as *const u8) as u64;
        diag_write_str("[WIFI] FW DMA_BUFFER=");
        diag_write_hex(dma_base as usize);
        diag_write_str("\n");

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
                        diag_write_str("[WIFI] FW SERVICE-DMA=TIMEOUT dst=");
                        diag_write_hex((dst + off as u32) as usize);
                        diag_write_str("\n");
                        return false;
                    }
                    off += chunk;
                }
                diag_write_str("[WIFI] FW SERVICE-DMA section dst=");
                diag_write_hex(dst as usize);
                diag_write_str(" bytes=");
                diag_write_usize(tlv_len);
                diag_write_str(" OK\n");
            }

            pos = next;
        }

        if !inst_seen || !data_seen {
            diag_write_str("[WIFI] FW TLV=RUNTIME_SECTIONS_MISSING\n");
            return false;
        }

        FW_INST_SIZE = inst_size as u32;
        FW_DATA_SIZE = data_size as u32;
        FW_LOADED = true;
        NEEDS_FW = false;
        diag_write_str("[WIFI] FW LOAD=OK via Intel FH service DMA\n");
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
            diag_write_str("[WIFI] FW EXEC=NOT-ATTEMPTED prerequisite missing\n");
            return false;
        }
        FW_EXEC_ATTEMPTED = true;
        if !init_rx_queue() {
            diag_write_str("[WIFI] RX-RING INIT=FAILED\n");
            return false;
        }
        // Do not execute STI here. Aether does not yet have a complete
        // legacy PIC dispatch path for every IRQ. Enabling CPU interrupts at
        // this point can vector an unrelated IRQ to an uninstalled IDT gate
        // and triple-fault/reboot the machine. Keep WiFi bring-up polled.
        diag_write_str("[WIFI] IRQ=POLLED (CPU IF unchanged) line=");
        diag_write_usize(IRQ_LINE as usize);
        diag_write_str("\n");
        let gp1_clr = (MMIO + 0x05C) as *mut u32;
        let csr = (MMIO + CSR_RESET) as *mut u32;
        core::ptr::write_volatile(gp1_clr, 0x0000_0006);
        core::ptr::write_volatile(csr, 0);
        let after = core::ptr::read_volatile(csr);
        FW_EXEC_STARTED = after == 0;
        diag_write_str("[WIFI] FW EXEC RELEASE_RESET=0 READBACK=");
        diag_write_hex(after as usize);
        diag_write_str(" RESULT=");
        diag_write_str(if FW_EXEC_STARTED { "STARTED" } else { "FAILED" });
        diag_write_str(" ALIVE=NOT-YET RX/IRQ=POLLING\n");

        // Poll the device instead of enabling global CPU interrupts. This
        // lets us validate the RX/ALIVE path without depending on the
        // unfinished common legacy PIC dispatcher.
        if !FW_EXEC_STARTED { return false; }
        let mut n = 0usize;
        while n < 5_000_000 {
            let inta = core::ptr::read_volatile((MMIO + CSR_INT) as *const u32);
            if (inta & (CSR_INT_BIT_FH_RX | CSR_INT_BIT_ALIVE)) != 0 {
                irq_handler();
                if ALIVE_SEEN { break; }
            }
            core::hint::spin_loop();
            n += 1;
        }
        diag_write_str("[WIFI] ALIVE=");
        diag_write_str(if ALIVE_SEEN { "SEEN" } else { "NOT-SEEN" });
        diag_write_str(" IRQ_COUNT=");
        diag_write_usize(RX_IRQ_COUNT as usize);
        diag_write_str("\n");
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
            diag_write_str("[WIFI] ACTIVATE=NOT-ATTEMPTED reset prerequisite missing\n");
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
        diag_write_str("[WIFI] ACTIVATE INIT_DONE=SET BEFORE=");
        diag_write_hex(before as usize);
        diag_write_str(" AFTER=");
        diag_write_hex(ready as usize);
        diag_write_str(" RESULT=");
        diag_write_str(if ACTIVATE_OK { "MAC_CLOCK_READY" } else { "TIMEOUT" });
        diag_write_str("\n");
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
            diag_write_str("[WIFI] RESET=NOT-ATTEMPTED prerequisite missing\n");
            return false;
        }

        let csr = (MMIO + CSR_RESET) as *mut u32;
        let before = core::ptr::read_volatile(csr);
        RESET_BEFORE = before;
        RESET_ATTEMPTED = true;
        diag_write_str("[WIFI] RESET path=INTEL_CSR_RESET SW_RESET=0x80 BEFORE=");
        diag_write_hex(before as usize);
        diag_write_str("\n");

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
        diag_write_str("[WIFI] RESET AFTER=");
        diag_write_hex(after as usize);
        diag_write_str(" RESULT=");
        diag_write_str(if RESET_OK { "READABLE" } else { "MMIO-FAIL" });
        diag_write_str("\n");
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
            diag_write_str("[WIFI] CAPS=NO-DEVICE\n");
            return;
        }

        let p = pci_r32(BUS, DEV, FUNC, 0x34);
        let mut ptr = (p & 0xFF) as u8;
        CAP_PTR = ptr;
        let mut count = 0usize;

        diag_write_str("[WIFI] CAPS PTR=");
        diag_write_hex(ptr as usize);
        diag_write_str("\n");

        while ptr >= 0x40 && count < 48 {
            let off = ptr & 0xFC;
            let word = pci_r32(BUS, DEV, FUNC, off);
            let cap_id = (word & 0xFF) as u8;
            let next = ((word >> 8) & 0xFF) as u8;
            diag_write_str("[WIFI] CAP off=");
            diag_write_hex(ptr as usize);
            diag_write_str(" ID=");
            diag_write_hex(cap_id as usize);
            diag_write_str(" NEXT=");
            diag_write_hex(next as usize);
            diag_write_str("\n");

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
        diag_write_str("[WIFI] CAPS PM=");
        diag_write_str(if CAP_PM { "YES" } else { "NO" });
        diag_write_str(" MSI=");
        diag_write_str(if CAP_MSI { "YES" } else { "NO" });
        diag_write_str(" MSIX=");
        diag_write_str(if CAP_MSIX { "YES" } else { "NO" });
        diag_write_str(" READ=YES\n");
    }
}

/// Read-only PCI prerequisite snapshot for the Intel 2230 bring-up stage.
/// No device reset, firmware load, interrupt enable, DMA, TX/RX, or association.
pub fn probe_prerequisites() {
    unsafe {
        if !FOUND {
            diag_write_str("[WIFI] PREREQ=NO-DEVICE\n");
            return;
        }
        let cmdstat = pci_r32(BUS, DEV, FUNC, 0x04);
        PCI_COMMAND = (cmdstat & 0xFFFF) as u16;
        PCI_STATUS = (cmdstat >> 16) as u16;
        let il = pci_r32(BUS, DEV, FUNC, 0x3C);
        IRQ_LINE = (il & 0xFF) as u8;
        IRQ_PIN = ((il >> 8) & 0xFF) as u8;
        PREREQS_READ = true;
        diag_write_str("[WIFI] PREREQ PCI_CMD=");
        diag_write_hex(PCI_COMMAND as usize);
        diag_write_str(" STATUS=");
        diag_write_hex(PCI_STATUS as usize);
        diag_write_str(" IRQ_LINE=");
        diag_write_usize(IRQ_LINE as usize);
        diag_write_str(" IRQ_PIN=");
        diag_write_usize(IRQ_PIN as usize);
        diag_write_str(" MMIO=");
        diag_write_str(if MMIO_MAPPED { "READY" } else { "NOT-MAPPED" });
        diag_write_str(" FW=");
        diag_write_str(if NEEDS_FW { "REQUIRED" } else { "LOADED" });
        diag_write_str("\n");
        diag_write_str("[WIFI] PREREQ RESET=NOT-TOUCHED INTERRUPTS=NOT-ENABLED DMA=NOT-STARTED\n");
        diag_write_str("[WIFI] PREREQ firmware loader=NOT-IMPLEMENTED (contract only)\n");
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
                    diag_write_str("[WIFI] SURVEY FOUND ");
                    diag_write_usize(bus as usize);
                    diag_write_str(":");
                    diag_write_usize(dev as usize);
                    diag_write_str(".");
                    diag_write_usize(func as usize);
                    diag_write_str(" DID=");
                    diag_write_hex(did as usize);
                    diag_write_str(" SUB=");
                    diag_write_hex(subsys as usize);
                    diag_write_str(" BAR0=");
                    diag_write_hex(bar0 as usize);
                    diag_write_str(" (READ-ONLY)\n");
                    return;
                }
            }
        }
        FOUND = false;
        READY = false;
        NEEDS_FW = true;
        diag_write_str("[WIFI] SURVEY no Intel WLAN on buses 0..31\n");
    }
}

/// Force probe Intel Centrino Wireless-N 2230 (AH532) and any 02:80 Intel WLAN
pub fn init() {
    diag_write_str("[WIFI] AH532 target: Centrino Wireless-N 2230 (8086:0887)\n");
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

                    diag_write_str("[WIFI] FOUND ");
                    diag_write_usize(bus as usize);
                    diag_write_str(":");
                    diag_write_usize(dev as usize);
                    diag_write_str(".");
                    diag_write_usize(func as usize);
                    diag_write_str(" DID=");
                    diag_write_hex(did as usize);
                    diag_write_str(" SUB=");
                    diag_write_hex(subsys as usize);
                    diag_write_str(" BAR0=");
                    diag_write_hex(bar0 as usize);
                    if is_2230 {
                        diag_write_str(" Centrino-N-2230");
                        if subsys == SUBSYS_BGN {
                            diag_write_str(" BGN");
                        }
                    }
                    diag_write_str("\n");

                    if bar0 != 0 && map_mmio(bar0, 0x2000) {
                        MMIO = bar0 as usize;
                        MMIO_MAPPED = true;
                        READY = true;
                        diag_write_str("[WIFI] MMIO mapped 8K phase1 OK\n");
                        // Touch first dword (alive check) — may be 0 without FW
                        let v = core::ptr::read_volatile(MMIO as *const u32);
                        diag_write_str("[WIFI] MMIO[0]=");
                        diag_write_hex(v as usize);
                        diag_write_str("\n");
                    } else {
                        diag_write_str("[WIFI] MMIO map FAIL or BAR0=0\n");
                    }
                    diag_write_str("[WIFI] firmware contract: iwlwifi-2030-5/6.ucode\n");
                    diag_write_str("[WIFI] assoc/TX requires firmware loader — NEEDS_FW\n");
                    return;
                }
            }
        }
        diag_write_str("[WIFI] no Intel WLAN on buses 0..31\n");
    }
}
