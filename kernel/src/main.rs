#![no_std]
#![no_main]

mod serial;
mod mm;
mod capability;
mod personality;
mod pe;
mod winamp;
mod win32_runtime;
mod winamp_builtin { include!("../build/winamp_blob.rs"); }
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
mod ai_agent;
mod virt_core;
mod virt_runtime;
mod smp;
mod desktop;
mod files_mgr;
mod ring3_resume;
mod time;
mod media_builtin;
mod wallpaper;
mod cursor_builtin;
mod mp3_decoder;
mod media_player;
mod alarm;
mod log;
mod shell;
mod userspace;
mod elf;
mod elf_blobs;
mod drivers { pub mod intel_igpu; pub mod intel_kms; pub mod ps2; pub mod xhci; pub mod ata; pub mod ahci; pub mod pci_usb_diag; pub mod video; pub mod audio; pub mod wifi; pub mod net; }
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

extern "C" {
    fn enter_user_mode(entry: u64, stack: u64) -> !;
    fn enter_compat_self_test() -> u64;
    static __kernel_end: u8;
}


/// VGA text marker at column `col` (0..80), white on black — visible on real BIOS
fn multiboot_usable_memory_mib(mbi: usize) -> u64 {
    if mbi == 0 { return 0; }
    unsafe {
        let total = core::ptr::read_unaligned(mbi as *const u32) as usize;
        if total < 16 || total > 0x100000 { return 0; }
        let mut off = 8usize;
        let mut usable: u64 = 0;
        while off + 8 <= total {
            let tag_type = core::ptr::read_unaligned((mbi + off) as *const u32);
            let tag_size = core::ptr::read_unaligned((mbi + off + 4) as *const u32) as usize;
            if tag_size < 8 || off + tag_size > total { break; }
            if tag_type == 0 { break; }
            if tag_type == 6 && tag_size >= 16 {
                let entry_size = core::ptr::read_unaligned((mbi + off + 8) as *const u32) as usize;
                if entry_size < 24 { break; }
                let mut p = off + 16;
                while p + entry_size <= off + tag_size {
                    let len = core::ptr::read_unaligned((mbi + p + 8) as *const u64);
                    let typ = core::ptr::read_unaligned((mbi + p + 16) as *const u32);
                    if typ == 1 { usable = usable.saturating_add(len); }
                    p += entry_size;
                }
                break;
            }
            off = (off + tag_size + 7) & !7;
        }
        usable / (1024 * 1024)
    }
}

