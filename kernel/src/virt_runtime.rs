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
static mut LAST_RAM_TOTAL: usize = 0;
static mut LAST_RAM_FREE: usize = 0;
static mut LAST_PID: usize = 0;
static mut LAST_PROCESS_IDS: [usize; crate::process::MAX_PROCESSES] = [0; crate::process::MAX_PROCESSES];
static mut PREVIOUS_PROCESS_IDS: [usize; crate::process::MAX_PROCESSES] = [0; crate::process::MAX_PROCESSES];
static mut LAST_PROCESS_COUNT: usize = 0;
static mut PREVIOUS_PROCESS_COUNT: usize = 0;
static mut PROCESS_BASELINE_READY: bool = false;
static mut LAST_PROCESS_EVENT_PID: usize = 0;
static mut LAST_PROCESS_EVENT_KIND: u8 = 0;
static mut LAST_PROCESS_EVENT_OLD_STATE: u8 = 0;
static mut LAST_PROCESS_EVENT_NEW_STATE: u8 = 0;
static mut LAST_PROCESS_STATES: [u8; crate::process::MAX_PROCESSES] = [0; crate::process::MAX_PROCESSES];
static mut PREVIOUS_PROCESS_STATES: [u8; crate::process::MAX_PROCESSES] = [0; crate::process::MAX_PROCESSES];

pub const PROCESS_EVENT_NONE: u8 = 0;
pub const PROCESS_EVENT_APPEARED: u8 = 1;
pub const PROCESS_EVENT_DISAPPEARED: u8 = 2;
pub const PROCESS_EVENT_STATE_CHANGED: u8 = 3;

pub const STATE_OFF: u8 = 0;
pub const STATE_AWAKE: u8 = 1;

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
        LAST_RAM_TOTAL = 0;
        LAST_RAM_FREE = 0;
        LAST_PID = 0;
        LAST_PROCESS_IDS = [0; crate::process::MAX_PROCESSES];
        PREVIOUS_PROCESS_IDS = [0; crate::process::MAX_PROCESSES];
        LAST_PROCESS_COUNT = 0;
        PREVIOUS_PROCESS_COUNT = 0;
        PROCESS_BASELINE_READY = false;
        LAST_PROCESS_EVENT_PID = 0;
        LAST_PROCESS_EVENT_KIND = PROCESS_EVENT_NONE;
        LAST_PROCESS_EVENT_OLD_STATE = 0;
        LAST_PROCESS_EVENT_NEW_STATE = 0;
        LAST_PROCESS_STATES = [0; crate::process::MAX_PROCESSES];
        PREVIOUS_PROCESS_STATES = [0; crate::process::MAX_PROCESSES];
    }
    serial::write_str("[VIRT RUNTIME] ACTIVE STATE=AWAKE heartbeat=RTC-second\n");
}

pub fn tick() {
    if !ready() { return; }
    let (_, _, _, _, _, second) = crate::time::rtc_read();
    unsafe {
        if second != LAST_SECOND {
            LAST_SECOND = second;
            HEARTBEATS = HEARTBEATS.wrapping_add(1);
            OBSERVATIONS = OBSERVATIONS.wrapping_add(1);
            observe_processes();
        }
    }
}

pub fn observe_system() {
    if !ready() { return; }
    let total = crate::mm::total_count();
    let free = crate::mm::free_count();
    let pid = crate::process::current_pid();
    unsafe {
        LAST_RAM_TOTAL = total;
        LAST_RAM_FREE = free;
        LAST_PID = pid;
    }
    serial::write_str("[VIRT RUNTIME] OBSERVE RAM_TOTAL_PAGES=");
    serial::write_usize(total);
    serial::write_str(" RAM_FREE_PAGES=");
    serial::write_usize(free);
    serial::write_str(" PID=");
    serial::write_usize(pid);
    serial::write_str("\n");
}

