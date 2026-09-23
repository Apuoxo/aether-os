static mut CAP_TEST_STAGE: u8 = 0;

use crate::serial;

fn vga_mark(col: usize, ch: u8) {
    unsafe {
        let p = (0xB8000 + col * 2) as *mut u16;
        *p = 0x0F00u16 | (ch as u16);
    }
}

fn vga_line2(msg: &[u8]) {
    unsafe {
        let mut i = 0usize;
        while i < msg.len() && i < 80 {
            let p = (0xB8000 + 160 + i * 2) as *mut u16;
            *p = 0x0A00u16 | (msg[i] as u16);
            i += 1;
        }
    }
}

#[no_mangle]
pub extern "C" fn page_fault_handler(cr2: u64, error: u64) {
    serial::write_str("\n!!! PF CR2=");
    serial::write_usize(cr2 as usize);
    serial::write_str(" ERR=");
    serial::write_usize(error as usize);
    let pid = crate::process::current_pid();
    serial::write_str(" PID=");
    serial::write_usize(pid);
    serial::write_str("\n");
    if error & 4 != 0 {
        serial::write_str("  [PF] PROOF: fault from CPL=3 (user access)\n");
    }
    if pid != 0 {
        serial::write_str("  [PF] killing process — isolation OK\n");
        crate::process::destroy(pid);
        crate::process::set_current(0);
        unsafe { crate::mm::paging::load_kernel_cr3(); }
        serial::write_str("  [PF] resume kernel after user PF\n");
        crate::ring3_resume::continue_boot();
    }
    serial::write_str("  [PF] kernel fault — halt\n");
    loop { unsafe { core::arch::asm!("hlt"); } }
}

/// Syscall ABI: RAX=nr, RDI=a1, RSI=a2, RDX=a3, R10=a4, R8=a5.
/// isr.s pushes GPRs in descending register order; frame indices therefore map
/// R15..RAX to 0..14, with the iret frame beginning at index 15.
#[no_mangle]
pub extern "C" fn syscall_handler(frame: *mut u64) -> u64 {
    unsafe {
        let nr = *frame.add(14);  // RAX
        let a1 = *frame.add(9);   // RDI
        let a2 = *frame.add(10);  // RSI
        let a3 = *frame.add(11);  // RDX
        let a4 = *frame.add(5);   // R10
        let a5 = *frame.add(7);   // R8
        let cs = *frame.add(16);
        let cpl = cs & 3;
        let pid = crate::process::current_pid();

        serial::write_str("  [SYSCALL] nr=");
        serial::write_usize(nr as usize);
        serial::write_str(" PID=");
        serial::write_usize(pid);
        serial::write_str(" CPL=");
        serial::write_usize(cpl as usize);
        serial::write_str(" CR3=");
        serial::write_hex(crate::mm::paging::read_cr3());
        serial::write_str("\n");

        if cpl != 3 {
            serial::write_str("  [SYSCALL] WARNING CPL!=3\n");
        } else {
            serial::write_str("  [SYSCALL] FROM CPL=3\n");
        }

        match nr {
            1 => sys_write(a1 as usize, a2 as usize),
            2 => sys_read(a1 as usize, a2 as usize, a3 as usize),
            3 => sys_list(a1 as usize, a2 as usize),
            10 => sys_poll_key(a1 as usize),
            11 => sys_draw_text(a1 as usize, a2 as usize, a3 as usize, a4 as u32),
            12 => sys_fill_rect(a1 as usize, a2 as usize, a3 as usize, a4 as usize, a5 as u32),
            13 => sys_yield(),
            60 => {
                serial::write_str("  [SYSCALL] exit FROM CPL=3\n");
                0xDEAD
            }
            _ => {
                serial::write_str("  [SYSCALL] unknown\n");
                u64::MAX
            }
        }
    }
}

unsafe fn user_ok(ptr: usize, len: usize) -> bool {
    if len > 4096 { return false; }
    if ptr < 0x40000000 || ptr.saturating_add(len) > 0x40010000 { return false; }
    true
}

