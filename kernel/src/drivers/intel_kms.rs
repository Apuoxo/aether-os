//! Intel Gen6 Sandy Bridge display MMIO bring-up — Stage 3.
//! MMIO identity mapping + bounded forcewake + read-only register dump.
//! Stage 5 primary-plane geometry path; no GGTT/GSM or EDID boot dependency.

use crate::{mm, serial, graphics};
use crate::drivers::intel_igpu;

const PCI_ADDR: u16 = 0x0CF8;
const PCI_DATA: u16 = 0x0CFC;

const FORCEWAKE_MT: usize = 0x0A188;
const FORCEWAKE_MT_ACK: usize = 0x130040;
const FORCEWAKE: usize = 0x0A18C;
const FORCEWAKE_ACK: usize = 0x130090;

const PIPEASRC: usize = 0x6001C;
const PIPEBSRC: usize = 0x6101C;
const PIPEACONF: usize = 0x70008;
const PIPEBCONF: usize = 0x71008;
const PIPEASTAT: usize = 0x70024;
const PIPEBSTAT: usize = 0x71024;
const PIPEADSL: usize = 0x70000;
const PIPEBDSL: usize = 0x71000;
const PIPESTAT_VBLANK: u32 = 1 << 1;
const VBLANK_WAIT_ITERS: usize = 2_000_000;
const DSPACNTR: usize = 0x70180;
const DSPASTRIDE: usize = 0x70188;
const DSPASIZE: usize = 0x70190;
const DSPASURF: usize = 0x7019C;
const DSPBCNTR: usize = 0x71180;
const DSPBSTRIDE: usize = 0x71188;
const DSPBSIZE: usize = 0x71190;
const DSPBSURF: usize = 0x7119C;
const PLANE_ENABLE: u32 = 1 << 31;
const PLANE_TILED: u32 = 1 << 10;
const PLANE_FORMAT_MASK: u32 = 0xF << 26;
const PLANE_FORMAT_XRGB8888: u32 = 0x6 << 26;
const PLANE_MAX_W: usize = 4096;
const PLANE_MAX_H: usize = 2160;

static mut MMIO_BASE: usize = 0;
static mut MMIO_READY: bool = false;
static mut FORCEWAKE_READY: bool = false;

unsafe fn pci_read32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") PCI_ADDR, in("eax") a,
        options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") PCI_DATA, out("eax") v,
        options(nostack, preserves_flags));
    v
}

unsafe fn pci_write32(bus: u8, dev: u8, func: u8, off: u8, value: u32) {
    let a = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") PCI_ADDR, in("eax") a,
        options(nostack, preserves_flags));
    core::arch::asm!("out dx, eax", in("dx") PCI_DATA, in("eax") value,
        options(nostack, preserves_flags));
}

unsafe fn mmio_read32(off: usize) -> u32 {
    core::ptr::read_volatile((MMIO_BASE + off) as *const u32)
}

unsafe fn mmio_write32(off: usize, value: u32) {
    core::ptr::write_volatile((MMIO_BASE + off) as *mut u32, value);
}

fn log_reg(name: &str, off: usize, value: u32) {
    serial::write_str("[KMS] ");
    serial::write_str(name);
    serial::write_str("=");
    serial::write_hex(value as usize);
    serial::write_str(" @");
    serial::write_hex(off);
    serial::write_str("\n");
}

/// Identity-map the MMIO BAR through the current kernel page tables.
/// The MMIO window is deliberately limited to 2 MiB: enough for the Gen6
/// display registers used in this stage, without introducing an aperture/GGTT.
unsafe fn map_mmio(base: usize) -> bool {
    if base == 0 || (base & 0xFFF) != 0 {
        return false;
    }

    let cr3 = crate::mm::paging::kernel_cr3();
    if cr3 == 0 {
        return false;
    }

    let mut off = 0usize;
    while off < 0x20_0000 {
        if !crate::mm::paging::map_page(
            cr3,
            base + off,
            base + off,
            crate::mm::paging::PAGE_PRESENT
                | crate::mm::paging::PAGE_WRITE
                | crate::mm::paging::PAGE_PCD
                | crate::mm::paging::PAGE_PWT,
        ) {
            return false;
        }
        off += crate::mm::paging::PAGE_SIZE;
    }
    crate::mm::paging::load_cr3(cr3);
    true
}

