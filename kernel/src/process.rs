//! Aether Process Manager — native, not Linux process model

use crate::serial;
use crate::mm;
use crate::elf;
use crate::capability::{Cap, CapTable, CAP_READ, CAP_WRITE, CAP_MAP};

pub const MAX_PROCESSES: usize = 8;

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Empty,
    Ready,
    Running,
    Exited,
}

#[derive(Clone, Copy)]
pub struct Process {
    pub pid: usize,
    /// Runtime personality owner. None means native Aether process.
    pub personality: crate::personality::PersonalityId,
    pub state: State,
    pub entry: usize,
    pub stack: usize,
    pub cr3: usize,
    pub pages: [usize; elf::MAX_IMAGE_PAGES],
    pub page_count: usize,
    pub name: [u8; 16],
    pub name_len: usize,
    /// Per-process capability handles.
    pub caps: CapTable,
}

impl Process {
    pub const fn empty() -> Self {
        Process {
            pid: 0,
            personality: crate::personality::PersonalityId::None,
            state: State::Empty,
            entry: 0,
            stack: 0,
            cr3: 0,
            pages: [0; elf::MAX_IMAGE_PAGES],
            page_count: 0,
            name: [0; 16],
            name_len: 0,
            caps: CapTable::new(),
        }
    }
}

static mut TABLE: [Process; MAX_PROCESSES] = [Process::empty(); MAX_PROCESSES];
static mut NEXT_PID: usize = 1;
static mut CURRENT_PID: usize = 0;

pub fn current_pid() -> usize {
    unsafe { CURRENT_PID }
}

pub fn set_current(pid: usize) {
    unsafe { CURRENT_PID = pid; }
}

pub fn free_count_before() -> usize {
    mm::free_count()
}

/// Create process from already-loaded ELF image
pub fn create_from_image(name: &str, img: &elf::LoadedImage) -> Option<usize> {
    create_from_image_with_personality(name, img, crate::personality::PersonalityId::None)
}

/// Create a process and bind it to exactly one personality owner.
pub fn create_from_image_with_personality(
    name: &str,
    img: &elf::LoadedImage,
    personality: crate::personality::PersonalityId,
) -> Option<usize> {
    unsafe {
        let mut slot = None;
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].state == State::Empty {
                slot = Some(i);
                break;
            }
            i += 1;
        }
        let s = slot?;
        if personality != crate::personality::PersonalityId::None
            && !crate::personality::load(personality) {
            return None;
        }
        let pid = NEXT_PID;
        NEXT_PID += 1;
        let mut p = Process::empty();
        p.pid = pid;
        p.personality = personality;
        p.state = State::Ready;

        // Bootstrap capability ABI is explicit: handles 0/1/2 are
        // READ/WRITE/MAP respectively. Do not depend on insert() ordering
        // for process creation; the capability self-test covers insert().
        p.caps.entries[0] = Cap::new(1, CAP_READ);
        p.caps.entries[1] = Cap::new(2, CAP_WRITE);
        p.caps.entries[2] = Cap::new(3, CAP_MAP);
        p.caps.used = 3;

        p.entry = img.entry;
        p.stack = img.stack_top;
        p.cr3 = img.cr3;
        p.pages = img.pages;
        p.page_count = img.page_count;
        let nb = name.as_bytes();
        let nlen = if nb.len() > 16 { 16 } else { nb.len() };
        let mut j = 0usize;
        while j < nlen {
            p.name[j] = nb[j];
            j += 1;
        }
        p.name_len = nlen;
        TABLE[s] = p;
        if personality != crate::personality::PersonalityId::None {
            crate::personality::process_attach(personality);
        }
        serial::write_str("  [PROC] create PID=");
        serial::write_usize(pid);
        serial::write_str(" name=");
        serial::write_str(name);
        serial::write_str(" entry=");
        serial::write_hex(img.entry);
        serial::write_str(" user_CR3=");
        serial::write_hex(img.cr3);
        serial::write_str(" pages=");
        serial::write_usize(img.page_count);
        serial::write_str(" caps=RWM");
        serial::write_str("\n");
        Some(pid)
    }
}

pub fn destroy(pid: usize) {
    unsafe {
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].pid == pid && TABLE[i].state != State::Empty {
                let n = TABLE[i].page_count;
                let personality = TABLE[i].personality;
                let mut j = 0usize;
                while j < n {
                    let pg = TABLE[i].pages[j];
                    if pg != 0 {
                        mm::free_page(pg);
                    }
                    j += 1;
                }
                serial::write_str("  [PROC] PID=");
                serial::write_usize(pid);
                serial::write_str(" EXIT pages_freed=");
                serial::write_usize(n);
                serial::write_str("\n");
                if personality != crate::personality::PersonalityId::None {
                    crate::personality::process_detach(personality);
                }
                TABLE[i] = Process::empty();
                if CURRENT_PID == pid {
                    CURRENT_PID = 0;
                }
                return;
            }
            i += 1;
        }
    }
}

pub fn next_ready() -> Option<usize> {
    unsafe {
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].state == State::Ready {
                return Some(TABLE[i].pid);
            }
            i += 1;
        }
        None
    }
}

pub fn get(pid: usize) -> Option<Process> {
    unsafe {
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].pid == pid && TABLE[i].state != State::Empty {
                return Some(TABLE[i]);
            }
            i += 1;
        }
        None
    }
}

pub fn update_caps(pid: usize, caps: CapTable) {
    unsafe {
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].pid == pid && TABLE[i].state != State::Empty {
                TABLE[i].caps = caps;
                return;
            }
            i += 1;
        }
    }
}

pub fn set_state(pid: usize, st: State) {
    unsafe {
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].pid == pid {
                TABLE[i].state = st;
                return;
            }
            i += 1;
        }
    }
}

/// Legacy stubs for other modules
pub fn count_by_personality(id: crate::personality::PersonalityId) -> usize {
    unsafe {
        let mut c = 0usize;
        let mut i = 0usize;
        while i < MAX_PROCESSES {
            if TABLE[i].state != State::Empty && TABLE[i].personality == id {
                c += 1;
            }
            i += 1;
        }
        c
    }
}
pub fn page_count(pid: usize) -> usize {
    get(pid).map(|p| p.page_count).unwrap_or(0)
}
pub fn exit(pid: usize) {
    destroy(pid);
}
pub fn create(
    _p: crate::personality::PersonalityId,
    entry: usize,
    stack: usize,
) -> Option<usize> {
    // Compatibility helper: preserve the caller's requested personality owner.
    // This must not silently convert a personality process into a native process.
    let img = elf::LoadedImage {
        entry,
        pages: [0; 8],
        page_count: 0,
        stack_top: stack,
        cr3: 0,
    };
    create_from_image_with_personality("stub", &img, _p)
}
