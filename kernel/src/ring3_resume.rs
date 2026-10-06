//! Resume kernel boot after Ring3 stage (no stack return from enter_user)

use crate::serial;
use crate::fb;
use crate::graphics;
use crate::drivers;
use crate::desktop;
use crate::shell;

static mut MBI: usize = 0;
static mut ARMED: bool = false;

pub fn arm(mbi: usize) {
    unsafe {
        MBI = mbi;
        ARMED = true;
    }
}

pub fn continue_boot() -> ! {
    let mbi = unsafe { MBI };
    serial::write_str("[BOOT] resume after Ring3, mbi=");
    serial::write_hex(mbi as usize);
    serial::write_str("\n");
    fb::try_init(mbi);
    if graphics::ready() && fb::is_ready() {
        serial::write_str("[DESKTOP] post-Ring3 start\n");
        serial::write_str("[DESKTOP/STAGE] video\n");
        drivers::video::init();
        serial::write_str("[DESKTOP/STAGE] audio\n");
        drivers::audio::init();
        serial::write_str("[DESKTOP/STAGE] wifi\n");
        drivers::wifi::init();
        serial::write_str("[DESKTOP/STAGE] usb-diag\n");
        drivers::pci_usb_diag::dump_usb_controllers();
        serial::write_str("[DESKTOP/STAGE] xhci\n");
        drivers::xhci::probe();
        serial::write_str("[DESKTOP/STAGE] desktop\n");
        desktop::run();
    }
    serial::write_str("[BOOT] desktop unavailable — shell\n");
    shell::run();
}