/// Enable PCI memory decoding if firmware left it disabled.
unsafe fn enable_memory_space() {
    let g = intel_igpu::info();
    let old = pci_read32(g.bus, g.dev, g.func, 0x04);
    let new = old | (1 << 1);
    if new != old {
        pci_write32(g.bus, g.dev, g.func, 0x04, new);
        serial::write_str("[KMS] PCI MEM=EN\n");
    } else {
        serial::write_str("[KMS] PCI MEM=already\n");
    }
}

/// Gen6 forcewake with a hard upper bound. No CLI and no unbounded polling.
pub fn forcewake_get() -> bool {
    unsafe {
        if !MMIO_READY {
            return false;
        }

        mmio_write32(FORCEWAKE_MT, 0x0001_0001);
        let mut i = 0usize;
        while i < 50_000 {
            if mmio_read32(FORCEWAKE_MT_ACK) & 1 != 0 {
                FORCEWAKE_READY = true;
                return true;
            }
            core::hint::spin_loop();
            i += 1;
        }

        // Fall back to legacy Gen6 forcewake path.
        mmio_write32(FORCEWAKE, 0x0000_0001);
        i = 0;
        while i < 50_000 {
            if mmio_read32(FORCEWAKE_ACK) & 1 != 0 {
                FORCEWAKE_READY = true;
                return true;
            }
            core::hint::spin_loop();
            i += 1;
        }

        FORCEWAKE_READY = false;
        false
    }
}

fn active_pipe() -> Option<u8> {
    unsafe {
        let a = mmio_read32(PIPEACONF) & (1 << 31) != 0;
        let b = mmio_read32(PIPEBCONF) & (1 << 31) != 0;
        match (a, b) {
            (true, false) => Some(0),
            (false, true) => Some(1),
            _ => None,
        }
    }
}

/// Wait for a bounded vertical-blank transition on the currently active pipe.
/// No interrupt is enabled and the wait is always bounded.
pub fn wait_vblank() -> bool {
    unsafe {
        let pipe = match active_pipe() {
            Some(p) => p,
            None => {
                serial::write_str("[KMS] VBLANK no unique active pipe\n");
                return false;
            }
        };
        wait_vblank_pipe(pipe)
    }
}

unsafe fn wait_vblank_pipe(pipe: u8) -> bool {
    let (stat_reg, dsl_reg) = if pipe == 0 { (PIPEASTAT, PIPEADSL) } else { (PIPEBSTAT, PIPEBDSL) };
    mmio_write32(stat_reg, PIPESTAT_VBLANK);

    let mut last_dsl = mmio_read32(dsl_reg);
    let mut moved = false;
    let mut i = 0usize;
    while i < VBLANK_WAIT_ITERS {
        let stat = mmio_read32(stat_reg);
        if stat & PIPESTAT_VBLANK != 0 {
            serial::write_str("[KMS] VBLANK pipe=");
            serial::write_usize(pipe as usize);
            serial::write_str(" DSL=");
            serial::write_hex(mmio_read32(dsl_reg) as usize);
            serial::write_str("\n");
            return true;
        }
        if (i & 0x3FF) == 0 {
            let dsl = mmio_read32(dsl_reg);
            if dsl != last_dsl {
                moved = true;
                last_dsl = dsl;
            }
        }
        core::hint::spin_loop();
        i += 1;
    }

    serial::write_str("[KMS] VBLANK=TIMEOUT pipe=");
    serial::write_usize(pipe as usize);
    serial::write_str(" DSL=");
    serial::write_hex(last_dsl as usize);
    serial::write_str(if moved { " DSL_MOVED\n" } else { " DSL_STATIC\n" });
    false
}

pub fn forcewake_put() {
    unsafe {
        if !MMIO_READY {
            return;
        }
        mmio_write32(FORCEWAKE_MT, 0x0001_0000);
        mmio_write32(FORCEWAKE, 0);
        FORCEWAKE_READY = false;
    }
}

