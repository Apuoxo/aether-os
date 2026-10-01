//! Intel 2230 DVM transport conformance wrapper.
//!
//! The original bring-up implementation is retained verbatim in
//! `wifi_legacy.rs`. This module applies the hardware-contract fixes that
//! must happen around that implementation: the documented HBUS write-pointer
//! doorbell at +0x060, keep-warm buffer, SCD interrupt mask, FH TX setup and
//! the initial command-queue write/read-pointer synchronization.

#[path = "wifi_legacy.rs"]
mod legacy;

pub use legacy::{
    set_wf_gui_output, found, ready, needs_firmware, mmio_ready,
    firmware_name, firmware_prefix, bar0, bus_dev_func,
    pci_command, pci_status, irq_line, irq_pin, prerequisites_read,
    cap_ptr, cap_msi, cap_msix, cap_pm, cap_chain_read, msi_ctrl,
    pcie_cap, pcie_link_status, pcie_device_status, pcie_link_speed,
    pcie_link_width, pcie_flr_supported, reset_attempted, reset_ok,
    reset_before, reset_after, activate_attempted, activate_ok,
    activate_before, activate_after, firmware_attempted, firmware_loaded,
    firmware_version, firmware_inst_size, firmware_data_size,
    firmware_init_inst_size, firmware_init_data_size, alive_log_ptr,
    alive_scd_ptr, firmware_exec_attempted, firmware_exec_started,
    rx_ready, rx_irq_count, alive_seen, alive_valid, alive_subtype,
    ui_scan_start_count, ui_scan_results_count, ui_scan_complete_count,
    ui_scan_complete_channels, ui_scan_complete_status,
    ui_scan_notification_seen, wf_post_scan_diagnostics,
    software_reset, irq_handler,
    command_queue_ready, rxon_24ghz, load_firmware, start_firmware,
    activate_nic, probe_capabilities, probe_prerequisites, survey, init,
};

const FH_MEM_LOWER_BOUND: usize = 0x1000;
const FH_KW_MEM_ADDR_REG: usize = FH_MEM_LOWER_BOUND + 0x97C;
const FH_TCSR_LOWER_BOUND: usize = FH_MEM_LOWER_BOUND + 0xD00;
const FH_TCSR_TX_DMA_CREDIT_ENABLE: u32 = 0x8000_0008;
const FH_TX_CHICKEN_BITS: usize = FH_MEM_LOWER_BOUND + 0xE98;
const FH_TX_CHICKEN_BITS_SCD_AUTO_RETRY_EN: u32 = 0x0000_0002;
const HBUS_TARG_WRPTR: usize = 0x060;
const IWL_CMD_QUEUE_NUM: u32 = 4;
const IWL_CMD_FIFO_NUM: usize = 7;
const SCD_BASE: u32 = 0x00A0_2C00;
const SCD_RDPTR: u32 = SCD_BASE + 0x68 + IWL_CMD_QUEUE_NUM * 4;
const SCD_INTERRUPT_MASK: u32 = SCD_BASE + 0x108;
const SCD_INTERRUPT_MASK_20_QUEUES: u32 = (1u32 << 20) - 1;

#[repr(align(4096))]
struct KeepWarm([u8; 4096]);
static mut KEEP_WARM: KeepWarm = KeepWarm([0; 4096]);
static mut FIXUP_WRPTR: u8 = 0;

unsafe fn prph_write(mmio: usize, addr: u32, val: u32) {
    core::ptr::write_volatile((mmio + 0x444) as *mut u32, (addr & 0x000F_FFFF) | (3 << 24));
    core::ptr::write_volatile((mmio + 0x44C) as *mut u32, val);
}

