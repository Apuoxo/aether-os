#![no_std]
#![no_main]

mod serial;
mod mm;
mod capability;
mod personality;
mod process;
mod sched;
mod graphics;
mod gui;
mod ui;
mod compositor;
mod input;
mod fs;
mod block;
mod part;
mod fs_fat;
mod fs_ntfs;
mod storage;
mod storage_hw_diag;
mod fb;
mod desktop;
mod files_mgr;
mod ring3_resume;
mod time;
mod log;
mod shell;
mod userspace;
mod elf;
mod elf_blobs;
mod drivers { pub mod ps2; pub mod xhci; pub mod ata; pub mod ahci; pub mod pci_usb_diag; pub mod video; pub mod audio; pub mod net; pub mod wifi; }
mod personalities {
    pub mod linux;
    pub mod windows;
    pub mod android;
}
mod arch {
    pub mod x86_64 {
        pub mod gdt;
        pub mod idt;
        pub mod handlers;
    }
}

use core::panic::PanicInfo;
use crate::mm::paging;

const AETHER_BUILD: u32 = 177;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    serial::write_str("PANIC\n");
    loop { unsafe { core::arch::asm!("hlt"); } }
}

#[no_mangle]
pub unsafe extern "C" fn memmove(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    if (d as usize) < (s as usize) {
        let mut i = 0;
        while i < n { *d.add(i) = *s.add(i); i += 1; }
    } else {
        let mut i = n;
        while i > 0 { i -= 1; *d.add(i) = *s.add(i); }
    }
    d
}
#[no_mangle]
pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    let mut i=0; while i<n { *d.add(i)=*s.add(i); i+=1; } d
}
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i=0; while i<n { *s.add(i)=c as u8; i+=1; } s
}
#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i=0; while i<n { let x=*a.add(i); let y=*b.add(i); if x!=y { return x as i32-y as i32; } i+=1; } 0
}

#[no_mangle]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        if *a.add(i) != *b.add(i) {
            return 1;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

extern "C" { fn enter_user_mode(entry: u64, stack: u64) -> !; }


/// VGA text marker at column `col` (0..80), white on black — visible on real BIOS
fn vga_mark(col: usize, ch: u8) {
    unsafe {
        let p = (0xB8000 + col * 2) as *mut u16;
        *p = 0x0F00u16 | (ch as u16);
    }
}