/// Stage 3: map display MMIO, bounded-forcewake, and dump the Gen6 Pipe A state.
/// This function never programs timing or plane registers.
pub fn init() -> bool {
    unsafe {
        MMIO_READY = false;
        FORCEWAKE_READY = false;

        if !intel_igpu::ready() || intel_igpu::gen() != 6 {
            serial::write_str("[KMS] SKIP: Intel Gen6 iGPU not ready\n");
            return false;
        }

        let base = intel_igpu::mmio_bar() as usize;
        serial::write_str("[KMS] MMIO BAR0=");
        serial::write_hex(base);
        serial::write_str("\n");

        enable_memory_space();

        if !map_mmio(base) {
            serial::write_str("[KMS] MMIO MAP=FAIL (LFB path preserved)\n");
            return false;
        }

        MMIO_BASE = base;
        MMIO_READY = true;
        serial::write_str("[KMS] MMIO MAP=READY size=0x200000\n");

        if !forcewake_get() {
            serial::write_str("[KMS] FORCEWAKE=TIMEOUT\n");
            return false;
        }
        serial::write_str("[KMS] FORCEWAKE=OK\n");

        log_reg("PIPEASRC", PIPEASRC, mmio_read32(PIPEASRC));
        log_reg("PIPEACONF", PIPEACONF, mmio_read32(PIPEACONF));
        log_reg("PIPEASTAT", PIPEASTAT, mmio_read32(PIPEASTAT));
        log_reg("PIPEADSL", PIPEADSL, mmio_read32(PIPEADSL));

        let vb = wait_vblank();
        forcewake_put();
        if vb {
            serial::write_str("[KMS] STAGE4=READY\n");
        } else {
            serial::write_str("[KMS] STAGE4=TIMEOUT (LFB path preserved)\n");
        }
        true
    }
}


/// Map the Intel graphics aperture as ordinary identity-mapped RAM.
/// This is deliberately limited to the current framebuffer footprint.
unsafe fn map_aperture(phys: usize, size: usize) -> bool {
    if phys == 0 || size == 0 {
        return false;
    }
    let cr3 = crate::mm::paging::kernel_cr3();
    if cr3 == 0 {
        return false;
    }
    let start = phys & !0xFFF;
    let end = match phys.checked_add(size.saturating_add(0xFFF)) {
        Some(v) => v & !0xFFF,
        None => return false,
    };
    let mut va = start;
    while va < end {
        if !crate::mm::paging::map_page(
            cr3, va, va,
            crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_WRITE,
        ) {
            return false;
        }
        va += crate::mm::paging::PAGE_SIZE;
    }
    crate::mm::paging::load_cr3(cr3);
    true
}

/// Stage 4/5: bind the firmware-selected scanout to the current Multiboot FB.
/// Timings and panel fitter remain firmware-owned; Aether exposes only the
/// already-working mode instead of pretending to support arbitrary modes.
#[derive(Copy, Clone)]
struct PlaneState {
    cntr: u32,
    stride: u32,
    size: u32,
    surf: u32,
}

unsafe fn plane_regs(pipe: u8) -> (usize, usize, usize, usize) {
    if pipe == 0 {
        (DSPACNTR, DSPASTRIDE, DSPASIZE, DSPASURF)
    } else {
        (DSPBCNTR, DSPBSTRIDE, DSPBSIZE, DSPBSURF)
    }
}

unsafe fn snapshot_plane(pipe: u8) -> PlaneState {
    let (cntr, stride, size, surf) = plane_regs(pipe);
    PlaneState {
        cntr: mmio_read32(cntr),
        stride: mmio_read32(stride),
        size: mmio_read32(size),
        surf: mmio_read32(surf),
    }
}

unsafe fn restore_plane(pipe: u8, old: PlaneState) {
    let (cntr, stride, size, surf) = plane_regs(pipe);
    mmio_write32(cntr, old.cntr & !PLANE_ENABLE);
    mmio_write32(stride, old.stride);
    mmio_write32(size, old.size);
    mmio_write32(surf, old.surf);
    mmio_write32(cntr, old.cntr);
}