fn current_has_cap(slot: usize, required: u32) -> bool {
    match unsafe { crate::process::get(crate::process::current_pid()) } {
        Some(p) => p.caps.check(slot, required),
        None => false,
    }
}

unsafe fn sys_write(buf: usize, len: usize) -> u64 {
    if !current_has_cap(1, crate::capability::CAP_WRITE) {
        serial::write_str("  [SYSCALL] write denied: capability\n");
        return u64::MAX;
    }
    if !user_ok(buf, len) {
        serial::write_str("  [SYSCALL] write bad ptr\n");
        return u64::MAX;
    }
    serial::write_str("  [SYSCALL] write FROM CPL=3 bytes=");
    serial::write_usize(len);
    serial::write_str("\n");
    let mut i = 0usize;
    while i < len {
        serial::write_byte(*((buf + i) as *const u8));
        i += 1;
    }
    len as u64
}

unsafe fn sys_read(path_ptr: usize, buf_ptr: usize, buflen: usize) -> u64 {
    if !current_has_cap(0, crate::capability::CAP_READ) {
        serial::write_str("  [SYSCALL] read denied: capability\n");
        return u64::MAX;
    }
    if !user_ok(path_ptr, 1) || !user_ok(buf_ptr, buflen) {
        serial::write_str("  [SYSCALL] read bad ptr\n");
        return u64::MAX;
    }
    let mut path = [0u8; 32];
    let mut plen = 0usize;
    while plen < 31 {
        let c = *((path_ptr + plen) as *const u8);
        if c == 0 { break; }
        path[plen] = c;
        plen += 1;
    }
    let path_s = core::str::from_utf8_unchecked(&path[..plen]);
    serial::write_str("  [SYSCALL] read path=");
    serial::write_str(path_s);
    serial::write_str(" FROM CPL=3\n");
    let mut kbuf = [0u8; 256];
    let n = match crate::fs::read(path_s, &mut kbuf) {
        Some(n) => n,
        None => return u64::MAX,
    };
    let copy = if n > buflen { buflen } else { n };
    let mut i = 0usize;
    while i < copy {
        *((buf_ptr + i) as *mut u8) = kbuf[i];
        i += 1;
    }
    copy as u64
}

unsafe fn sys_list(buf_ptr: usize, buflen: usize) -> u64 {
    if !current_has_cap(0, crate::capability::CAP_READ) {
        return u64::MAX;
    }
    if !user_ok(buf_ptr, buflen) { return u64::MAX; }
    let mut names = [[0u8; 24]; 16];
    let mut lens = [0usize; 16];
    let n = crate::fs::list(&mut names, &mut lens);
    let mut off = 0usize;
    let mut i = 0usize;
    while i < n && off < buflen {
        let mut j = 0usize;
        while j < lens[i] && off < buflen {
            *((buf_ptr + off) as *mut u8) = names[i][j];
            off += 1;
            j += 1;
        }
        if off < buflen {
            *((buf_ptr + off) as *mut u8) = b'\n';
            off += 1;
        }
        i += 1;
    }
    off as u64
}

unsafe fn sys_poll_key(out_ptr: usize) -> u64 {
    if !current_has_cap(0, crate::capability::CAP_READ) {
        return u64::MAX;
    }
    if !user_ok(out_ptr, 4) { return u64::MAX; }
    if let Some(ev) = crate::input::poll() {
        *((out_ptr) as *mut u8) = ev.key;
        *((out_ptr + 1) as *mut u8) = if ev.pressed { 1 } else { 0 };
        *((out_ptr + 2) as *mut u8) = ev.hid_code;
        *((out_ptr + 3) as *mut u8) = ev.modifiers;
        return 1;
    }
    crate::drivers::ps2::poll();
    let sc = crate::drivers::ps2::last_scancode();
    if sc != 0 {
        if let Some(key) = crate::drivers::ps2::scancode_to_ascii(sc) {
            *((out_ptr) as *mut u8) = key;
            *((out_ptr + 1) as *mut u8) = 1;
            *((out_ptr + 2) as *mut u8) = sc;
            *((out_ptr + 3) as *mut u8) = 0;
            return 1;
        }
    }
    0
}