/// Build the current process state map and compare it with the previous map.
pub fn observe_processes() {
    if !ready() { return; }
    let mut ids = [0usize; crate::process::MAX_PROCESSES];
    let mut states = [0u8; crate::process::MAX_PROCESSES];
    let count = crate::process::snapshot_states(&mut ids, &mut states);
    unsafe {
        LAST_PROCESS_EVENT_PID = 0;
        LAST_PROCESS_EVENT_KIND = PROCESS_EVENT_NONE;
        LAST_PROCESS_EVENT_OLD_STATE = 0;
        LAST_PROCESS_EVENT_NEW_STATE = 0;
        if !PROCESS_BASELINE_READY {
            PREVIOUS_PROCESS_IDS = ids;
            PREVIOUS_PROCESS_STATES = states;
            PREVIOUS_PROCESS_COUNT = count;
            LAST_PROCESS_IDS = ids;
            LAST_PROCESS_STATES = states;
            LAST_PROCESS_COUNT = count;
            PROCESS_BASELINE_READY = true;
            return;
        }
        let mut i = 0usize;
        while i < count {
            let pid = ids[i];
            let mut found = false;
            let mut j = 0usize;
            while j < PREVIOUS_PROCESS_COUNT {
                if PREVIOUS_PROCESS_IDS[j] == pid {
                    found = true;
                    if PREVIOUS_PROCESS_STATES[j] != states[i] {
                        EVENTS = EVENTS.wrapping_add(1);
                        LAST_PROCESS_EVENT_PID = pid;
                        LAST_PROCESS_EVENT_KIND = PROCESS_EVENT_STATE_CHANGED;
                        LAST_PROCESS_EVENT_OLD_STATE = PREVIOUS_PROCESS_STATES[j];
                        LAST_PROCESS_EVENT_NEW_STATE = states[i];
                    }
                    break;
                }
                j += 1;
            }
            if !found {
                EVENTS = EVENTS.wrapping_add(1);
                LAST_PROCESS_EVENT_PID = pid;
                LAST_PROCESS_EVENT_KIND = PROCESS_EVENT_APPEARED;
                LAST_PROCESS_EVENT_OLD_STATE = 0;
                LAST_PROCESS_EVENT_NEW_STATE = states[i];
            }
            i += 1;
        }
        if LAST_PROCESS_EVENT_KIND == PROCESS_EVENT_NONE {
            let mut p = 0usize;
            while p < PREVIOUS_PROCESS_COUNT {
                let pid = PREVIOUS_PROCESS_IDS[p];
                let mut found = false;
                let mut j = 0usize;
                while j < count {
                    if ids[j] == pid { found = true; break; }
                    j += 1;
                }
                if !found {
                    EVENTS = EVENTS.wrapping_add(1);
                    LAST_PROCESS_EVENT_PID = pid;
                    LAST_PROCESS_EVENT_KIND = PROCESS_EVENT_DISAPPEARED;
                    LAST_PROCESS_EVENT_OLD_STATE = PREVIOUS_PROCESS_STATES[p];
                    LAST_PROCESS_EVENT_NEW_STATE = 0;
                    break;
                }
                p += 1;
            }
        }
        PREVIOUS_PROCESS_IDS = ids;
        PREVIOUS_PROCESS_STATES = states;
        PREVIOUS_PROCESS_COUNT = count;
        LAST_PROCESS_IDS = ids;
        LAST_PROCESS_STATES = states;
        LAST_PROCESS_COUNT = count;
    }
}

pub fn last_process_count() -> usize { unsafe { LAST_PROCESS_COUNT } }
pub fn last_process_pid(index: usize) -> usize {
    if index >= crate::process::MAX_PROCESSES { return 0; }
    unsafe { LAST_PROCESS_IDS[index] }
}

/// Resolve a PID directly in the current Virt process-state map.
pub fn process_state(pid: usize) -> u8 {
    unsafe {
        let mut i = 0usize;
        while i < LAST_PROCESS_COUNT {
            if LAST_PROCESS_IDS[i] == pid { return LAST_PROCESS_STATES[i]; }
            i += 1;
        }
        0
    }
}

/// Resolve a PID in the previous observation map.
pub fn previous_process_state(pid: usize) -> u8 {
    unsafe {
        let mut i = 0usize;
        while i < PREVIOUS_PROCESS_COUNT {
            if PREVIOUS_PROCESS_IDS[i] == pid { return PREVIOUS_PROCESS_STATES[i]; }
            i += 1;
        }
        0
    }
}

pub fn last_process_event_kind() -> u8 { unsafe { LAST_PROCESS_EVENT_KIND } }
pub fn last_process_event_pid() -> usize { unsafe { LAST_PROCESS_EVENT_PID } }
pub fn last_process_event_old_state() -> u8 { unsafe { LAST_PROCESS_EVENT_OLD_STATE } }
pub fn last_process_event_new_state() -> u8 { unsafe { LAST_PROCESS_EVENT_NEW_STATE } }
pub fn last_process_state(index: usize) -> u8 {
    if index >= crate::process::MAX_PROCESSES { return 0; }
    unsafe { LAST_PROCESS_STATES[index] }
}
pub fn last_ram_total_pages() -> usize { unsafe { LAST_RAM_TOTAL } }
pub fn last_ram_free_pages() -> usize { unsafe { LAST_RAM_FREE } }
pub fn last_pid() -> usize { unsafe { LAST_PID } }
pub fn ready() -> bool { unsafe { ACTIVE } }
pub fn heartbeat_count() -> u64 { unsafe { HEARTBEATS } }
pub fn record_event() { if ready() { unsafe { EVENTS = EVENTS.wrapping_add(1); } } }
pub fn record_action() { if ready() { unsafe { ACTIONS = ACTIONS.wrapping_add(1); } } }
pub fn record_reasoning_request() { if ready() { unsafe { REASONING_REQUESTS = REASONING_REQUESTS.wrapping_add(1); } } }
pub fn state() -> u8 { unsafe { STATE } }
pub fn observation_count() -> u64 { unsafe { OBSERVATIONS } }
pub fn event_count() -> u64 { unsafe { EVENTS } }
pub fn action_count() -> u64 { unsafe { ACTIONS } }
pub fn reasoning_request_count() -> u64 { unsafe { REASONING_REQUESTS } }
