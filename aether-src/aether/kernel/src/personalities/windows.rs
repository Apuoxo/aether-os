//! Windows Personality

use crate::personality::PersonalityId;
use crate::serial;
use crate::process;

pub fn init() {
    serial::write_str("  [windows] init\n");
}

pub fn syscall(num: u64, _a1: u64, _a2: u64, _a3: u64) -> u64 {
    match num {
        0x01 => {
            serial::write_str("  [windows] NtWrite\n");
            0
        }
        0x2C => {
            serial::write_str("  [windows] NtTerminateProcess\n");
            0
        }
        _ => {
            serial::write_str("  [windows] unknown\n");
            u64::MAX
        }
    }
}

pub fn start_hello_process() -> Option<usize> {
    process::create(PersonalityId::Windows, 0x401000, 0x800000)
}
