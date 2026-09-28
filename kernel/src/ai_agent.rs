//! Native Aether AI-agent foundation: compact observation state.
/// This is deliberately small and deterministic; it stores structured
/// observations that later native planning/inference code can consume.
static mut LAST_NETWORK_FOUND: u8 = 0;
static mut LAST_NETWORK_READY: u8 = 0;
static mut NETWORK_OBSERVATIONS: u32 = 0;

pub fn record_network_probe(found: u8, ready: u8) {
    unsafe {
        LAST_NETWORK_FOUND = found;
        LAST_NETWORK_READY = ready;
        NETWORK_OBSERVATIONS = NETWORK_OBSERVATIONS.wrapping_add(1);
    }
}

pub fn last_network_probe() -> (u8, u8, u32) {
    unsafe { (LAST_NETWORK_FOUND, LAST_NETWORK_READY, NETWORK_OBSERVATIONS) }
}