fn vga_write_dec(row: usize, prefix: &[u8], value: u64, suffix: &[u8], color: u8) {
    unsafe {
        let base = 0xB8000 + row * 160;
        let mut col = 0usize;
        for &b in prefix {
            if col >= 80 { return; }
            *((base + col * 2) as *mut u16) = ((color as u16) << 8) | b as u16;
            col += 1;
        }
        let mut digits = [0u8; 20];
        let mut n = 0usize;
        let mut v = value;
        if v == 0 { digits[0] = b'0'; n = 1; }
        while v != 0 && n < digits.len() {
            digits[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
        }
        while n > 0 && col < 80 {
            n -= 1;
            *((base + col * 2) as *mut u16) = ((color as u16) << 8) | digits[n] as u16;
            col += 1;
        }
        for &b in suffix {
            if col >= 80 { return; }
            *((base + col * 2) as *mut u16) = ((color as u16) << 8) | b as u16;
            col += 1;
        }
    }
}

fn vga_mark(col: usize, ch: u8) {
    unsafe {
        let p = (0xB8000 + col * 2) as *mut u16;
        *p = 0x0F00u16 | (ch as u16);
    }
}

fn draw_boot_ram_screen(detected_mib: u64, managed_mib: u64, total_mib: u64, free_mib: u64, limit: bool) {
    if !graphics::ready() {
        return;
    }
    graphics::fill(0x00000000);
    graphics::draw_str(48, 80, "AETHER OS", 0x00FFFFFF);
    graphics::draw_str(48, 120, "MEMORY INITIALIZATION", 0x0080C0FF);

    let labels = [
        "RAM DETECTED:",
        "PMM MANAGED:",
        "PMM TOTAL:",
        "PMM FREE:",
        "PMM LIMIT:",
    ];
    let values = [detected_mib, managed_mib, total_mib, free_mib, if limit { 1 } else { 0 }];

    let mut row = 0usize;
    while row < labels.len() {
        let y = 176 + row * 72;
        graphics::draw_str(48, y, labels[row], 0x00FFFFFF);

        if row == 4 {
            graphics::draw_str(48, y + 32, if limit { "YES" } else { "NO" },
                if limit { 0x00FF8080 } else { 0x0080FF80 });
        } else {
            let mut buf = [0u8; 20];
            let mut n = 0usize;
            let mut v = values[row];
            if v == 0 {
                buf[0] = b'0';
                n = 1;
            } else {
                while v != 0 && n < buf.len() {
                    buf[n] = b'0' + (v % 10) as u8;
                    v /= 10;
                    n += 1;
                }
                let mut i = 0usize;
                while i < n / 2 {
                    let j = n - 1 - i;
                    let t = buf[i];
                    buf[i] = buf[j];
                    buf[j] = t;
                    i += 1;
                }
            }
            graphics::draw_bytes(48, y + 32, &buf[..n], 0x00FFFFFF);
            graphics::draw_str(48 + n * 8, y + 32, " MiB", 0x00FFFFFF);
        }
        row += 1;
    }

    // Keep the boot RAM diagnostic visible long enough to read all values.
    // This changes only the display delay; initialization order is unchanged.
    let mut delay = 0usize;
    while delay < 600_000_000 {
        core::hint::spin_loop();
        delay += 1;
    }
}
#[no_mangle]
pub extern "C" fn kernel_main(mbi: usize) -> ! {
    // Stage markers on VGA (after boot.s wrote OK at cols 0-1)
    vga_mark(2, b'K'); // entered kernel_main
    serial::init();
    vga_mark(3, b'S'); // serial init done (or skipped)
    serial::write_str("\nAETHER OS Build 178\n");
    serial::write_str("AETHER v1.7-hw\n\n");
    // PMM must begin after the linked kernel image + .bss. The embedded
    // 2230 firmware increased the kernel beyond the old fixed 2 MiB boundary,
    // which caused early page allocations to overwrite the kernel itself.
    let kernel_end = unsafe { &__kernel_end as *const u8 as usize };
    virt_core::init(mbi, kernel_end);
    virt_runtime::init();
    let pmm_start = (kernel_end + 0x1F_FFFF) & !0x1F_FFFF;
    serial::write_str("[PMM] kernel_end=");
    serial::write_hex(kernel_end);
    serial::write_str(" start=");
    serial::write_hex(pmm_start);
    serial::write_str("\n");
    mm::init(pmm_start, 64 * 1024 * 1024);
    virt_runtime::observe_system();
    // Capture the real Multiboot RAM value for the graphical boot screen.
    let detected_ram_mib = multiboot_usable_memory_mib(mbi);
    let managed_ram_mib = (mm::total_count() as u64) / 256;
    serial::write_str("[RAM] Multiboot usable=");
    serial::write_usize(detected_ram_mib as usize);
    serial::write_str(" MiB, PMM managed=");
    serial::write_usize(managed_ram_mib as usize);
    serial::write_str(" MiB\\n");
    serial::write_str("[RAM] PMM TOTAL=");
    serial::write_usize(mm::total_count() / 256);
    serial::write_str(" MiB FREE=");
    serial::write_usize(mm::free_count() / 256);
    serial::write_str(" MiB\\n");
    if detected_ram_mib > managed_ram_mib {
        serial::write_str("[RAM] PMM_LIMIT=YES (detected RAM exceeds managed range or >4GiB bitmap ceiling)\\n");
    } else {
        serial::write_str("[RAM] PMM_LIMIT=NO\\n");
    }
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
    smp::init();
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

    // The IA-32e compatibility probe must be an actual boot-stage call,
    // not merely a declared/linked entry point. Run it only after the user
    // code/stack mappings and IDT/GDT/TSS are fully initialized.
    serial::write_str("[IA32E] compatibility-mode probe START\n");
    let compat_rc = unsafe { enter_compat_self_test() };
    if compat_rc == 0xC032 {
        serial::write_str("[IA32E] compatibility-mode probe PASS\n");
    } else {
        serial::write_str("[IA32E] compatibility-mode probe FAIL rc=");
        serial::write_hex(compat_rc as usize);
        serial::write_str("\n");
    }

    drivers::ps2::init();
    // Network validation is intentionally before graphics/KMS so QEMU Ethernet
    // cannot be masked by a later display-stage stall.
    drivers::net::init();
    let _ = drivers::net::qemu_ping();
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
    virt_runtime::restore_persisted_experience();
    // Install the first external Windows target without changing the boot path.
    // The desktop remains the next foreground stage exactly as before.
    if pe::install_hello() {
        serial::write_str("[WIN32] installed /hello.exe\n");
    } else {
        serial::write_str("[WIN32] could not install /hello.exe\n");
    }
    if winamp::install() {
        serial::write_str("[WIN32] installed real Winamp executable /winamp.exe\n");
    } else {
        serial::write_str("[WIN32] could not install /winamp.exe\n");
    }
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
    drivers::intel_igpu::init();
    drivers::intel_kms::init();

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
    // Capability Ring3 regression remains available as an explicit test, but it
    // must never be a foreground boot stage. Aether Desktop is the host UI and
    // must start directly after framebuffer/storage initialization.
    serial::write_str("[CAP-RING3] boot test deferred; starting native desktop\\n");

    // If enter_user returned (should not) or load failed:
    vga_mark(16, b'G');
    fb::try_init(mbi);
    if graphics::ready() && fb::is_ready() {
        serial::write_str("[DESKTOP] starting (no Ring3)\n");
        drivers::video::init();
        draw_boot_ram_screen(detected_ram_mib, managed_ram_mib, mm::total_count() as u64 / 256, mm::free_count() as u64 / 256, detected_ram_mib > managed_ram_mib);
        desktop::terminal_write("RAM DETECTED: ");
        desktop::terminal_write_usize(detected_ram_mib as usize);
        desktop::terminal_write(" MiB\\nPMM MANAGED: ");
        desktop::terminal_write_usize(managed_ram_mib as usize);
        desktop::terminal_write(" MiB\\nPMM LIMIT: ");
        desktop::terminal_write(if detected_ram_mib > managed_ram_mib { "YES\\n" } else { "NO\\n" });
        drivers::audio::init();
        media_player::init();
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