unsafe fn configure_transport() -> bool {
    let mmio = legacy::bar0() as usize;
    if mmio == 0 || !legacy::mmio_ready() {
        return false;
    }

    // Intel requires a dedicated 4 KiB, 4 KiB-aligned keep-warm buffer.
    let kw_phys = match crate::mm::paging::virt_to_phys((&KEEP_WARM.0 as *const u8) as usize) {
        Some(p) => p,
        None => return false,
    };
    if (kw_phys & 0xFFF) != 0 {
        return false;
    }
    core::ptr::write_volatile((mmio + FH_KW_MEM_ADDR_REG) as *mut u32, (kw_phys >> 4) as u32);

    // Gen1/gen2 TX startup enables DMA + credit on all eight FH TX channels.
    // Host-end-TFD CIRQ is not part of the baseline channel-enable value.
    let mut ch = 0usize;
    while ch < 8 {
        core::ptr::write_volatile(
            (mmio + FH_TCSR_LOWER_BOUND + ch * 0x20) as *mut u32,
            FH_TCSR_TX_DMA_CREDIT_ENABLE,
        );
        ch += 1;
    }

    // Match iwlwifi's legacy FH scheduler-retry setup.
    let chicken = core::ptr::read_volatile((mmio + FH_TX_CHICKEN_BITS) as *const u32);
    core::ptr::write_volatile(
        (mmio + FH_TX_CHICKEN_BITS) as *mut u32,
        chicken | FH_TX_CHICKEN_BITS_SCD_AUTO_RETRY_EN,
    );

    // All 20 hardware queues are covered by the SCD interrupt mask on 2000/2030.
    prph_write(mmio, SCD_INTERRUPT_MASK, SCD_INTERRUPT_MASK_20_QUEUES);

    // Linux's txq_set_wr_ptrs() writes the actual HBUS doorbell and seeds
    // the corresponding SCD read pointer. The legacy implementation was
    // only maintaining the private SCD write-pointer register here.
    core::ptr::write_volatile(
        (mmio + HBUS_TARG_WRPTR) as *mut u32,
        IWL_CMD_QUEUE_NUM << 8,
    );
    prph_write(mmio, SCD_RDPTR, 0);

    FIXUP_WRPTR = 0;
    true
}

/// Initialize the legacy implementation, then complete the documented
/// PCIe/FH/SCD transport prerequisites before any host command is sent.
pub fn init_command_queue() -> bool {
    unsafe {
        if !legacy::init_command_queue() {
            return false;
        }
        configure_transport()
    }
}

/// The legacy command path historically wrote the scheduler-looking +0x460
/// register instead of the documented HBUS_TARG_WRPTR at +0x060. Keep the
/// proven command construction intact, then publish the same queue index to
/// the real hardware doorbell.
pub fn send_command(cmd: u8, payload: &[u8]) -> bool {
    unsafe {
        if !legacy::send_command(cmd, payload) {
            return false;
        }
        FIXUP_WRPTR = FIXUP_WRPTR.wrapping_add(1);
        let mmio = legacy::bar0() as usize;
        core::ptr::write_volatile(
            (mmio + HBUS_TARG_WRPTR) as *mut u32,
            (FIXUP_WRPTR as u32) | (IWL_CMD_QUEUE_NUM << 8),
        );
        true
    }
}

/// Preserve the existing RXON+SCAN packet construction, but publish both
/// queued commands through the real HBUS doorbell after the legacy builder
/// has filled their TFDs.
pub fn scan_24ghz() -> bool {
    unsafe {
        let ok = legacy::scan_24ghz();
        if !ok {
            return false;
        }
        // scan_24ghz() emits exactly two command-queue entries: RXON and SCAN.
        // The real hardware pointer therefore advances from 0 to 2.
        let mmio = legacy::bar0() as usize;
        FIXUP_WRPTR = 2;
        core::ptr::write_volatile(
            (mmio + HBUS_TARG_WRPTR) as *mut u32,
            (FIXUP_WRPTR as u32) | (IWL_CMD_QUEUE_NUM << 8),
        );
        true
    }
}
