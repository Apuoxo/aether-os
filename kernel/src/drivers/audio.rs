//! Aether native audio hardware foundation.
//!
//! PC speaker is diagnostic only. The media path is HDA/PCM DMA.
//! This stage discovers the HDA controller, records BAR0, maps one MMIO page,
//! and reads capability/status registers. It deliberately does not reset HDA,
//! start DMA, or send codec verbs yet.

use crate::serial;

static mut SPEAKER_OK: bool = true;
static mut HDA_FOUND: bool = false;
static mut HDA_BDF: u16 = 0;
static mut HDA_BAR0: u64 = 0;
static mut HDA_MMIO_READY: bool = false;
static mut HDA_CORB_PHYS: usize = 0;
static mut HDA_RIRB_PHYS: usize = 0;
static mut HDA_STREAM_BASE: usize = 0;
static mut HDA_STREAM_READY: bool = false;
static mut HDA_STREAM_RUNNING: bool = false;
static mut HDA_GCAP: u16 = 0;
static mut HDA_STATESTS: u16 = 0;
static mut HDA_OSS: u8 = 0;
static mut HDA_ISS: u8 = 0;
static mut HDA_BSS: u8 = 0;
static mut HDA_AFG: u8 = 0;
static mut HDA_ANALOG_PIN: u8 = 0;
static mut HDA_OUTPUT_CONV: u8 = 0;
static mut HDA_STREAM_TAG: u8 = 1;
static mut HDA_STREAM_FMT: u16 = 0;
static mut HDA_DMA_PHYS: usize = 0;
static mut HDA_BDL_PHYS: usize = 0;
static mut HDA_DMA_PERIOD: usize = 4096;
static mut HDA_DMA_PERIODS: usize = 4;
static mut HDA_DMA_NEXT: usize = 0;
static mut HDA_DMA_TOTAL: usize = 0;
static mut HDA_DMA_LAST_LPIB: u32 = 0;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nostack, preserves_flags));
    v
}
unsafe fn pci_addr(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x8000_0000u32 | ((bus as u32) << 16) | ((dev as u32) << 11)
        | ((func as u32) << 8) | ((off as u32) & 0xFC)
}
unsafe fn pci_r32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = pci_addr(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") v, options(nostack, preserves_flags));
    v
}
unsafe fn pci_w32(bus: u8, dev: u8, func: u8, off: u8, value: u32) {
    let a = pci_addr(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    core::arch::asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") value, options(nostack, preserves_flags));
}
unsafe fn hda_r8(base: usize, off: usize) -> u8 {
    core::ptr::read_volatile((base + off) as *const u8)
}
unsafe fn hda_r16(base: usize, off: usize) -> u16 {
    core::ptr::read_volatile((base + off) as *const u16)
}
unsafe fn hda_r32(base: usize, off: usize) -> u32 {
    core::ptr::read_volatile((base + off) as *const u32)
}
unsafe fn hda_w8(base: usize, off: usize, v: u8) {
    core::ptr::write_volatile((base + off) as *mut u8, v);
}
unsafe fn hda_w16(base: usize, off: usize, v: u16) {
    core::ptr::write_volatile((base + off) as *mut u16, v);
}
unsafe fn hda_w32(base: usize, off: usize, v: u32) {
    core::ptr::write_volatile((base + off) as *mut u32, v);
}
fn wait_short() {
    let mut i = 0u32;
    while i < 100_000 { core::hint::spin_loop(); i += 1; }
}

pub fn speaker_ok() -> bool { unsafe { SPEAKER_OK } }
pub fn hda_found() -> bool { unsafe { HDA_FOUND } }
pub fn hda_bar0() -> u64 { unsafe { HDA_BAR0 } }
pub fn hda_mmio_ready() -> bool { unsafe { HDA_MMIO_READY } }
pub fn hda_stream_ready() -> bool { unsafe { HDA_STREAM_READY } }
pub fn hda_stream_running() -> bool { unsafe { HDA_STREAM_RUNNING } }
pub fn hda_stream_base() -> usize { unsafe { HDA_STREAM_BASE } }
pub fn hda_stream_format() -> u16 { unsafe { HDA_STREAM_FMT } }
pub fn hda_gcap() -> u16 { unsafe { HDA_GCAP } }
pub fn hda_statests() -> u16 { unsafe { HDA_STATESTS } }
pub fn hda_oss() -> u8 { unsafe { HDA_OSS } }
pub fn hda_iss() -> u8 { unsafe { HDA_ISS } }
pub fn hda_bss() -> u8 { unsafe { HDA_BSS } }
pub fn hda_afg() -> u8 { unsafe { HDA_AFG } }
pub fn hda_analog_pin() -> u8 { unsafe { HDA_ANALOG_PIN } }
pub fn hda_output_conv() -> u8 { unsafe { HDA_OUTPUT_CONV } }
pub fn hda_dma_phys() -> usize { unsafe { HDA_DMA_PHYS } }
pub fn hda_bdl_phys() -> usize { unsafe { HDA_BDL_PHYS } }
pub fn hda_dma_total() -> usize { unsafe { HDA_DMA_TOTAL } }
pub fn hda_dma_next() -> usize { unsafe { HDA_DMA_NEXT } }
pub fn hda_lpib() -> u32 {
    unsafe {
        if !HDA_MMIO_READY || !HDA_STREAM_READY { return 0; }
        hda_r32(HDA_BAR0 as usize, HDA_STREAM_BASE + 0x04)
    }
}
pub fn hda_stream_control() -> u32 {
    unsafe {
        if !HDA_MMIO_READY || !HDA_STREAM_READY { return 0; }
        hda_r32(HDA_BAR0 as usize, HDA_STREAM_BASE)
    }
}
pub fn hda_stream_status() -> u8 {
    unsafe {
        if !HDA_MMIO_READY || !HDA_STREAM_READY { return 0; }
        hda_r8(HDA_BAR0 as usize, HDA_STREAM_BASE + 0x03)
    }
}
pub fn hda_stream_cbl() -> u32 {
    unsafe {
        if !HDA_MMIO_READY || !HDA_STREAM_READY { return 0; }
        hda_r32(HDA_BAR0 as usize, HDA_STREAM_BASE + 0x08)
    }
}
pub fn hda_stream_lvi() -> u16 {
    unsafe {
        if !HDA_MMIO_READY || !HDA_STREAM_READY { return 0; }
        hda_r16(HDA_BAR0 as usize, HDA_STREAM_BASE + 0x0C)
    }
}

pub fn beep() { beep_hz(880, 12); }

pub fn play_startup_chime() { beep_hz(880, 8); beep_hz(1320, 8); }

pub fn beep_hz(freq_hz: u32, duration_units: u32) {
    if freq_hz < 20 || freq_hz > 20000 { return; }
    unsafe {
        let div = 1193182u32 / freq_hz;
        outb(0x43, 0xB6);
        outb(0x42, (div & 0xFF) as u8);
        outb(0x42, ((div >> 8) & 0xFF) as u8);
        let t = inb(0x61);
        outb(0x61, t | 3);
        let mut i = 0u32;
        while i < duration_units.saturating_mul(40_000) {
            i += 1;
            core::hint::spin_loop();
        }
        outb(0x61, inb(0x61) & !3);
    }
}

fn probe_hda() {
    unsafe {
        HDA_FOUND = false;
        HDA_BDF = 0;
        HDA_BAR0 = 0;
        HDA_MMIO_READY = false;
        HDA_GCAP = 0;
        HDA_STATESTS = 0;
        HDA_OSS = 0;
        HDA_ISS = 0;
        HDA_BSS = 0;
        HDA_AFG = 0;
        HDA_ANALOG_PIN = 0;
        HDA_OUTPUT_CONV = 0;

        for dev in 0u8..32 {
            for func in 0u8..8 {
                let id = pci_r32(0, dev, func, 0);
                if id == 0xFFFF_FFFF || id == 0 { continue; }
                let cr = pci_r32(0, dev, func, 0x08);
                let class = ((cr >> 24) & 0xFF) as u8;
                let sub = ((cr >> 16) & 0xFF) as u8;
                if class != 0x04 || sub != 0x03 { continue; }

                let vendor = (id & 0xFFFF) as u16;
                let device = (id >> 16) as u16;
                let bar0 = pci_r32(0, dev, func, 0x10);
                let bar1 = pci_r32(0, dev, func, 0x14);
                let bar_is_io = (bar0 & 1) != 0;
                let base = if bar_is_io {
                    0
                } else if (bar0 & 0x6) == 0x4 {
                    ((bar1 as u64) << 32) | ((bar0 as u64) & 0xFFFF_FFF0)
                } else {
                    (bar0 as u64) & 0xFFFF_FFF0
                };

                HDA_FOUND = true;
                HDA_BDF = ((dev as u16) << 3) | func as u16;
                HDA_BAR0 = base;

                serial::write_str("[AUDIO] HDA BDF=00:");
                serial::write_usize(dev as usize);
                serial::write_str(".");
                serial::write_usize(func as usize);
                serial::write_str(" VID=");
                serial::write_hex(vendor as usize);
                serial::write_str(" DID=");
                serial::write_hex(device as usize);
                serial::write_str(" BAR0=");
                serial::write_hex(base as usize);
                serial::write_str("\n");

                if base == 0 || bar_is_io {
                    serial::write_str("[AUDIO] HDA BAR0 unusable; MMIO deferred\n");
                    return;
                }

                let cmd = pci_r32(0, dev, func, 0x04);
                let new_cmd = cmd | 0x0000_0006;
                if new_cmd != cmd { pci_w32(0, dev, func, 0x04, new_cmd); }

                let phys = base as usize & !0xFFF;
                let cr3 = crate::mm::paging::read_cr3();
                let mapped = crate::mm::paging::map_page(
                    cr3, phys, phys,
                    crate::mm::paging::PAGE_PRESENT
                        | crate::mm::paging::PAGE_WRITE
                        | crate::mm::paging::PAGE_PCD
                        | crate::mm::paging::PAGE_PWT,
                );
                if !mapped {
                    serial::write_str("[AUDIO] HDA MMIO map failed; untouched\n");
                    return;
                }
                crate::mm::paging::load_cr3(cr3);

                let mmio = base as usize;
                let gcap = hda_r16(mmio, 0x00);
                HDA_GCAP = gcap;
                HDA_OSS = ((gcap >> 12) & 0x0F) as u8;
                HDA_ISS = ((gcap >> 8) & 0x0F) as u8;
                HDA_BSS = ((gcap >> 3) & 0x1F) as u8;
                let gctl = hda_r32(mmio, 0x08);
                let statests = hda_r16(mmio, 0x0E);
                let intsts = hda_r32(mmio, 0x24);
                let walclk = hda_r32(mmio, 0x30);
                let corb_size = hda_r8(mmio, 0x4E);
                let rirb_size = hda_r8(mmio, 0x5E);
                HDA_MMIO_READY = true;

                serial::write_str("[AUDIO] HDA MMIO READY GCAP=");
                serial::write_hex(gcap as usize);
                serial::write_str(" GCTL=");
                serial::write_hex(gctl as usize);
                serial::write_str(" STATESTS=");
                serial::write_hex(statests as usize);
                serial::write_str(" INTSTS=");
                serial::write_hex(intsts as usize);
                serial::write_str(" WALCLK=");
                serial::write_hex(walclk as usize);
                serial::write_str(" CORBSIZE=");
                serial::write_hex(corb_size as usize);
                serial::write_str(" RIRBSIZE=");
                serial::write_hex(rirb_size as usize);
                serial::write_str("\n");
                hda_w32(mmio, 0x08, gctl & !1);
                let mut reset_ok = false;
                let mut n = 0u32;
                while n < 1000 {
                    if hda_r32(mmio, 0x08) & 1 == 0 { reset_ok = true; break; }
                    n += 1;
                    core::hint::spin_loop();
                }
                if !reset_ok {
                    serial::write_str("[AUDIO] HDA RESET_ASSERT_TIMEOUT\n");
                    return;
                }

                hda_w32(mmio, 0x08, hda_r32(mmio, 0x08) | 1);
                let mut running = false;
                n = 0;
                while n < 5000 {
                    if hda_r32(mmio, 0x08) & 1 != 0 { running = true; break; }
                    n += 1;
                    core::hint::spin_loop();
                }
                if !running {
                    serial::write_str("[AUDIO] HDA RESET_RELEASE_TIMEOUT\n");
                    return;
                }

                // Codec enumeration may take up to 25 HDA frames after CRST=1.
                let mut d = 0;
                while d < 6 { wait_short(); d += 1; }

                let states = hda_r16(mmio, 0x0E);
                HDA_STATESTS = states;
                serial::write_str("[AUDIO] HDA RESET=OK STATESTS=");
                serial::write_hex(states as usize);
                serial::write_str("\n");
                if states == 0 {
                    serial::write_str("[AUDIO] HDA CODEC_NONE\n");
                    return;
                }
                serial::write_str("[AUDIO] HDA CODEC_ADDRS=");
                serial::write_hex(states as usize);
                serial::write_str("\n");
                // Allocate physically contiguous, naturally aligned DMA rings.
                // CORB: 256 x 4-byte commands = 1 KiB.
                // RIRB: 256 x 8-byte responses = 2 KiB.
                let corb_phys = match crate::mm::alloc_pages(1) {
                    Some(p) => p,
                    None => {
                        serial::write_str("[AUDIO] CORB_ALLOC_FAIL\n");
                        return;
                    }
                };
                let rirb_phys = match crate::mm::alloc_pages(2) {
                    Some(p) => p,
                    None => {
                        crate::mm::free_page(corb_phys);
                        serial::write_str("[AUDIO] RIRB_ALLOC_FAIL\n");
                        return;
                    }
                };
                crate::mm::zero_pages(corb_phys, 1);
                crate::mm::zero_pages(rirb_phys, 2);

                let cr3_dma = crate::mm::paging::read_cr3();
                let mut p = corb_phys;
                while p < corb_phys + 0x1000 {
                    if !crate::mm::paging::map_page(
                        cr3_dma, p, p,
                        crate::mm::paging::PAGE_PRESENT
                            | crate::mm::paging::PAGE_WRITE
                            | crate::mm::paging::PAGE_PCD
                            | crate::mm::paging::PAGE_PWT,
                    ) { serial::write_str("[AUDIO] CORB_MAP_FAIL\n"); return; }
                    p += 0x1000;
                }
                p = rirb_phys;
                while p < rirb_phys + 0x2000 {
                    if !crate::mm::paging::map_page(
                        cr3_dma, p, p,
                        crate::mm::paging::PAGE_PRESENT
                            | crate::mm::paging::PAGE_WRITE
                            | crate::mm::paging::PAGE_PCD
                            | crate::mm::paging::PAGE_PWT,
                    ) { serial::write_str("[AUDIO] RIRB_MAP_FAIL\n"); return; }
                    p += 0x1000;
                }
                crate::mm::paging::load_cr3(cr3_dma);

                HDA_CORB_PHYS = corb_phys;
                HDA_RIRB_PHYS = rirb_phys;

                // Stop engines before programming base addresses.
                hda_w8(mmio, 0x4C, 0);
                hda_w8(mmio, 0x5C, 0);
                hda_w16(mmio, 0x48, 0x8000);
                hda_w16(mmio, 0x48, 0);
                hda_w16(mmio, 0x58, 0x8000);
                hda_w16(mmio, 0x58, 0);

                // Select 256-entry rings when the controller advertises that size.
                if hda_r8(mmio, 0x4E) & 0x40 != 0 {
                    hda_w8(mmio, 0x4E, (hda_r8(mmio, 0x4E) & 0xFC) | 0x02);
                }
                if hda_r8(mmio, 0x5E) & 0x40 != 0 {
                    hda_w8(mmio, 0x5E, (hda_r8(mmio, 0x5E) & 0xFC) | 0x02);
                }

                hda_w32(mmio, 0x40, corb_phys as u32);
                hda_w32(mmio, 0x44, (corb_phys >> 32) as u32);
                hda_w32(mmio, 0x50, rirb_phys as u32);
                hda_w32(mmio, 0x54, (rirb_phys >> 32) as u32);

                // One response is enough for synchronous codec discovery.
                hda_w16(mmio, 0x5A, 1);
                hda_w8(mmio, 0x4C, 0x02);
                hda_w8(mmio, 0x5C, 0x02);

                serial::write_str("[AUDIO] HDA CORB/RIRB READY CORB=");
                serial::write_hex(corb_phys);
                serial::write_str(" RIRB=");
                serial::write_hex(rirb_phys);
                serial::write_str("\n");

                // Get Parameter(Vendor ID) from root node 0 of codec address 0.
                let codec = states.trailing_zeros() as u8;
                let cmd = ((codec as u32) << 28) | (0xF00u32 << 8);
                let wp = hda_r16(mmio, 0x48) & 0x00FF;
                let next = (wp.wrapping_add(1)) & 0x00FF;
                core::ptr::write_volatile((corb_phys + (next as usize) * 4) as *mut u32, cmd);
                hda_w16(mmio, 0x48, next);

                let mut got = false;
                let mut tries = 0u32;
                while tries < 200_000 {
                    if (hda_r16(mmio, 0x58) & 0x00FF) == next { got = true; break; }
                    tries += 1;
                    core::hint::spin_loop();
                }
                if !got {
                    serial::write_str("[AUDIO] HDA VERB_TIMEOUT\n");
                    return;
                }

                let resp = core::ptr::read_volatile((rirb_phys + (next as usize) * 8) as *const u32);
                serial::write_str("[AUDIO] HDA CODEC=");
                serial::write_usize(codec as usize);
                serial::write_str(" VID_DID=");
                serial::write_hex(resp as usize);
                serial::write_str("\n");

                // Minimal bounded codec topology discovery.  We need the
                // Audio Function Group and widget ranges before selecting an
                // output pin/path; no stream is started at this stage.
                let mut verb_wp = next;
                let mut send_verb = |verb: u32| -> Option<u32> {
                    verb_wp = (verb_wp.wrapping_add(1)) & 0x00FF;
                    core::ptr::write_volatile(
                        (corb_phys + (verb_wp as usize) * 4) as *mut u32, verb
                    );
                    hda_w16(mmio, 0x48, verb_wp);
                    let mut t = 0u32;
                    while t < 200_000 {
                        if (hda_r16(mmio, 0x58) & 0x00FF) == verb_wp {
                            return Some(core::ptr::read_volatile(
                                (rirb_phys + (verb_wp as usize) * 8) as *const u32
                            ));
                        }
                        t += 1;
                        core::hint::spin_loop();
                    }
                    None
                };

                // Root node 0, Subordinate Node Count (parameter 0x04).
                let root_nodes = match send_verb(((codec as u32) << 28) | (0xF04u32 << 8)) {
                    Some(v) => v,
                    None => {
                        serial::write_str("[AUDIO] HDA ROOT_NODE_COUNT_TIMEOUT\n");
                        return;
                    }
                };
                let root_start = ((root_nodes >> 16) & 0xFFFF) as u16;
                let root_count = (root_nodes & 0xFFFF) as u16;
                serial::write_str("[AUDIO] HDA ROOT_NODES START=");
                serial::write_hex(root_start as usize);
                serial::write_str(" COUNT=");
                serial::write_hex(root_count as usize);
                serial::write_str("\n");

                // Scan only the returned root range, bounded to 64 nodes.
                let scan_count = (root_count as usize).min(64);
                let mut afg = 0u8;
                let mut node = root_start as usize;
                let mut scanned = 0usize;
                while scanned < scan_count {
                    let nid = node as u8;
                    let fg = match send_verb(
                        ((codec as u32) << 28) | ((nid as u32) << 20) | (0xF05u32 << 8)
                    ) {
                        Some(v) => v,
                        None => { node += 1; scanned += 1; continue; }
                    };
                    let fg_type = (fg & 0xFF) as u8;
                    serial::write_str("[AUDIO] HDA NODE=");
                    serial::write_hex(nid as usize);
                    serial::write_str(" TYPE=");
                    serial::write_hex(fg_type as usize);
                    serial::write_str("\n");
                    if fg_type == 1 && afg == 0 {
                        afg = nid;
                        HDA_AFG = nid;
                    }
                    node += 1;
                    scanned += 1;
                }

                if afg != 0 {
                    let afg_nodes = match send_verb(
                        ((codec as u32) << 28) | ((afg as u32) << 20) | (0xF04u32 << 8)
                    ) {
                        Some(v) => v,
                        None => {
                            serial::write_str("[AUDIO] HDA AFG_NODE_COUNT_TIMEOUT\n");
                            return;
                        }
                    };
                    serial::write_str("[AUDIO] HDA AFG=");
                    serial::write_hex(afg as usize);
                    serial::write_str(" WIDGET_START=");
                    serial::write_hex(((afg_nodes >> 16) & 0xFFFF) as usize);
                    serial::write_str(" WIDGET_COUNT=");
                    serial::write_hex((afg_nodes & 0xFFFF) as usize);
                    serial::write_str("\n");

                    // Bounded widget capability discovery. Do not program the
                    // codec yet: first identify a real analog output pin and
                    // an Audio Output converter.
                    let start = ((afg_nodes >> 16) & 0xFFFF) as usize;
                    let count = (afg_nodes & 0xFFFF) as usize;
                    let limit = count.min(64);
                    let mut analog_pin = 0u8;
                    let mut output_conv = 0u8;
                    let mut i = 0usize;
                    while i < limit {
                        let nid = (start + i) as u8;
                        let caps = match send_verb(
                            ((codec as u32) << 28) |
                            ((nid as u32) << 20) |
                            ((0xF00u32 << 8) | 0x09u32)
                        ) {
                            Some(v) => v,
                            None => { i += 1; continue; }
                        };
                        let wtype = ((caps >> 20) & 0xF) as u8;
                        serial::write_str("[AUDIO] HDA WIDGET NID=");
                        serial::write_hex(nid as usize);
                        serial::write_str(" TYPE=");
                        serial::write_hex(wtype as usize);
                        serial::write_str("\n");

                        // Audio Output widget type = 0.
                        if wtype == 0 && output_conv == 0 {
                            output_conv = nid;
                        }

                        // Pin Complex widget type = 4. Query pin caps and
                        // default configuration, but do not change state.
                        if wtype == 4 && analog_pin == 0 {
                            let pin_caps = match send_verb(
                                ((codec as u32) << 28) |
                                ((nid as u32) << 20) |
                                ((0xF00u32 << 8) | 0x0Cu32)
                            ) { Some(v) => v, None => 0 };
                            let output_capable = (pin_caps & (1 << 4)) != 0;
                            if output_capable {
                                let cfg = match send_verb(
                                    ((codec as u32) << 28) |
                                    ((nid as u32) << 20) |
                                    (0xF1Cu32 << 8)
                                ) { Some(v) => v, None => 0 };
                                let device = ((cfg >> 20) & 0xF) as u8;
                                // 0x0 = line out, 0x1 = speaker, 0x2 = HP out
                                // are useful analog endpoint candidates.
                                if device <= 2 {
                                    analog_pin = nid;
                                    HDA_ANALOG_PIN = nid;
                                    serial::write_str("[AUDIO] HDA ANALOG_PIN NID=");
                                    serial::write_hex(nid as usize);
                                    serial::write_str(" DEV=");
                                    serial::write_hex(device as usize);
                                    serial::write_str("\n");
                                }
                            }
                        }
                        i += 1;
                    }
                    // Prefer an output converter reachable from the selected analog pin.
                    // The codec graph is traversed backwards through connection-list widgets;
                    // this avoids hard-coding ALC269 NIDs while still staying bounded.
                    let mut selected_conv = output_conv;
                    if analog_pin != 0 {
                        let mut frontier = [0u8; 32];
                        let mut next_frontier = [0u8; 32];
                        let mut fcount = 1usize;
                        frontier[0] = analog_pin;
                        let mut depth = 0usize;
                        let mut found = 0u8;
                        while depth < 5 && fcount > 0 && found == 0 {
                            let mut nf = 0usize;
                            let mut fi = 0usize;
                            while fi < fcount && found == 0 {
                                let cur = frontier[fi];
                                let caps = match send_verb(((codec as u32)<<28)|((cur as u32)<<20)|(0xF00u32<<8)|0x09) {
                                    Some(v)=>v, None=>{fi+=1;continue}
                                };
                                let typ=((caps>>20)&0xF) as u8;
                                if typ==0 { found=cur; break; }
                                let lp=match send_verb(((codec as u32)<<28)|((cur as u32)<<20)|(0xF00u32<<8)|0x0E) {
                                    Some(v)=>v, None=>{fi+=1;continue}
                                };
                                let n=(lp&0xFF).min(16) as usize;
                                let mut ci=0usize;
                                while ci<n && nf<32 {
                                    let ent=match send_verb(((codec as u32)<<28)|((cur as u32)<<20)|(0xF02u32<<8)|((ci as u32)&0xFF)) {
                                        Some(v)=>v, None=>{ci+=1;continue}
                                    };
                                    let cn=(ent&0x7F) as u8;
                                    if cn==0 {ci+=1;continue}
                                    if cn==output_conv {found=cn;break;}
                                    let mut dup=false; let mut x=0usize;
                                    while x<nf {if next_frontier[x]==cn{dup=true;break;} x+=1;}
                                    if !dup {next_frontier[nf]=cn;nf+=1;}
                                    ci+=1;
                                }
                                fi+=1;
                            }
                            let mut x=0usize;
                            while x<nf {frontier[x]=next_frontier[x];x+=1;}
                            fcount=nf;
                            depth+=1;
                        }
                        if found!=0 { selected_conv=found; }
                    }
                    output_conv=selected_conv;
                    HDA_OUTPUT_CONV = output_conv;

                serial::write_str("[AUDIO] HDA OUTPUT_CANDIDATES PIN=";
                    serial::write_hex(analog_pin as usize);
                    serial::write_str(" CONV=");
                    serial::write_hex(output_conv as usize);
                    serial::write_str("\n");

                    if analog_pin != 0 && output_conv != 0 {
                        // First bring up the codec endpoint.  The converter gets
                        // stream tag 1/channel 0 and 44.1k/16-bit PCM (0x4011);
                        // pin control 0x40 enables output.  These are the standard
                        // HDA stream/codec programming steps, leaving routing
                        // discovery data-driven rather than ALC269-hardcoded.
                        let codec_stream = ((codec as u32)<<28)|((output_conv as u32)<<20);
                        let _ = send_verb(codec_stream | (0x705u32<<8)); // D0
                        let _ = send_verb(codec_stream | (0x706u32<<8) | 0x10); // tag=1,ch=0
                        let _ = send_verb(codec_stream | (0x200u32<<8) | 0x11); // 16-bit stereo/mono fmt low bits
                        let pin_cmd = ((codec as u32)<<28)|((analog_pin as u32)<<20);
                        let _ = send_verb(pin_cmd | (0x707u32<<8) | 0x40); // output enable
                        let _ = send_verb(pin_cmd | (0x705u32<<8)); // D0
                        let _ = send_verb(pin_cmd | (0x300u32<<8) | 0xA000); // output amp, unmuted, gain 0
                        let conv_cmd = ((codec as u32)<<28)|((output_conv as u32)<<20);
                        let _ = send_verb(conv_cmd | (0x300u32<<8) | 0xA000);
                        serial::write_str("[AUDIO] HDA ANALOG PATH PROGRAMMED PIN=");
                        serial::write_hex(analog_pin as usize);
                        serial::write_str(" CONV=");
                        serial::write_hex(output_conv as usize);
                        serial::write_str("\n");
                    }

                    // Select the first output stream exposed by GCAP.
                    let iss = ((gcap >> 8) & 0x0F) as usize;
                    let bss = ((gcap >> 3) & 0x1F) as usize;
                    let oss = ((gcap >> 12) & 0x0F) as usize;
                    if oss == 0 {
                        serial::write_str("[AUDIO] HDA NO_OUTPUT_STREAM\n");
                    } else {
                        HDA_STREAM_BASE = 0x80 + iss * 0x20;
                        HDA_STREAM_READY = true;
                        HDA_STREAM_RUNNING = false;
                        HDA_STREAM_FMT = 0x4011;
                        serial::write_str("[AUDIO] HDA OUTPUT_STREAM BASE=");
                        serial::write_hex(HDA_STREAM_BASE);
                        serial::write_str(" OSS=");
                        serial::write_hex(oss);
                        serial::write_str("\n");
                    }
                } else {
                    serial::write_str("[AUDIO] HDA AFG_NOT_FOUND\n");
                }

                if HDA_STREAM_READY {
                    serial::write_str("[AUDIO] HDA PROBE PASS; PCM stream descriptor selected\n");
                } else if HDA_AFG == 0 {
                    serial::write_str("[AUDIO] HDA PROBE PARTIAL; AFG_NOT_FOUND\n");
                } else if HDA_ANALOG_PIN == 0 {
                    serial::write_str("[AUDIO] HDA PROBE PARTIAL; NO_ANALOG_PIN\n");
                } else if HDA_OUTPUT_CONV == 0 {
                    serial::write_str("[AUDIO] HDA PROBE PARTIAL; NO_OUTPUT_CONVERTER\n");
                } else {
                    serial::write_str("[AUDIO] HDA PROBE PARTIAL; NO_OUTPUT_STREAM\n");
                }
                return;
            }
        }
        serial::write_str("[AUDIO] HDA controller not found on bus0\n");
    }
}


unsafe fn dma_map(phys: usize, pages: usize) -> bool {
    let cr3=crate::mm::paging::read_cr3();
    let mut p=phys;
    let end=phys+pages*0x1000;
    while p<end {
        if !crate::mm::paging::map_page(cr3,p,p,
            crate::mm::paging::PAGE_PRESENT|crate::mm::paging::PAGE_WRITE|
            crate::mm::paging::PAGE_PCD|crate::mm::paging::PAGE_PWT) { return false; }
        p+=0x1000;
    }
    crate::mm::paging::load_cr3(cr3);
    true
}

fn stream_format(rate:u32,ch:u16,bits:u16)->Option<u16>{
    if ch<1 || ch>2 || bits!=16 { return None; }
    if rate==44100 { Some(0x4010u16 | (ch-1)) }
    else if rate==48000 { Some(0x0010u16 | (ch-1)) }
    else { None }
}

pub fn playback_start(rate:u32,ch:u16,bits:u16)->bool {
    unsafe {
        if !HDA_STREAM_READY { serial::write_str("[AUDIO] PLAYBACK_NO_STREAM\n"); return false; }
        let fmt=match stream_format(rate,ch,bits){Some(v)=>v,None=>{serial::write_str("[AUDIO] PLAYBACK_FORMAT_UNSUPPORTED\n");return false;}};
        if HDA_STREAM_RUNNING { playback_stop(); }
        let total_pages=5usize;
        let phys=match crate::mm::alloc_pages(total_pages){Some(p)=>p,None=>{serial::write_str("[AUDIO] PLAYBACK_DMA_ALLOC_FAIL\n");return false;}};
        if !dma_map(phys,total_pages) {
            serial::write_str("[AUDIO] PLAYBACK_DMA_MAP_FAIL\n"); return false;
        }
        crate::mm::zero_pages(phys, total_pages);
        HDA_DMA_PHYS=phys;
        HDA_BDL_PHYS=phys+4*0x1000;
        HDA_DMA_PERIOD=4096;
        HDA_DMA_PERIODS=4;
        HDA_DMA_NEXT=0;
        HDA_DMA_TOTAL=0;
        HDA_DMA_LAST_LPIB=0;
        HDA_STREAM_FMT=fmt;

        let sd=HDA_STREAM_BASE;
        // Stream reset is bit 0; RUN is bit 1.  Reset before programming
        // CBL/LVI/FMT/BDL, then verify the reset clears.
        hda_w32(HDA_BAR0 as usize,sd,1);
        let mut t=0u32; while t<10000 && (hda_r32(HDA_BAR0 as usize,sd)&1)==0 {t+=1;core::hint::spin_loop();}
        hda_w32(HDA_BAR0 as usize,sd,0);
        t=0; while t<10000 && (hda_r32(HDA_BAR0 as usize,sd)&1)!=0 {t+=1;core::hint::spin_loop();}
        hda_w32(HDA_BAR0 as usize,sd+0x08,0);
        hda_w16(HDA_BAR0 as usize,sd+0x0C,3);
        hda_w16(HDA_BAR0 as usize,sd+0x12,fmt);
        hda_w32(HDA_BAR0 as usize,sd+0x08,(HDA_DMA_PERIOD*HDA_DMA_PERIODS) as u32);
        hda_w16(HDA_BAR0 as usize,sd+0x0C,3);

        let mut i=0usize;
        while i<4 {
            let got=crate::media_player::pcm_buffer(
                core::slice::from_raw_parts_mut((HDA_DMA_PHYS+i*4096) as *mut u8,4096));
            if got==0 { break; }
            HDA_DMA_TOTAL+=got;
            crate::media_player::consume_pcm(got);
            i+=1;
        }
        if i==0 { serial::write_str("[AUDIO] PLAYBACK_NO_PCM\n"); return false; }
        let mut j=0usize;
        while j<4 {
            let e=HDA_BDL_PHYS+j*16;
            core::ptr::write_volatile(e as *mut u64,(HDA_DMA_PHYS+j*4096) as u64);
            core::ptr::write_volatile((e+8) as *mut u32,4096u32);
            core::ptr::write_volatile((e+12) as *mut u32,1u32);
            j+=1;
        }
        hda_w32(HDA_BAR0 as usize,sd+0x18,HDA_BDL_PHYS as u32);
        hda_w32(HDA_BAR0 as usize,sd+0x1C,(HDA_BDL_PHYS>>32) as u32);
        hda_w8(HDA_BAR0 as usize,sd+0x03,0x1C); // clear BCIS/FIFOE/DESE
        hda_w32(HDA_BAR0 as usize,sd+0x04,(4096*4) as u32);
        hda_w16(HDA_BAR0 as usize,sd+0x0C,3);
        let ctl=(HDA_STREAM_TAG as u32)<<20;
        hda_w32(HDA_BAR0 as usize,sd,ctl | 0x00000002); // RUN=1
        HDA_STREAM_RUNNING=true;
        serial::write_str("[AUDIO] HDA PLAY START FMT=");
        serial::write_hex(fmt as usize); serial::write_str(" BDL="); serial::write_hex(HDA_BDL_PHYS);
        serial::write_str(" DATA="); serial::write_hex(HDA_DMA_TOTAL); serial::write_str("\n");
        true
    }
}

pub fn playback_stop() {
    unsafe {
        if !HDA_STREAM_RUNNING { return; }
        let sd=HDA_STREAM_BASE;
        let base=HDA_BAR0 as usize;
        hda_w32(base,sd, hda_r32(base,sd) & !0x2);
        let mut t=0u32; while t<10000 && (hda_r32(base,sd)&0x2)!=0 {t+=1;core::hint::spin_loop();}
        HDA_STREAM_RUNNING=false;
        serial::write_str("[AUDIO] HDA PLAY STOP LPIB=");
        serial::write_hex(hda_r32(base,sd+0x04) as usize); serial::write_str("\n");
    }
}

pub fn playback_poll() {
    unsafe {
        if !HDA_STREAM_RUNNING { return; }
        let base=HDA_BAR0 as usize;
        let sd=HDA_STREAM_BASE;
        let lp=hda_r32(base,sd+0x04);
        let period=HDA_DMA_PERIOD as u32;
        let cbl=(HDA_DMA_PERIOD*HDA_DMA_PERIODS) as u32;

        // LPIB is the cyclic position. Refill every period that the DMA
        // has fully passed; when LPIB wraps, finish the tail periods first.
        if lp < HDA_DMA_LAST_LPIB {
            while HDA_DMA_NEXT < HDA_DMA_PERIODS {
                let slot=HDA_DMA_NEXT;
                let got=crate::media_player::pcm_buffer(
                    core::slice::from_raw_parts_mut((HDA_DMA_PHYS+slot*4096) as *mut u8,4096));
                if got==0 {
                    hda_w32(base,sd,hda_r32(base,sd)&!0x2);
                    HDA_STREAM_RUNNING=false;
                    serial::write_str("[AUDIO] HDA PLAY EOF\n");
                    return;
                }
                crate::media_player::consume_pcm(got);
                HDA_DMA_TOTAL+=got;
                HDA_DMA_NEXT+=1;
            }
            HDA_DMA_NEXT=0;
        }

        let completed=((lp/period) as usize).min(HDA_DMA_PERIODS);
        while HDA_DMA_NEXT < completed {
            let slot=HDA_DMA_NEXT;
            let got=crate::media_player::pcm_buffer(
                core::slice::from_raw_parts_mut((HDA_DMA_PHYS+slot*4096) as *mut u8,4096));
            if got==0 {
                hda_w32(base,sd,hda_r32(base,sd)&!0x2);
                HDA_STREAM_RUNNING=false;
                serial::write_str("[AUDIO] HDA PLAY EOF\n");
                return;
            }
            crate::media_player::consume_pcm(got);
            HDA_DMA_TOTAL+=got;
            HDA_DMA_NEXT+=1;
        }
        if HDA_DMA_NEXT>=HDA_DMA_PERIODS && lp>=cbl {
            HDA_DMA_NEXT=0;
        }
        HDA_DMA_LAST_LPIB=lp;
    }
}

pub fn init() {
    unsafe { SPEAKER_OK = true; }
    serial::write_str("[AUDIO] native HDA foundation init\n");
    probe_hda();
}
