//! Native Aether AI-agent foundation: compact observation state.
/// This is deliberately small and deterministic; it stores structured
/// observations that later native planning/inference code can consume.
static mut LAST_NETWORK_FOUND: u8 = 0;
static mut LAST_NETWORK_READY: u8 = 0;
static mut NETWORK_OBSERVATIONS: u32 = 0;
static mut LAST_NETWORK_LINK: u8 = 0;
static mut LAST_NETWORK_MAC: [u8; 6] = [0; 6];

pub fn record_network_probe(found: u8, ready: u8) {
    record_network_state(found, ready, 0, [0; 6]);
}

pub fn record_network_state(found: u8, ready: u8, link: u8, mac: [u8; 6]) {
    unsafe {
        LAST_NETWORK_FOUND = found;
        LAST_NETWORK_READY = ready;
        LAST_NETWORK_LINK = link;
        LAST_NETWORK_MAC = mac;
        NETWORK_OBSERVATIONS = NETWORK_OBSERVATIONS.wrapping_add(1);
    }
}

pub fn last_network_probe() -> (u8, u8, u32) {
    unsafe { (LAST_NETWORK_FOUND, LAST_NETWORK_READY, NETWORK_OBSERVATIONS) }
}

pub fn last_network_state() -> (u8, u8, u8, [u8; 6], u32) {
    unsafe {
        (LAST_NETWORK_FOUND, LAST_NETWORK_READY, LAST_NETWORK_LINK, LAST_NETWORK_MAC, NETWORK_OBSERVATIONS)
    }
}