unsafe fn sys_draw_text(x: usize, y: usize, ptr: usize, color: u32) -> u64 {
    if !current_has_cap(2, crate::capability::CAP_MAP) {
        return u64::MAX;
    }
    if !user_ok(ptr, 128) || !crate::graphics::ready() { return u64::MAX; }
    let mut n = 0usize;
    while n < 127 {
        if *((ptr + n) as *const u8) == 0 { break; }
        n += 1;
    }
    let mut buf = [0u8; 128];
    let mut i = 0usize;
    while i < n { buf[i] = *((ptr + i) as *const u8); i += 1; }
    match core::str::from_utf8(&buf[..n]) {
        Ok(text) => { crate::graphics::draw_str(x, y, text, color); n as u64 }
        Err(_) => u64::MAX,
    }
}

unsafe fn sys_fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) -> u64 {
    if !current_has_cap(2, crate::capability::CAP_MAP) {
        return u64::MAX;
    }
    if w == 0 || h == 0 || w > 1024 || h > 768 || !crate::graphics::ready() {
        return u64::MAX;
    }
    crate::graphics::fill_rect(x, y, w, h, color);
    0
}

fn sys_yield() -> u64 {
    unsafe { core::arch::asm!("pause", options(nostack, preserves_flags)); }
    0
}

