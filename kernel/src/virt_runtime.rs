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
static mut NEXT_REASONING_REQUEST_ID: u64 = 1;
static mut REASONING_REQUEST_ID: u64 = 0;
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
static mut LAST_REASONING_CLASSIFICATION: u8 = 0;
static mut LAST_ACTION_GATE: u8 = 0;
static mut LAST_ACTION_AUTHORIZATION: u8 = 0;
static mut LAST_ACTION_KIND: u8 = 0;
static mut ACTION_VERIFY_STATE: u8 = 0;
static mut ACTION_VERIFY_EXPECTED_COUNT: u64 = 0;
static mut SYSTEM_STATE: [u8; SYSTEM_SUBSYSTEM_COUNT] = [SYSTEM_STATE_UNKNOWN; SYSTEM_SUBSYSTEM_COUNT];
static mut PREVIOUS_SYSTEM_STATE: [u8; SYSTEM_SUBSYSTEM_COUNT] = [SYSTEM_STATE_UNKNOWN; SYSTEM_SUBSYSTEM_COUNT];
static mut SYSTEM_STATE_CHANGES: u64 = 0;
static mut SYSTEM_STATE_HEALTH: u8 = SYSTEM_HEALTH_UNKNOWN;
static mut SYSTEM_STATE_INITIALIZED: bool = false;

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

pub const REASONING_CLASS_NONE: u8 = 0;
pub const REASONING_CLASS_ERROR: u8 = 1;
pub const REASONING_CLASS_OBSERVATION: u8 = 2;
pub const REASONING_CLASS_RECOMMENDATION: u8 = 3;
pub const REASONING_CLASS_ACTION: u8 = 4;
pub const REASONING_CLASS_TEXT: u8 = 5;

pub const ACTION_GATE_NONE: u8 = 0;
pub const ACTION_GATE_PROPOSED: u8 = 1;

pub const ACTION_AUTH_NONE: u8 = 0;
pub const ACTION_AUTH_GRANTED: u8 = 1;

// First executable capability: a bounded internal runtime marker only.
// It performs no device I/O, process mutation, filesystem write, or model call.
pub const ACTION_KIND_NONE: u8 = 0;
pub const ACTION_KIND_RUNTIME_MARK: u8 = 1;

pub const ACTION_VERIFY_NONE: u8 = 0;
pub const ACTION_VERIFY_PENDING: u8 = 1;
pub const ACTION_VERIFY_PASSED: u8 = 2;
pub const ACTION_VERIFY_FAILED: u8 = 3;

pub const SYSTEM_STATE_UNKNOWN: u8 = 0;
pub const SYSTEM_STATE_READY: u8 = 1;
pub const SYSTEM_STATE_DEGRADED: u8 = 2;
pub const SYSTEM_STATE_FAILED: u8 = 3;

pub const SYSTEM_HEALTH_UNKNOWN: u8 = 0;
pub const SYSTEM_HEALTH_READY: u8 = 1;
pub const SYSTEM_HEALTH_DEGRADED: u8 = 2;
pub const SYSTEM_HEALTH_FAILED: u8 = 3;