unsafe fn rollback_plane(pipe: u8, old: PlaneState, reason: &str) {
    serial::write_str("[KMS] MODESET ROLLBACK ");
    serial::write_str(reason);
    serial::write_str(" pipe=");
    serial::write_usize(pipe as usize);
    serial::write_str("\n");
    restore_plane(pipe, old);
}

fn current_mode_supported(w: usize, h: usize) -> bool {
    if !graphics::ready() || graphics::bpp() != 32 {
        return false;
    }
    let fw = graphics::width();
    let fh = graphics::height();
    fw == w && fh == h
}

pub fn set_plane_surface(surf: u32, stride: u32, w: u16, h: u16) -> bool {
    unsafe {
        if !MMIO_READY || !graphics::ready() || !FORCEWAKE_READY {
            return false;
        }
        let pipe = match active_pipe() {
            Some(p) => p,
            None => {
                serial::write_str("[KMS] PLANE REFUSE no unique active pipe\n");
                return false;
            }
        };
        let w = w as usize;
        let h = h as usize;
        if surf != 0 || w < 320 || h < 200 || w > PLANE_MAX_W || h > PLANE_MAX_H {
            serial::write_str("[KMS] PLANE REFUSE unsupported geometry\n");
            return false;
        }
        if stride == 0 || (stride & 63) != 0 || stride as usize < w.saturating_mul(4) {
            serial::write_str("[KMS] PLANE REFUSE invalid stride\n");
            return false;
        }

        let aper = intel_igpu::aperture_bar() as usize;
        let lfb = graphics::fb_addr();
        if aper == 0 || lfb != aper {
            serial::write_str("[KMS] PLANE REFUSE LFB!=GMADR LFB=");
            serial::write_hex(lfb);
            serial::write_str(" GMADR=");
            serial::write_hex(aper);
            serial::write_str("\n");
            return false;
        }

        let size = match (stride as usize).checked_mul(h) {
            Some(v) if v != 0 && v <= 16 * 1024 * 1024 => v,
            _ => return false,
        };
        if !map_aperture(aper, size) {
            serial::write_str("[KMS] GMADR MAP=FAIL\n");
            return false;
        }

        let (cntr_reg, stride_reg, size_reg, surf_reg) = plane_regs(pipe);
        let src_reg = if pipe == 0 { PIPEASRC } else { PIPEBSRC };
        let old = snapshot_plane(pipe);
        let old_src = mmio_read32(src_reg);
        let old_pfit = mmio_read32(PFIT_CONTROL);
        let old_pfit_ratio = mmio_read32(PFIT_PGM_RATIOS);

        mmio_write32(cntr_reg, old.cntr & !PLANE_ENABLE);
        let mut n = 0usize;
        while n < 80_000 && (mmio_read32(cntr_reg) & PLANE_ENABLE) != 0 {
            core::hint::spin_loop();
            n += 1;
        }
        if (mmio_read32(cntr_reg) & PLANE_ENABLE) != 0 {
            rollback_plane(pipe, old, "disable-timeout");
            return false;
        }

        // Sandy Bridge is Gen6 (965+ panel fitter semantics). Keep the
        // firmware-selected pipe timing and scale the 800x600 source to the
        // existing panel mode. Native 1366x768 disables fitting.
        if w == 1366 && h == 768 {
            mmio_write32(PFIT_PGM_RATIOS, 0);
            mmio_write32(PFIT_CONTROL, 0);
        } else if w == 800 && h == 600 {
            mmio_write32(PFIT_PGM_RATIOS, 0);
            mmio_write32(
                PFIT_CONTROL,
                PFIT_ENABLE | ((pipe as u32) << PFIT_PIPE_SHIFT)
                    | PFIT_SCALING_AUTO | PFIT_FILTER_FUZZY,
            );
        } else {
            mmio_write32(cntr_reg, old.cntr);
            return false;
        }

        mmio_write32(stride_reg, stride);
        mmio_write32(size_reg, (((h - 1) as u32) << 16) | ((w - 1) as u32));
        mmio_write32(src_reg, (((w - 1) as u32) << 16) | ((h - 1) as u32));

        // Gen6 primary-plane latch order: surface is written last.
        mmio_write32(surf_reg, surf);
        let mut plane = old.cntr & !(PLANE_FORMAT_MASK | PLANE_TILED);
        plane |= PLANE_FORMAT_XRGB8888 | PLANE_ENABLE;
        mmio_write32(cntr_reg, plane);

        let expected_size = (((h - 1) as u32) << 16) | ((w - 1) as u32);
        let expected_src = ((w - 1) as u32) << 16 | ((h - 1) as u32);
        let programmed_size = mmio_read32(size_reg);
        let programmed_src = mmio_read32(src_reg);
        let programmed_stride = mmio_read32(stride_reg);
        let programmed_surf = mmio_read32(surf_reg);
        let programmed_pfit = mmio_read32(PFIT_CONTROL);
        if programmed_size != expected_size
            || programmed_src != expected_src
            || programmed_stride != stride
            || programmed_surf != surf
            || (w == 800 && h == 600 && programmed_pfit & PFIT_ENABLE == 0)
            || (w == 1366 && h == 768 && programmed_pfit & PFIT_ENABLE != 0)
        {
            mmio_write32(PFIT_CONTROL, old_pfit);
            mmio_write32(PFIT_PGM_RATIOS, old_pfit_ratio);
            mmio_write32(src_reg, old_src);
            rollback_plane(pipe, old, "readback-mismatch");
            return false;
        }

        let ok = wait_vblank_pipe(pipe);
        if !ok {
            mmio_write32(PFIT_CONTROL, old_pfit);
            mmio_write32(PFIT_PGM_RATIOS, old_pfit_ratio);
            mmio_write32(src_reg, old_src);
            rollback_plane(pipe, old, "vblank-fail");
        }
        serial::write_str("[KMS] PLANE ");
        serial::write_str(if ok { "PASS" } else { "FAIL" });
        serial::write_str(" pipe=");
        serial::write_usize(pipe as usize);
        serial::write_str(" geometry=");
        serial::write_usize(w);
        serial::write_str("x");
        serial::write_usize(h);
        serial::write_str("\n");
        ok
    }
}


