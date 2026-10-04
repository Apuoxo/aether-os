//! Virt Runtime — persistent Aether-side presence loop.
//!
//! This runtime is the local execution boundary for the assistant inside
//! Aether. It stays active independently of the chat window and exposes a
//! safe observe -> decide boundary before any system-changing action exists.

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
static mut LAST_DECISION: u8 = 0;
static mut LAST_REASONING_GATE: u8 = 0;
static mut REASONING_WAITING: bool = false;
static mut LAST_REASONING_RESPONSE_LEN: usize = 0;
static mut LAST_REASONING_RESPONSE: [u8; 128] = [0; 128];

pub const PROCESS_EVENT_NONE: u8 = 0;
pub const PROCESS_EVENT_APPEARED: u8 = 1;
pub const PROCESS_EVENT_DISAPPEARED: u8 = 2;
pub const PROCESS_EVENT_STATE_CHANGED: u8 = 3;

pub const DECISION_NONE: u8 = 0;
pub const DECISION_MONITOR: u8 = 1;
pub const DECISION_INSPECT_PROCESS: u8 = 2;
pub const DECISION_REVIEW_STATE_CHANGE: u8 = 3;

pub const REASONING_GATE_NONE: u8 = 0;
pub const REASONING_GATE_SKIP: u8 = 1;
pub const REASONING_GATE_REQUEST: u8 = 2;

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
        LAST_DECISION = DECISION_NONE;
        LAST_REASONING_GATE = REASONING_GATE_NONE;
        REASONING_WAITING = false;
        LAST_REASONING_RESPONSE_LEN = 0;
        LAST_REASONING_RESPONSE = [0; 128];
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
            decide();
            evaluate_reasoning_gate();
            emit_reasoning_request();
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

/// Select a safe next observation target from the latest event.
///
/// This is deliberately a decision-only layer. It does not mutate a process,
/// schedule anything, access I/O, or execute a reasoning/model backend.
pub fn decide() -> u8 {
    let decision = unsafe {
        match LAST_PROCESS_EVENT_KIND {
            PROCESS_EVENT_APPEARED | PROCESS_EVENT_DISAPPEARED => DECISION_INSPECT_PROCESS,
            PROCESS_EVENT_STATE_CHANGED => DECISION_REVIEW_STATE_CHANGE,
            _ => DECISION_MONITOR,
        }
    };
    unsafe { LAST_DECISION = decision; }
    decision
}

/// Decide whether the current decision is important enough to hand to a reasoning backend.
///
/// This is only a gate. It does not call a model and does not increment the
/// reasoning-request counter; that counter remains reserved for an actual
/// transport request.
pub fn evaluate_reasoning_gate() -> u8 {
    let gate = unsafe {
        match LAST_DECISION {
            DECISION_REVIEW_STATE_CHANGE => REASONING_GATE_REQUEST,
            _ => REASONING_GATE_SKIP,
        }
    };
    unsafe { LAST_REASONING_GATE = gate; }
    gate
}

/// Emit one bounded reasoning request when the gate explicitly requests it.
fn emit_reasoning_request() {
    if !ready() { return; }
    unsafe {
        if LAST_REASONING_GATE != REASONING_GATE_REQUEST {
            return;
        }
        serial::write_str("AI_REQ:SRC=RUNTIME EVENT=STATE_CHANGE PID=");
        serial::write_usize(LAST_PROCESS_EVENT_PID);
        serial::write_str(" OLD=");
        serial::write_usize(LAST_PROCESS_EVENT_OLD_STATE as usize);
        serial::write_str(" NEW=");
        serial::write_usize(LAST_PROCESS_EVENT_NEW_STATE as usize);
        serial::write_str(" DEC=");
        serial::write_usize(LAST_DECISION as usize);
        serial::write_str("\n");
        REASONING_REQUESTS = REASONING_REQUESTS.wrapping_add(1);
        REASONING_WAITING = true;
        LAST_REASONING_RESPONSE_LEN = 0;
        LAST_REASONING_GATE = REASONING_GATE_NONE;
    }
}

pub fn reasoning_waiting() -> bool { unsafe { REASONING_WAITING } }

/// Accept the bounded response belonging to the runtime reasoning request.
pub fn receive_reasoning_response(bytes: &[u8]) {
    if !ready() { return; }
    unsafe {
        if !REASONING_WAITING { return; }
        let n = bytes.len().min(LAST_REASONING_RESPONSE.len());
        let mut i = 0usize;
        while i < n { LAST_REASONING_RESPONSE[i] = bytes[i]; i += 1; }
        LAST_REASONING_RESPONSE_LEN = n;
        REASONING_WAITING = false;
        serial::write_str("[VIRT RUNTIME] REASONING_RESPONSE_LEN=");
        serial::write_usize(n);
        serial::write_str("\n");
    }
}

pub fn last_reasoning_response_len() -> usize { unsafe { LAST_REASONING_RESPONSE_LEN } }

pub fn last_reasoning_gate() -> u8 { unsafe { LAST_REASONING_GATE } }

pub fn last_decision() -> u8 { unsafe { LAST_DECISION } }
pub fn last_process_count() -> usize { unsafe { LAST_PROCESS_COUNT } }
pub fn last_process_pid(index: usize) -> usize {
    if index >= crate::process::MAX_PROCESSES { return 0; }
    unsafe { LAST_PROCESS_IDS[index] }
}
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