pub const SYSTEM_CPU: usize = 0;
pub const SYSTEM_MEMORY: usize = 1;
pub const SYSTEM_PROCESSES: usize = 2;
pub const SYSTEM_GRAPHICS: usize = 3;
pub const SYSTEM_DISPLAY: usize = 4;
pub const SYSTEM_AUDIO: usize = 5;
pub const SYSTEM_WIFI: usize = 6;
pub const SYSTEM_NETWORK: usize = 7;
pub const SYSTEM_STORAGE: usize = 8;
pub const SYSTEM_FILESYSTEM: usize = 9;
pub const SYSTEM_USB: usize = 10;
pub const SYSTEM_INPUT: usize = 11;
pub const SYSTEM_DESKTOP: usize = 12;
pub const SYSTEM_AI: usize = 13;
pub const SYSTEM_QEMU: usize = 14;
pub const SYSTEM_BOOT: usize = 15;
pub const SYSTEM_INTERRUPTS: usize = 16;
pub const SYSTEM_RUNTIME: usize = 17;
pub const SYSTEM_SUBSYSTEM_COUNT: usize = 18;

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
        NEXT_REASONING_REQUEST_ID = 1;
        REASONING_REQUEST_ID = 0;
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
        LAST_REASONING_CLASSIFICATION = REASONING_CLASS_NONE;
        LAST_ACTION_GATE = ACTION_GATE_NONE;
        LAST_ACTION_AUTHORIZATION = ACTION_AUTH_NONE;
        LAST_ACTION_KIND = ACTION_KIND_NONE;
        ACTION_VERIFY_STATE = ACTION_VERIFY_NONE;
        ACTION_VERIFY_EXPECTED_COUNT = 0;
        SYSTEM_STATE = [SYSTEM_STATE_UNKNOWN; SYSTEM_SUBSYSTEM_COUNT];
        PREVIOUS_SYSTEM_STATE = [SYSTEM_STATE_UNKNOWN; SYSTEM_SUBSYSTEM_COUNT];
        SYSTEM_STATE_CHANGES = 0;
        SYSTEM_STATE_HEALTH = SYSTEM_HEALTH_UNKNOWN;
        SYSTEM_STATE_INITIALIZED = false;
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
            observe_system_state();
            observe_processes();
            decide();
            evaluate_reasoning_gate();
            emit_reasoning_request();
            act_if_authorized();
            verify_last_action();
        }
    }
}

/// Capture a one-second whole-OS snapshot from authoritative state sources.
/// Unknown means the corresponding subsystem has not yet exposed a trustworthy
/// runtime status; it is never guessed as healthy.
fn observe_system_state() {
    if !ready() { return; }
    let total = crate::mm::total_count();
    let free = crate::mm::free_count();
    let process_count = crate::process::snapshot_states(
        &mut [0usize; crate::process::MAX_PROCESSES],
        &mut [0u8; crate::process::MAX_PROCESSES],
    );
    let graphics_ready = crate::graphics::ready();
    let network = crate::ai_agent::last_network_probe();

    unsafe {
        PREVIOUS_SYSTEM_STATE = SYSTEM_STATE;
        SYSTEM_STATE = [SYSTEM_STATE_UNKNOWN; SYSTEM_SUBSYSTEM_COUNT];

        SYSTEM_STATE[SYSTEM_CPU] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_MEMORY] = if total > 0 && free <= total {
            SYSTEM_STATE_READY
        } else {
            SYSTEM_STATE_FAILED
        };
        SYSTEM_STATE[SYSTEM_PROCESSES] = if process_count <= crate::process::MAX_PROCESSES {
            SYSTEM_STATE_READY
        } else {
            SYSTEM_STATE_FAILED
        };
        SYSTEM_STATE[SYSTEM_GRAPHICS] = if graphics_ready {
            SYSTEM_STATE_READY
        } else {
            SYSTEM_STATE_FAILED
        };
        SYSTEM_STATE[SYSTEM_DISPLAY] = SYSTEM_STATE[SYSTEM_GRAPHICS];
        SYSTEM_STATE[SYSTEM_AUDIO] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_WIFI] = if network.0 != 0 && network.1 == 0 {
            SYSTEM_STATE_DEGRADED
        } else if network.0 != 0 && network.1 != 0 {
            SYSTEM_STATE_READY
        } else {
            SYSTEM_STATE_UNKNOWN
        };
        SYSTEM_STATE[SYSTEM_NETWORK] = if network.1 != 0 {
            SYSTEM_STATE_READY
        } else if network.0 != 0 {
            SYSTEM_STATE_DEGRADED
        } else {
            SYSTEM_STATE_UNKNOWN
        };
        SYSTEM_STATE[SYSTEM_STORAGE] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_FILESYSTEM] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_USB] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_INPUT] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_DESKTOP] = if graphics_ready {
            SYSTEM_STATE_READY
        } else {
            SYSTEM_STATE_UNKNOWN
        };
        SYSTEM_STATE[SYSTEM_AI] = if REASONING_WAITING {
            SYSTEM_STATE_DEGRADED
        } else {
            SYSTEM_STATE_READY
        };
        SYSTEM_STATE[SYSTEM_QEMU] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_BOOT] = SYSTEM_STATE_READY;
        SYSTEM_STATE[SYSTEM_INTERRUPTS] = SYSTEM_STATE_UNKNOWN;
        SYSTEM_STATE[SYSTEM_RUNTIME] = if ACTIVE && STATE == STATE_AWAKE {
            SYSTEM_STATE_READY
        } else {
            SYSTEM_STATE_FAILED
        };

        if SYSTEM_STATE_INITIALIZED {
            let mut i = 0usize;
            while i < SYSTEM_SUBSYSTEM_COUNT {
                if SYSTEM_STATE[i] != PREVIOUS_SYSTEM_STATE[i] {
                    SYSTEM_STATE_CHANGES = SYSTEM_STATE_CHANGES.wrapping_add(1);
                }
                i += 1;
            }
        } else {
            SYSTEM_STATE_INITIALIZED = true;
        }

        let mut has_failed = false;
        let mut has_degraded = false;
        let mut has_unknown = false;
        let mut i = 0usize;
        while i < SYSTEM_SUBSYSTEM_COUNT {
            match SYSTEM_STATE[i] {
                SYSTEM_STATE_FAILED => has_failed = true,
                SYSTEM_STATE_DEGRADED => has_degraded = true,
                SYSTEM_STATE_UNKNOWN => has_unknown = true,
                _ => {}
            }
            i += 1;
        }
        SYSTEM_STATE_HEALTH = if has_failed {
            SYSTEM_HEALTH_FAILED
        } else if has_degraded {
            SYSTEM_HEALTH_DEGRADED
        } else if has_unknown {
            SYSTEM_HEALTH_UNKNOWN
        } else {
            SYSTEM_HEALTH_READY
        };
    }
}