/// Modeset deliberately accepts only the firmware-selected framebuffer mode.
/// No arbitrary timing/panel-fitter programming is claimed in this stage.
pub fn modeset_to(w: u16, h: u16) -> bool {
    unsafe {
        if !MMIO_READY || !graphics::ready() || w == 0 || h == 0 {
            return false;
        }
        if !((w == 1366 && h == 768) || (w == 800 && h == 600)) {
            serial::write_str("[KMS] MODESET REFUSE unsupported Stage5 mode\n");
            return false;
        }
        let pipe = match active_pipe() {
            Some(p) => p,
            None => {
                serial::write_str("[KMS] MODESET REFUSE no unique active pipe\n");
                return false;
            }
        };
        let old_w = graphics::width();
        let old_h = graphics::height();
        if !forcewake_get() {
            serial::write_str("[KMS] MODESET FORCEWAKE=FAIL\n");
            return false;
        }
        let stride = graphics::pitch();
        if stride < (w as usize).saturating_mul(4) {
            forcewake_put();
            serial::write_str("[KMS] MODESET REFUSE pitch\n");
            return false;
        }

        let ok = set_plane_surface(0, stride as u32, w, h);
        if ok {
            if !graphics::resize_geometry(w as usize, h as usize) {
                serial::write_str("[KMS] MODESET graphics-sync=FAIL\n");
                forcewake_put();
                return false;
            }
            crate::drivers::ps2::clamp_to_screen();
            serial::write_str("[KMS] MODESET=PASS geometry ");
            serial::write_usize(w as usize);
            serial::write_str("x");
            serial::write_usize(h as usize);
            serial::write_str(" pipe=");
            serial::write_usize(pipe as usize);
            serial::write_str(" previous=");
            serial::write_usize(old_w);
            serial::write_str("x");
            serial::write_usize(old_h);
            serial::write_str("\n");
        }
        forcewake_put();
        ok
    }
}


pub fn ready() -> bool { unsafe { MMIO_READY } }
pub fn forcewake_ready() -> bool { unsafe { FORCEWAKE_READY } }
pub fn mmio_base() -> usize { unsafe { MMIO_BASE } }