#[no_mangle]
pub extern "C" fn kernel_main(mbi: usize) -> ! {
    // Stage markers on VGA (after boot.s wrote OK at cols 0-1)
    vga_mark(2, b'K'); // entered kernel_main
    serial::init();
    vga_mark(3, b'S'); // serial init done (or skipped)
    serial::write_str("\nAETHER OS Build 177\n");
    serial::write_str("AETHER v1.7-hw\n\n");
    // PMM: start 2MiB, size conservative 64MiB (avoid claiming non-existent RAM)
    mm::init(2 * 1024 * 1024, 64 * 1024 * 1024);
    vga_mark(4, b'P'); // PMM
    serial::write_str("[OK] PMM\n");
    // Kernel stack must be large: rust_kernel_after_user has big locals; Ring3 TSS uses rsp0
    let mut kstack_base = 0usize;
    let mut ki = 0usize;
    while ki < 16 {
        if let Some(p) = mm::alloc_page() {
            if kstack_base == 0 { kstack_base = p; }
            // prefer contiguous
            ki += 1;
        } else { break; }
    }
    let kstack_top = if kstack_base != 0 { kstack_base + ki * 4096 } else { 0x200000 + 0x10000 };
    vga_mark(5, b'G');
    arch::x86_64::gdt::init(kstack_top as u64);
    arch::x86_64::idt::init();
    vga_mark(6, b'I'); // IDT
    serial::write_str("[OK] GDT+IDT\n");
    if crate::capability::self_test() {
        serial::write_str("[CAP-TEST] rights/derive/revoke PASS\n");
    } else {
        serial::write_str("[CAP-TEST] FAIL\n");
    }    if crate::personality::self_test() {
        serial::write_str("[PERSONALITY-TEST] attach/block-unload/detach/unload PASS\n");
    } else {
        serial::write_str("[PERSONALITY-TEST] FAIL\n");
    }
    unsafe { paging::set_kernel_cr3(paging::read_cr3()); }
    serial::write_str("[OK] kernel CR3 saved=");
    serial::write_hex(unsafe { paging::kernel_cr3() });
    serial::write_str("\n");
    // Mask PIC early — avoid spurious IRQs on real HW
    unsafe {
        core::arch::asm!(
            "mov al, 0xFF; out 0x21, al; out 0xA1, al",
            options(nostack, preserves_flags)
        );
    }
    vga_mark(7, b'M'); // PIC masked

    // USER maps kept for later Ring3 — but demo deferred (HW-safe path)
    let code_phys = mm::alloc_page().unwrap_or(0x300000);
    let stack_phys = mm::alloc_page().unwrap_or(0x301000);
    unsafe {
        let c = code_phys as *mut u8;
        *c.add(0) = 0xB8; *c.add(1) = 1; *c.add(2) = 0; *c.add(3) = 0; *c.add(4) = 0;
        *c.add(5) = 0xCD; *c.add(6) = 0x80;
        *c.add(7) = 0xB8; *c.add(8) = 60; *c.add(9) = 0; *c.add(10) = 0; *c.add(11) = 0;
        *c.add(12) = 0xCD; *c.add(13) = 0x80;
        *c.add(14) = 0xF4;
        let cr3 = paging::read_cr3();
        let _ = paging::map_page(cr3, 0x40000000, code_phys,
            paging::PAGE_PRESENT | paging::PAGE_WRITE | paging::PAGE_USER);
        let _ = paging::map_page(cr3, 0x40001000, stack_phys,
            paging::PAGE_PRESENT | paging::PAGE_WRITE | paging::PAGE_USER);
        paging::load_cr3(cr3);
    }
    vga_mark(8, b'U');
    serial::write_str("[OK] USER\n");

    drivers::ps2::init();
    vga_mark(9, b'Y');
    unsafe {
        core::arch::asm!("mov al, 0xFF; out 0x21, al; out 0xA1, al", options(nostack, preserves_flags));
    }
    // Skip Ring3 demo on boot stack path — avoids return on 4K TSS stack
    vga_mark(10, b'r'); // ring3 demo skipped
    vga_mark(11, b'3'); // placeholder compatibility
    serial::write_str("[HW] skip Ring3 demo — stay on boot stack\n");

    vga_mark(12, b'F');
    vga_mark(13, b'c'); // GUI skip
    serial::write_str("[FB] text mode\n");

    vga_mark(14, b'A');
    let stor_ok = fs::init_storage();
    storage_hw_diag::run();
    let _ahci = drivers::ahci::init();
    storage::init();
    vga_mark(15, b'D');
    unsafe {
        let msg = if stor_ok {
            b"RAMDISK OK | AetherFS mounted"
        } else {
            b"STORAGE FAIL                 "
        };
        let mut i = 0usize;
        while i < msg.len() && i < 80 {
            let p = (0xB8000 + 160 + i * 2) as *mut u16;
            *p = 0x0A00u16 | (msg[i] as u16);
            i += 1;
        }
    }

    // Prepare the existing Multiboot framebuffer before Ring3. This only
    // discovers/maps the firmware-provided scanout surface; it does not
    // program Intel display registers or touch the known-unsafe GGTT/GSM.
    serial::write_str("\n======== USERSPACE DISPLAY PREP ========\n");
    if fb::init_from_mbi(mbi) {
        serial::write_str("[APP-DISPLAY] framebuffer ready before Ring3\n");
        let _ = drivers::video::init();
    } else {
        serial::write_str("[APP-DISPLAY] framebuffer unavailable; apps remain headless\n");
    }

    // ---- Permanent userspace: /bin/init -> Calculator -> /bin/sh ----
    ring3_resume::arm(mbi);
    serial::write_str("\n======== USERSPACE INIT STAGE ========\n");
    serial::write_str("[INIT] install ELF /bin/init /bin/sh to AetherFS\n");
    let _ = fs::write_large("/bin/init", elf_blobs::INIT_ELF);
    let _ = fs::write_large("/bin/sh", elf_blobs::SH_ELF);
    if elf_blobs::calculator::CALCULATOR_ELF.len() > 4 {
        serial::write_str("[INIT] Calculator ELF bundled bytes=");
        serial::write_usize(elf_blobs::calculator::CALCULATOR_ELF.len());
        serial::write_str(" (RAM/package path; no host-disk write)\n");
    } else {
        serial::write_str("[INIT] Calculator ELF not bundled\n");
    }
    serial::write_str("[CAP-RING3] starting real allow/deny regression\\n");
    if !arch::x86_64::handlers::start_capability_ring3_test() {
        serial::write_str("[CAP-RING3] launch failed; continuing normal boot\\n");
    }

    // If enter_user returned (should not) or load failed:
    vga_mark(16, b'G');
    fb::try_init(mbi);
    if graphics::ready() && fb::is_ready() {
        serial::write_str("[DESKTOP] starting (no Ring3)\n");
        drivers::video::init();
        drivers::audio::init();
        drivers::net::init();
        drivers::wifi::init();
        drivers::pci_usb_diag::dump_usb_controllers();
        drivers::xhci::probe();
        desktop::run();
    }

    vga_mark(17, b'X');
    drivers::pci_usb_diag::dump_usb_controllers();
    drivers::xhci::probe();
    vga_mark(18, b'H');
    // clear line 2 status
    unsafe {
        let mut i = 0usize;
        let msg = b"Aether READY - shell";
        while i < msg.len() {
            let p = (0xB8000 + 160 + i * 2) as *mut u16;
            *p = 0x0A00u16 | (msg[i] as u16);
            i += 1;
        }
    }
    serial::write_str("[OK] entering shell\n");
    shell::run();
}