fn launch_cap_test_process(name: &str, revoke_write: bool) -> bool {
    let buf = crate::elf_blobs::INIT_ELF;
    if let Some(img) = crate::elf::load(buf) {
        if let Some(pid) = crate::process::create_from_image(name, &img) {
            if revoke_write {
                if let Some(p) = crate::process::get(pid) {
                    let mut caps = p.caps;
                    if caps.revoke(1) {
                        crate::process::update_caps(pid, caps);
                    }
                }
            }
            crate::process::set_state(pid, crate::process::State::Running);
            crate::process::set_current(pid);
            serial::write_str("[CAP-RING3] launch ");
            serial::write_str(name);
            serial::write_str(" WRITE=");
            serial::write_str(if revoke_write { "DENY" } else { "ALLOW" });
            serial::write_str(" CPL=3 pending
");
            unsafe {
                crate::mm::paging::load_cr3(img.cr3);
                enter_user_mode(img.entry as u64, img.stack_top as u64);
            }
        }
    }
    false
}

pub fn start_capability_ring3_test() -> bool {
    unsafe {
        CAP_TEST_STAGE = 1;
    }
    launch_cap_test_process("init", false)
}

/// Exit path from Ring 3: destroy process, then resume kernel boot/desktop
#[no_mangle]
pub extern "C" fn process_exit_dispatch() {
    let pid = crate::process::current_pid();
    if pid != 0 {
        crate::process::destroy(pid);
        serial::write_str("  [MM] free_pages=");
        serial::write_usize(crate::mm::free_count());
        serial::write_str("\n");
    }
    crate::process::set_current(0);
    unsafe { crate::mm::paging::load_kernel_cr3(); }

    if let Some(next) = crate::process::next_ready() {
        serial::write_str("  [SCHED] switch PID=");
        serial::write_usize(next);
        serial::write_str("\n");
        crate::process::set_state(next, crate::process::State::Running);
        crate::process::set_current(next);
        if let Some(p) = crate::process::get(next) {
            unsafe {
                if p.cr3 != 0 { crate::mm::paging::load_cr3(p.cr3); }
                enter_user_mode(p.entry as u64, p.stack as u64);
            }
        }
    }
        unsafe {
        if CAP_TEST_STAGE == 1 {
            CAP_TEST_STAGE = 2;
            if launch_cap_test_process("cap-deny", true) {
                return;
            }
        }
    }

    static mut APP_STAGE: u8 = 0;
    unsafe {
        if APP_STAGE == 0 {
            APP_STAGE = 1;
            serial::write_str("\n======== USERSACE CALCULATOR STAGE ========\n");
            let buf = crate::elf_blobs::calculator::CALCULATOR_ELF;
            if buf.len() > 4 {
                if let Some(img) = crate::elf::load(buf) {
                    if let Some(pid) = crate::process::create_from_image("calculator", &img) {
                        crate::process::set_state(pid, crate::process::State::Running);
                        crate::process::set_current(pid);
                        unsafe {
                            crate::mm::paging::load_cr3(img.cr3);
                            enter_user_mode(img.entry as u64, img.stack_top as u64);
                        }
                    }
                }
            }
        }
    }
    static mut SH_DONE: bool = false;
    let launch_sh = unsafe {
        if !SH_DONE { SH_DONE = true; true } else { false }
    };
    if launch_sh {
        let mut buf = [0u8; 8192];
        if let Some(n) = crate::fs::read_large("/bin/sh", &mut buf) {
            if let Some(img) = crate::elf::load(&buf) {
                if let Some(pid) = crate::process::create_from_image("sh", &img) {
                    crate::process::set_state(pid, crate::process::State::Running);
                    crate::process::set_current(pid);
                    unsafe {
                        crate::mm::paging::load_cr3(img.cr3);
                        enter_user_mode(img.entry as u64, img.stack_top as u64);
                    }
                }
            }
        }
    }
    crate::ring3_resume::continue_boot();
}

extern "C" { fn enter_user_mode(entry: u64, stack: u64) -> !; }

fn install_user_programs() -> bool {
    use crate::fs;
    use crate::elf_blobs;
    let _ = fs::create("/bin/hello");
    if !fs::write("/bin/hello", elf_blobs::HELLO_ELF) {
        let _ = fs::create("/hello.elf");
        if !fs::write("/hello.elf", elf_blobs::HELLO_ELF) { return false; }
    }
    true
}

fn run_elf_from_fs(path: &str, name: &str) -> bool {
    use crate::fs;
    use crate::elf;
    use crate::process;
    let mut buf = [0u8; 8192];
    let n = match fs::read_large(path, &mut buf) { Some(n) => n, None => return false };
    if n < 4 || buf[0] != 0x7f || buf[1] != b'E' { return false; }
    let img = match elf::load(&buf) { Some(i) => i, None => return false };
    process::create_from_image(name, &img).is_some()
}

static mut STAGE_DONE: bool = false;

#[no_mangle]
pub extern "C" fn rust_kernel_after_user() {
    unsafe {
        if STAGE_DONE {
            serial::write_str("\nAether process stage complete\n");
            vga_mark(16, b'H');
            vga_line2(b"Aether Shell READY");
            crate::shell::run();
        }
        STAGE_DONE = true;
    }
    serial::write_str("  [kernel] Ring3 demo SUCCESS\n");
    unsafe { let p = (0xB8000 + 11 * 2) as *mut u16; *p = 0x0F00u16 | (b'3' as u16); }
    vga_mark(12, b'F');
    vga_line2(b"text mode (GUI skip)");
    vga_mark(13, b'c');
    serial::write_str("[FB] skipped — text mode\n");
    vga_mark(14, b'A');
    vga_line2(b"ATA...");
    let _ = crate::fs::init_storage();
    vga_mark(15, b'D');
    vga_line2(b"storage done     ");
    if crate::fs::write_large("/bin/hello", crate::elf_blobs::HELLO_ELF)
        && crate::fs::write_large("/bin/counter", crate::elf_blobs::COUNTER_ELF)
    {
        let free_before = crate::mm::free_count();
        serial::write_str("  [MM] free before procs=");
        serial::write_usize(free_before);
        serial::write_str("\n");
        let _ = run_elf_from_fs("/bin/hello", "hello");
        let _ = run_elf_from_fs("/bin/counter", "counter");
        if let Some(pid) = crate::process::next_ready() {
            crate::process::set_state(pid, crate::process::State::Running);
            crate::process::set_current(pid);
            if let Some(p) = crate::process::get(pid) {
                unsafe { enter_user_mode(p.entry as u64, p.stack as u64); }
            }
        }
    }
    crate::drivers::xhci::probe();
    crate::shell::run();
}

#[no_mangle]
pub extern "C" fn rust_ring3_done() -> ! {
    serial::write_str("\n======== RING3 STAGE COMPLETE ========\n");
    loop { crate::ring3_resume::continue_boot(); }
}
