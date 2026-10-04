//! Virt Runtime — persistent Aether-side presence loop.
//!
//! This is deliberately not a language model. It keeps Virt active inside
//! Aether independently of the chat window and provides the first stable
//! heartbeat boundary for later observation, memory and reasoning backends.

use crate::serial;

static mut ACTIVE: bool = false;
static mut HEARTBEATS: u64 = 0;
static mut LAST_SECOND: u8 = 255;

/// Activate the persistent Virt runtime once during kernel boot.
pub fn init() {
    let (_, _, _, _, _, second) = crate::time::rtc_read();
    unsafe {
        ACTIVE = true;
        HEARTBEATS = 0;
        LAST_SECOND = second;
    }
    serial::write_str("[VIRT RUNTIME] ACTIVE heartbeat=RTC-second\n");
}

/// Advance the runtime without invoking any reasoning backend.
///
/// The desktop loop calls this continuously. A heartbeat is recorded only
/// when the RTC second changes, so the model is not invoked and no serial
/// spam is produced every frame.
pub fn tick() {
    if !ready() {
        return;
    }
    let (_, _, _, _, _, second) = crate::time::rtc_read();
    unsafe {
        if second != LAST_SECOND {
            LAST_SECOND = second;
            HEARTBEATS = HEARTBEATS.wrapping_add(1);
        }
    }
}

pub fn ready() -> bool {
    unsafe { ACTIVE }
}

pub fn heartbeat_count() -> u64 {
    unsafe { HEARTBEATS }
}