pub fn system_state(subsystem: usize) -> u8 {
    if subsystem >= SYSTEM_SUBSYSTEM_COUNT { return SYSTEM_STATE_UNKNOWN; }
    unsafe { SYSTEM_STATE[subsystem] }
}

pub fn system_state_health() -> u8 { unsafe { SYSTEM_STATE_HEALTH } }
pub fn system_state_changes() -> u64 { unsafe { SYSTEM_STATE_CHANGES } }

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
fn act_if_authorized() {
    unsafe {
        if LAST_ACTION_AUTHORIZATION != ACTION_AUTH_GRANTED {
            return;
        }
    }
    let _ = execute_authorized_action();
}

fn emit_reasoning_request() {
    if !ready() { return; }
    unsafe {
        if LAST_REASONING_GATE != REASONING_GATE_REQUEST {
            return;
        }
        let request_id = NEXT_REASONING_REQUEST_ID;
        NEXT_REASONING_REQUEST_ID = NEXT_REASONING_REQUEST_ID.wrapping_add(1);
        REASONING_REQUEST_ID = request_id;
        serial::write_str("AI_REQ:REQ=R");
        serial::write_usize(request_id as usize);
        serial::write_str(" SRC=RUNTIME EVENT=STATE_CHANGE PID=");
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
pub fn reasoning_request_id() -> u64 { unsafe { REASONING_REQUEST_ID } }

/// Accept the bounded response belonging to the runtime reasoning request.
pub fn receive_reasoning_response(request_id: u64, bytes: &[u8]) {
    if !ready() { return; }
    unsafe {
        if !REASONING_WAITING || request_id != REASONING_REQUEST_ID { return; }
        let n = bytes.len().min(LAST_REASONING_RESPONSE.len());
        let mut i = 0usize;
        while i < n { LAST_REASONING_RESPONSE[i] = bytes[i]; i += 1; }
        LAST_REASONING_RESPONSE_LEN = n;
        LAST_REASONING_CLASSIFICATION = classify_reasoning_response(n);
        LAST_ACTION_GATE = if LAST_REASONING_CLASSIFICATION == REASONING_CLASS_ACTION {
            LAST_ACTION_KIND = ACTION_KIND_RUNTIME_MARK;
            ACTION_GATE_PROPOSED
        } else {
            LAST_ACTION_KIND = ACTION_KIND_NONE;
            ACTION_GATE_NONE
        };
        REASONING_WAITING = false;
        serial::write_str("[VIRT RUNTIME] REASONING_RESPONSE_LEN=");
        serial::write_usize(n);
        serial::write_str("\n");
    }
}

fn response_contains(needle: &[u8], len: usize) -> bool {
    if needle.is_empty() || len < needle.len() { return false; }
    unsafe {
        let mut i = 0usize;
        while i + needle.len() <= len {
            let mut j = 0usize;
            let mut matched = true;
            while j < needle.len() {
                let mut a = LAST_REASONING_RESPONSE[i + j];
                let mut b = needle[j];
                if a >= b'A' && a <= b'Z' { a = a + 32; }
                if b >= b'A' && b <= b'Z' { b = b + 32; }
                if a != b { matched = false; break; }
                j += 1;
            }
            if matched { return true; }
            i += 1;
        }
    }
    false
}

/// Classify the accepted model response without interpreting or executing it.
/// This is intentionally deterministic and bounded; it is not yet semantic understanding.
fn classify_reasoning_response(len: usize) -> u8 {
    if len == 0 { return REASONING_CLASS_TEXT; }
    if response_contains(b"model_error", len) || response_contains(b"error:", len) {
        return REASONING_CLASS_ERROR;
    }
    if response_contains(b"observe", len) || response_contains(b"observation", len) {
        return REASONING_CLASS_OBSERVATION;
    }
    if response_contains(b"recommend", len) || response_contains(b"recommendation", len) {
        return REASONING_CLASS_RECOMMENDATION;
    }
    if response_contains(b"ACTION: RUNTIME_MARK", len) {
        return REASONING_CLASS_ACTION;
    }
    REASONING_CLASS_TEXT
}

pub fn last_reasoning_response_len() -> usize { unsafe { LAST_REASONING_RESPONSE_LEN } }

pub fn last_reasoning_classification() -> u8 { unsafe { LAST_REASONING_CLASSIFICATION } }
pub fn last_action_gate() -> u8 { unsafe { LAST_ACTION_GATE } }

/// Explicitly authorize the currently proposed action without executing it.
pub fn authorize_action() -> u8 {
    unsafe {
        if LAST_ACTION_GATE != ACTION_GATE_PROPOSED {
            LAST_ACTION_AUTHORIZATION = ACTION_AUTH_NONE;
            return ACTION_AUTH_NONE;
        }
        LAST_ACTION_AUTHORIZATION = ACTION_AUTH_GRANTED;
        LAST_ACTION_GATE = ACTION_GATE_NONE;
        ACTION_AUTH_GRANTED
    }
}

pub fn last_action_authorization() -> u8 { unsafe { LAST_ACTION_AUTHORIZATION } }

pub fn last_action_kind() -> u8 { unsafe { LAST_ACTION_KIND } }

fn verify_last_action() {
    unsafe {
        if ACTION_VERIFY_STATE != ACTION_VERIFY_PENDING { return; }
        if ACTIONS == ACTION_VERIFY_EXPECTED_COUNT {
            ACTION_VERIFY_STATE = ACTION_VERIFY_PASSED;
            serial::write_str("[VIRT RUNTIME] ACTION_VERIFY=PASSED kind=RUNTIME_MARK\\n");
        } else {
            ACTION_VERIFY_STATE = ACTION_VERIFY_FAILED;
            serial::write_str("[VIRT RUNTIME] ACTION_VERIFY=FAILED kind=RUNTIME_MARK\\n");
        }
    }
}

pub fn action_verify_state() -> u8 { unsafe { ACTION_VERIFY_STATE } }

/// Execute exactly one explicitly authorized, bounded internal action.
/// No external I/O or arbitrary code execution is reachable from this path.
pub fn execute_authorized_action() -> u8 {
    unsafe {
        if LAST_ACTION_AUTHORIZATION != ACTION_AUTH_GRANTED {
            return ACTION_AUTH_NONE;
        }
        if LAST_ACTION_KIND != ACTION_KIND_RUNTIME_MARK {
            LAST_ACTION_AUTHORIZATION = ACTION_AUTH_NONE;
            LAST_ACTION_KIND = ACTION_KIND_NONE;
            return ACTION_AUTH_NONE;
        }
        ACTIONS = ACTIONS.wrapping_add(1);
        ACTION_VERIFY_EXPECTED_COUNT = ACTIONS;
        ACTION_VERIFY_STATE = ACTION_VERIFY_PENDING;
        serial::write_str("[VIRT RUNTIME] ACTION_EXECUTED kind=RUNTIME_MARK\\n");
        LAST_ACTION_AUTHORIZATION = ACTION_AUTH_NONE;
        LAST_ACTION_KIND = ACTION_KIND_NONE;
        ACTION_AUTH_GRANTED
    }
}

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
