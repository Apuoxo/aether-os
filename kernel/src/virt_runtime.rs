//! Virt Runtime — persistent Aether-side presence loop.
//!
//! This is deliberately not a language model. It keeps Virt active inside
//! Aether independently of the chat window and provides the first stable
//! heartbeat boundary for later observation, memory and reasoning backends.

use crate::serial;

static mut ACTIVE: bool = false;
static mut STATE: u8 = 0;
static mut HEARTBEATS: u64 = 0;
static mut LAST_SECOND: u8 = 255;
static mut OBSERVATIONS: u64 = 0;
static mut EVENTS: u64 = 0;
static mut ACTIONS: u64 = 0;
static mut REASONING_REQUESTS: u64 = 0;

pub const STATE_OFF: u8 = 0;
pub const STATE_AWAKE: u8 = 1;

/// Activate the persistent Virt runtime once during kernel boot.
pub fn init() {
    let (_, _, _, _, _, second) = crate::time::rtc_read();
    unsafe {
        ACTIVE = true;
        STATE = STATE_AWAKE;
        HEARTBEATS = 0;
        LAST_SECOND = second;
        OBSERVATIONS = 0;
        EVENTS = 0;
        ACTIONS = 0;
        REASONING_REQUESTS = 0;
    }
    serial::write_str("[VIRT RUNTIME] ACTIVE STATE=AWAKE heartbeat=RTC-second\n");
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
            OBSERVATIONS = OBSERVATIONS.wrapping_add(1);
        }
    }
}

pub fn ready() -> bool {
    unsafe { ACTIVE }
}

pub fn heartbeat_count() -> u64 {
    unsafe { HEARTBEATS }
}

/// Record an event delivered to the Virt runtime.
///
/// Event handling is intentionally separate from reasoning: receiving an
/// event does not imply that a model must be invoked.
pub fn record_event() {
    if ready() {
        unsafe {
            EVENTS = EVENTS.wrapping_add(1);
        }
    }
}

/// Record a completed runtime action.
pub fn record_action() {
    if ready() {
        unsafe {
            ACTIONS = ACTIONS.wrapping_add(1);
        }
    }
}

/// Record a request that was explicitly handed to a reasoning backend.
pub fn record_reasoning_request() {
    if ready() {
        unsafe {
            REASONING_REQUESTS = REASONING_REQUESTS.wrapping_add(1);
        }
    }
}

pub fn state() -> u8 {
    unsafe { STATE }
}

pub fn observation_count() -> u64 {
    unsafe { OBSERVATIONS }
}

pub fn event_count() -> u64 {
    unsafe { EVENTS }
}

pub fn action_count() -> u64 {
    unsafe { ACTIONS }
}

pub fn reasoning_request_count() -> u64 {
    unsafe { REASONING_REQUESTS }
}
