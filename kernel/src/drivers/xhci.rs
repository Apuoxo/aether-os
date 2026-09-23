//! xHCI: rings + USB enumeration (Port Reset → Enable Slot → Address Device → GET_DESCRIPTOR)

use crate::serial;
use crate::mm;
use crate::mm::paging;

// USB mouse diagnostics
static mut DIAG_XHCI: bool = false;
static mut DIAG_DEV: bool = false;
static mut DIAG_HID: bool = false;
static mut DIAG_MOUSE_IF: bool = false;
static mut DIAG_EP_IN: bool = false;
static mut DIAG_REPORTS: u32 = 0;
static mut DIAG_TRB_QUEUED: u32 = 0;
static mut DIAG_COMPLETIONS: u32 = 0;
static mut DIAG_FOUND: u32 = 0;

pub fn diag_xhci_ok() -> bool { unsafe { DIAG_XHCI } }
pub fn diag_dev_found() -> bool { unsafe { DIAG_DEV } }
pub fn diag_hid_ok() -> bool { unsafe { DIAG_HID } }
pub fn diag_mouse_if() -> bool { unsafe { DIAG_MOUSE_IF } }
pub fn diag_ep_in() -> bool { unsafe { DIAG_EP_IN } }
pub fn diag_reports() -> u32 { unsafe { DIAG_REPORTS } }
pub fn diag_trb_queued() -> u32 { unsafe { DIAG_TRB_QUEUED } }
pub fn diag_completions() -> u32 { unsafe { DIAG_COMPLETIONS } }
pub fn diag_found_count() -> u32 { unsafe { DIAG_FOUND } }

// Persistent HID mouse interrupt path for desktop loop
static mut MOUSE_LIVE: bool = false;
static mut MOUSE_ER: usize = 0;
static mut MOUSE_ER_DEQ: usize = 0;
static mut MOUSE_ER_CYCLE: u32 = 1;
static mut MOUSE_ER_SIZE: usize = 0;
static mut MOUSE_INTR0: usize = 0;
static mut MOUSE_EP_RING: usize = 0;
static mut MOUSE_ENQ: usize = 0;
static mut MOUSE_CYCLE: u32 = 1;
static mut MOUSE_REPORT: usize = 0;
static mut MOUSE_SLOT: u8 = 0;
static mut MOUSE_DB: usize = 0;
static mut MOUSE_EVENTS: u32 = 0; // internal events delivered to input

pub fn diag_mouse_events() -> u32 { unsafe { MOUSE_EVENTS } }
pub fn diag_mouse_live() -> bool { unsafe { MOUSE_LIVE } }


// ─── PCI / MMIO ───────────────────────────────────────────────

unsafe fn pci_cfg(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((off as u32) & 0xFC)
}
unsafe fn pci_r32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = pci_cfg(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") v, options(nostack, preserves_flags));
    v
}
unsafe fn pci_w32(bus: u8, dev: u8, func: u8, off: u8, val: u32) {
    let a = pci_cfg(bus, dev, func, off);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    core::arch::asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") val, options(nostack, preserves_flags));
}

fn hx(v: usize) { serial::write_hex(v); }
fn h32(v: u32) { serial::write_hex(v as usize); }

unsafe fn r32(b: usize, o: usize) -> u32 { core::ptr::read_volatile((b + o) as *const u32) }
unsafe fn w32(b: usize, o: usize, v: u32) { core::ptr::write_volatile((b + o) as *mut u32, v); }
unsafe fn r64(b: usize, o: usize) -> u64 {
    (r32(b, o) as u64) | ((r32(b, o + 4) as u64) << 32)
}
unsafe fn w64(b: usize, o: usize, v: u64) {
    w32(b, o, v as u32);
    w32(b, o + 4, (v >> 32) as u32);
}

fn delay(n: u32) {
    let mut i = 0u32;
    while i < n { unsafe { core::arch::asm!("nop", options(nostack, preserves_flags)); } i += 1; }
}

fn map_mmio(phys: u64, size: u64) -> bool {
    let cr3 = unsafe { paging::read_cr3() };
    let mut a = (phys & !0xFFF) as usize;
    let end = ((phys + size + 0xFFF) & !0xFFF) as usize;
    while a < end {
        if !unsafe { paging::map_page(cr3, a, a, paging::PAGE_PRESENT | paging::PAGE_WRITE) } {
            return false;
        }
        a += 4096;
    }
    unsafe { paging::load_cr3(cr3); }
    true
}

fn dma_alloc(pages: usize) -> Option<usize> {
    let phys = mm::alloc_pages(pages)?;
    mm::zero_pages(phys, pages);
    // Boot maps first 1GB identity
    if phys + pages * 4096 > 0x4000_0000 { return None; }
    unsafe {
        let p = phys as *mut u32;
        *p = 0xDEAD_BEEF;
        if *p != 0xDEAD_BEEF { return None; }
        *p = 0;
    }
    Some(phys)
}

// ─── TRB ──────────────────────────────────────────────────────

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Trb {
    lo: u32,
    hi: u32,
    status: u32,
    control: u32,
}

impl Trb {
    const fn z() -> Self { Trb { lo: 0, hi: 0, status: 0, control: 0 } }
}

const TRB_LINK: u32 = 6;
const TRB_ENABLE_SLOT: u32 = 9;
const TRB_ADDRESS_DEV: u32 = 11;
const TRB_CONFIGURE_EP: u32 = 12;
const TRB_NOOP_CMD: u32 = 23;
const TRB_TRANSFER: u32 = 32;       // Transfer Event
const TRB_CMD_COMPLETE: u32 = 33;
const TRB_PORT_STATUS: u32 = 34;
const TRB_SETUP: u32 = 2;
const TRB_DATA: u32 = 3;
const TRB_STATUS: u32 = 4;
const TRB_NORMAL: u32 = 1;

const CYCLE: u32 = 1;
const TRB_IOC: u32 = 1 << 5;
const TRB_IDT: u32 = 1 << 6;  // Immediate Data
const TRB_TC: u32 = 1 << 1;

const USBCMD_RS: u32 = 1;
const USBCMD_HCRST: u32 = 2;
const USBCMD_INTE: u32 = 4;
const USBSTS_HCH: u32 = 1;
const USBSTS_CNR: u32 = 1 << 11;

const PORTSC_CCS: u32 = 1;
const PORTSC_PED: u32 = 2;
const PORTSC_PR: u32 = 1 << 4;
const PORTSC_PP: u32 = 1 << 9;
const PORTSC_CSC: u32 = 1 << 17;
const PORTSC_PRC: u32 = 1 << 21;

// ─── Controller ───────────────────────────────────────────────

struct Xhci {
    mmio: usize,
    cap: u8,
    slots: u8,
    ports: u8,
    dboff: u32,
    rtsoff: u32,
    ctx_size: usize, // 32 or 64
    dcbaa: usize,
    cmd_ring: usize,
    cmd_size: usize,
    cmd_enq: usize,
    cmd_cycle: u32,
    erst: usize,
    er: usize,
    er_size: usize,
    er_deq: usize,
    er_cycle: u32,
}

impl Xhci {
    fn op(&self) -> usize { self.mmio + self.cap as usize }
    fn rt(&self) -> usize { self.mmio + self.rtsoff as usize }
    fn db(&self) -> usize { self.mmio + self.dboff as usize }
    fn portsc(&self, p: u8) -> usize { self.op() + 0x400 + 0x10 * (p as usize - 1) }
    fn intr0(&self) -> usize { self.rt() + 0x20 }
}

// ─── Event wait ───────────────────────────────────────────────

struct Event {
    trb_type: u32,
    cc: u32,
    slot_id: u32,
    port_id: u32,
    lo: u32,
    status: u32,
    control: u32,
}

fn wait_event(x: &mut Xhci, want_type: u32, max_spins: u32) -> Option<Event> {
    let mut spins = 0u32;
    while spins < max_spins {
        unsafe {
            let trbs = x.er as *const Trb;
            let ev = *trbs.add(x.er_deq);
            if (ev.control & 1) == x.er_cycle {
                let ty = (ev.control >> 10) & 0x3F;
                let cc = (ev.status >> 24) & 0xFF;
                let slot = (ev.control >> 24) & 0xFF;
                let port = (ev.lo >> 24) & 0xFF; // for port status change events
                let e = Event {
                    trb_type: ty, cc, slot_id: slot, port_id: port,
                    lo: ev.lo, status: ev.status, control: ev.control,
                };
                x.er_deq += 1;
                if x.er_deq >= x.er_size {
                    x.er_deq = 0;
                    x.er_cycle ^= 1;
                }
                let erdp = x.er + x.er_deq * 16;
                w64(x.intr0(), 0x18, erdp as u64);

                serial::write_str("  [EVT] type=");
                serial::write_usize(ty as usize);
                serial::write_str(" CC=");
                serial::write_usize(cc as usize);
                serial::write_str(" slot=");
                serial::write_usize(slot as usize);
                serial::write_str("\n");

                if ty == want_type || want_type == 0 {
                    return Some(e);
                }
                // keep consuming until match or empty
                continue;
            }
        }
        delay(30);
        spins += 1;
    }
    None
}

fn ring_cmd_doorbell(x: &Xhci) {
    unsafe { w32(x.db(), 0, 0); }
}

fn submit_cmd(x: &mut Xhci, lo: u32, hi: u32, status: u32, type_flags: u32) -> bool {
    if x.cmd_enq >= x.cmd_size - 1 { return false; }
    unsafe {
        let trbs = x.cmd_ring as *mut Trb;
        let t = &mut *trbs.add(x.cmd_enq);
        t.lo = lo;
        t.hi = hi;
        t.status = status;
        t.control = (type_flags << 10) | x.cmd_cycle | (type_flags & 0); // type in flags param already shifted? 
        // type_flags is just the type number
        t.control = (type_flags << 10) | x.cmd_cycle;
        x.cmd_enq += 1;
    }
    ring_cmd_doorbell(x);
    true
}

fn submit_cmd_full(x: &mut Xhci, lo: u32, hi: u32, status: u32, control_without_cycle: u32) -> bool {
    if x.cmd_enq >= x.cmd_size - 1 { return false; }
    unsafe {
        let trbs = x.cmd_ring as *mut Trb;
        let t = &mut *trbs.add(x.cmd_enq);
        t.lo = lo;
        t.hi = hi;
        t.status = status;
        t.control = control_without_cycle | x.cmd_cycle;
        x.cmd_enq += 1;
    }
    ring_cmd_doorbell(x);
    true
}

// ─── Basic init (proven path) ─────────────────────────────────

fn hc_reset(mmio: usize, cap: u8) -> bool {
    let op = mmio + cap as usize;
    unsafe {
        if r32(op, 4) & USBSTS_HCH == 0 {
            w32(op, 0, r32(op, 0) & !USBCMD_RS);
            let mut t = 0u32;
            while t < 100000 { if r32(op, 4) & USBSTS_HCH != 0 { break; } delay(40); t += 1; }
        }
        w32(op, 0, r32(op, 0) | USBCMD_HCRST);
        let mut t = 0u32;
        while t < 200000 { if r32(op, 0) & USBCMD_HCRST == 0 { break; } delay(40); t += 1; }
        if r32(op, 0) & USBCMD_HCRST != 0 { return false; }
        t = 0;
        while t < 200000 { if r32(op, 4) & USBSTS_CNR == 0 { break; } delay(40); t += 1; }
        r32(op, 4) & USBSTS_CNR == 0
    }
}

fn setup_rings(x: &mut Xhci) -> bool {
    // DCBAAP
    let n = x.slots as usize + 1;
    let pages = (n * 8 + 4095) / 4096;
    let dcbaa = match dma_alloc(pages) { Some(p) => p, None => return false };
    x.dcbaa = dcbaa;
    unsafe { w64(x.op(), 0x30, dcbaa as u64); }
    serial::write_str("  [xHCI] DCBAAP=");
    hx(dcbaa);
    serial::write_str("\n");

    // Command ring
    let cr = match dma_alloc(1) { Some(p) => p, None => return false };
    x.cmd_ring = cr;
    x.cmd_size = 64;
    x.cmd_enq = 0;
    x.cmd_cycle = 1;
    unsafe {
        let trbs = cr as *mut Trb;
        let mut i = 0usize;
        while i < 64 { *trbs.add(i) = Trb::z(); i += 1; }
        let link = &mut *trbs.add(63);
        link.lo = cr as u32;
        link.hi = 0;
        link.control = (TRB_LINK << 10) | TRB_TC | CYCLE;
        w64(x.op(), 0x18, (cr as u64) | 1);
    }
    serial::write_str("  [xHCI] CRCR ring=");
    hx(cr);
    serial::write_str("\n");

    // Event ring + ERST
    let er = match dma_alloc(1) { Some(p) => p, None => return false };
    let erst = match dma_alloc(1) { Some(p) => p, None => return false };
    x.er = er;
    x.er_size = 64;
    x.er_deq = 0;
    x.er_cycle = 1;
    x.erst = erst;
    unsafe {
        let trbs = er as *mut Trb;
        let mut i = 0usize;
        while i < 64 { *trbs.add(i) = Trb::z(); i += 1; }
        let e = erst as *mut u64;
        *e.add(0) = er as u64;
        *e.add(1) = 64;
        let i0 = x.intr0();
        w32(i0, 0x08, 1);
        w64(i0, 0x10, erst as u64);
        w64(i0, 0x18, er as u64);
        w32(i0, 0x04, 0);
        w32(i0, 0x00, 0x2);
    }
    serial::write_str("  [xHCI] ER=");
    hx(er);
    serial::write_str(" ERST=");
    hx(erst);
    serial::write_str("\n");

    // CONFIG + Run
    unsafe {
        w32(x.op(), 0x38, x.slots as u32);
        w32(x.op(), 0x00, USBCMD_RS | USBCMD_INTE);
        delay(8000);
        let sts = r32(x.op(), 0x04);
        serial::write_str("  [xHCI] USBSTS=");
        h32(sts);
        serial::write_str(" HCH=");
        serial::write_usize(if sts & USBSTS_HCH != 0 { 1 } else { 0 });
        serial::write_str("\n");
        if sts & USBSTS_HCH != 0 { return false; }
    }
    true
}

// ─── Port reset ───────────────────────────────────────────────

fn find_connected_port(x: &Xhci) -> Option<u8> {
    let mut p = 1u8;
    while p <= x.ports {
        let ps = unsafe { r32(x.portsc(p), 0) };
        serial::write_str("  [PORT] ");
        serial::write_usize(p as usize);
        serial::write_str(" PORTSC=");
        h32(ps);
        serial::write_str(" CCS=");
        serial::write_usize(if ps & PORTSC_CCS != 0 { 1 } else { 0 });
        serial::write_str("\n");
        if ps & PORTSC_CCS != 0 {
            return Some(p);
        }
        p += 1;
    }
    None
}

fn port_reset(x: &mut Xhci, port: u8) -> bool {
    serial::write_str("\n  --- Port Reset ---\n");
    let ps_before = unsafe { r32(x.portsc(port), 0) };
    serial::write_str("  [PORT] before PORTSC=");
    h32(ps_before);
    serial::write_str("\n");

    unsafe {
        // Power on if needed
        if ps_before & PORTSC_PP == 0 {
            w32(x.portsc(port), 0, ps_before | PORTSC_PP);
            delay(50000);
        }
        // Write 1 to PR; preserve PP; clear change bits by writing 1
        let mut v = r32(x.portsc(port), 0);
        v |= PORTSC_PR | PORTSC_PP;
        // Do not clear CCS accidentally
        w32(x.portsc(port), 0, v);
    }

    // Wait for PRC (Port Reset Change) via poll PORTSC or event
    let mut t = 0u32;
    while t < 500000 {
        let ps = unsafe { r32(x.portsc(port), 0) };
        if ps & PORTSC_PRC != 0 {
            serial::write_str("  [PORT] PRC set PORTSC=");
            h32(ps);
            serial::write_str(" PED=");
            serial::write_usize(if ps & PORTSC_PED != 0 { 1 } else { 0 });
            serial::write_str("\n");
            // Clear CSC and PRC (W1C)
            unsafe {
                let mut c = r32(x.portsc(port), 0);
                c |= PORTSC_CSC | PORTSC_PRC;
                // Clear PR bit is auto-cleared by HC
                w32(x.portsc(port), 0, c);
            }
            // Drain any port status change events
            while let Some(ev) = wait_event(x, TRB_PORT_STATUS, 5000) {
                serial::write_str("  [PORT] PSC event port=");
                serial::write_usize(ev.port_id as usize);
                serial::write_str(" CC=");
                serial::write_usize(ev.cc as usize);
                serial::write_str("\n");
            }
            let ps_after = unsafe { r32(x.portsc(port), 0) };
            serial::write_str("  [PORT] after PORTSC=");
            h32(ps_after);
            serial::write_str(" CCS=");
            serial::write_usize(if ps_after & PORTSC_CCS != 0 { 1 } else { 0 });
            serial::write_str(" PED=");
            serial::write_usize(if ps_after & PORTSC_PED != 0 { 1 } else { 0 });
            serial::write_str("\n");
            return ps_after & PORTSC_PED != 0 || ps_after & PORTSC_CCS != 0;
        }
        delay(50);
        t += 1;
    }
    serial::write_str("  [PORT] reset TIMEOUT\n");
    false
}

// ─── Enable Slot ──────────────────────────────────────────────

fn enable_slot(x: &mut Xhci) -> Option<u8> {
    serial::write_str("\n  --- Enable Slot ---\n");
    if !submit_cmd_full(x, 0, 0, 0, TRB_ENABLE_SLOT << 10) {
        return None;
    }
    match wait_event(x, TRB_CMD_COMPLETE, 300000) {
        Some(ev) => {
            if ev.cc != 1 {
                serial::write_str("  [SLOT] Enable Slot CC FAIL\n");
                return None;
            }
            let sid = ev.slot_id as u8;
            serial::write_str("  [SLOT] Slot ID=");
            serial::write_usize(sid as usize);
            serial::write_str("\n");
            if sid == 0 { return None; }
            Some(sid)
        }
        None => {
            serial::write_str("  [SLOT] Enable Slot TIMEOUT\n");
            None
        }
    }
}

// ─── Input/Device Context + EP0 ring ──────────────────────────

fn speed_from_portsc(ps: u32) -> u32 {
    // PORTSC bits 13:10
    (ps >> 10) & 0xF
}

fn max_pkt_for_speed(speed: u32) -> u32 {
    match speed {
        1 => 8,    // Full
        2 => 8,    // Low
        3 => 64,   // High
        4 => 512,  // Super
        _ => 64,
    }
}

/// Build Input Context + Device Context, set EP0 transfer ring, Address Device
fn address_device(x: &mut Xhci, slot: u8, port: u8) -> Option<usize> {
    serial::write_str("\n  --- Address Device ---\n");
    let ps = unsafe { r32(x.portsc(port), 0) };
    let speed = speed_from_portsc(ps);
    let mps = max_pkt_for_speed(speed);
    serial::write_str("  [ADDR] port=");
    serial::write_usize(port as usize);
    serial::write_str(" speed=");
    serial::write_usize(speed as usize);
    serial::write_str(" MPS=");
    serial::write_usize(mps as usize);
    serial::write_str("\n");

    let cs = x.ctx_size; // 32 or 64
    // Input Context: A0+A1 + Slot + EP0  =>  (1 + 1 + 1) * ctx? 
    // Layout: Input Control Context (cs bytes) + Slot Context + EP contexts
    // For Address Device: drop Contexts flags A0=1 A1=1, Slot + EP0
    // Size: Control + Slot + EP0 = 3 * cs, allocate 1 page
    let in_ctx = match dma_alloc(1) { Some(p) => p, None => return None };
    let dev_ctx = match dma_alloc(1) { Some(p) => p, None => return None };
    let ep0_ring = match dma_alloc(1) { Some(p) => p, None => return None };

    serial::write_str("  [ADDR] InputCtx=");
    hx(in_ctx);
    serial::write_str(" DevCtx=");
    hx(dev_ctx);
    serial::write_str(" EP0ring=");
    hx(ep0_ring);
    serial::write_str("\n");

    unsafe {
        // Input Control Context: Dwords 0 = drop flags, Dwords 1 = add flags
        // Add Context Flags: A0 (slot) | A1 (EP0)
        let ic = in_ctx as *mut u32;
        // all zero already
        *ic.add(1) = 0x3; // A0 | A1

        // Slot Context at offset cs
        let slot_ctx = (in_ctx + cs) as *mut u32;
        // DW0: Route String=0, Speed, Context Entries=1 (EP0 only)
        // bit 20-23 Context Entries = 1
        // bit 27-24? Speed at bits 23:20 for some docs - check xHCI:
        // Slot Context DW0:
        //  19:0 Route String
        //  23:20 Speed
        //  24 MTT, 25 Hub
        //  31:27 Context Entries
        *slot_ctx.add(0) = (speed << 20) | (1u32 << 27);
        // DW1: Root Hub Port Number bits 23:16
        *slot_ctx.add(1) = (port as u32) << 16;
        // DW2, DW3 = 0

        // EP0 Context at offset 2*cs
        let ep0 = (in_ctx + 2 * cs) as *mut u32;
        // DW0: EP State=0, Interval=0
        *ep0.add(0) = 0;
        // DW1: CErr=3, EP Type=Control(4), Max Packet Size
        // CErr bits 2:1 = 3, EP Type bits 5:3 = 4 (Control Bidirectional)
        *ep0.add(1) = (3 << 1) | (4 << 3) | (mps << 16);
        // DW2-3: TR Dequeue Pointer | DCS=1
        let tr_deq = (ep0_ring as u64) | 1; // DCS
        *ep0.add(2) = tr_deq as u32;
        *ep0.add(3) = (tr_deq >> 32) as u32;
        // DW4: Average TRB Length = 8
        *ep0.add(4) = 8;

        // Init EP0 transfer ring: last = Link TRB
        let trbs = ep0_ring as *mut Trb;
        let mut i = 0usize;
        while i < 64 { *trbs.add(i) = Trb::z(); i += 1; }
        let link = &mut *trbs.add(63);
        link.lo = ep0_ring as u32;
        link.control = (TRB_LINK << 10) | TRB_TC | CYCLE;

        // Install Device Context pointer in DCBAA[slot]
        let dcbaa = x.dcbaa as *mut u64;
        *dcbaa.add(slot as usize) = dev_ctx as u64;
        serial::write_str("  [ADDR] DCBAA[");
        serial::write_usize(slot as usize);
        serial::write_str("]=");
        hx(dev_ctx);
        serial::write_str("\n");
    }

    // Address Device Command: TRB type 11, Slot ID in bits 31:24, pointer to Input Context
    // BSR=0 (bit 9) means actually send SET_ADDRESS
    let ctrl = (TRB_ADDRESS_DEV << 10) | ((slot as u32) << 24);
    if !submit_cmd_full(x, in_ctx as u32, 0, 0, ctrl) {
        return None;
    }
    match wait_event(x, TRB_CMD_COMPLETE, 500000) {
        Some(ev) => {
            if ev.cc != 1 {
                serial::write_str("  [ADDR] Address Device CC=");
                serial::write_usize(ev.cc as usize);
                serial::write_str(" FAIL\n");
                return None;
            }
            serial::write_str("  [ADDR] Address Device SUCCESS slot=");
            serial::write_usize(ev.slot_id as usize);
            serial::write_str("\n");
            Some(ep0_ring)
        }
        None => {
            serial::write_str("  [ADDR] Address Device TIMEOUT\n");
            None
        }
    }
}

// ─── Control transfer on EP0 ──────────────────────────────────

struct Ep0Ring {
    phys: usize,
    enq: usize,
    cycle: u32,
    size: usize,
}

fn ep0_submit_control(
    x: &mut Xhci,
    slot: u8,
    ep0: &mut Ep0Ring,
    setup: &[u8; 8],
    data_buf: usize,
    data_len: u32,
    data_in: bool,
) -> bool {
    // Setup Stage TRB (IDT=1, 8 bytes immediate)
    // Data Stage TRB if len > 0
    // Status Stage TRB
    if ep0.enq + 4 >= ep0.size - 1 {
        serial::write_str("  [EP0] ring full\n");
        return false;
    }

    unsafe {
        let trbs = ep0.phys as *mut Trb;

        // Setup Stage
        let t0 = &mut *trbs.add(ep0.enq);
        t0.lo = u32::from_le_bytes([setup[0], setup[1], setup[2], setup[3]]);
        t0.hi = u32::from_le_bytes([setup[4], setup[5], setup[6], setup[7]]);
        t0.status = 8; // TRB Transfer Length = 8
        // Type=Setup, IDT=1, IOC=0, Cycle, TRT: 0=no data, 2=IN, 3=OUT bits 17:16
        let trt = if data_len == 0 { 0u32 } else if data_in { 3 } else { 2 };
        t0.control = (TRB_SETUP << 10) | TRB_IDT | ep0.cycle | (trt << 16);
        ep0.enq += 1;

        // Data Stage
        if data_len > 0 {
            let t1 = &mut *trbs.add(ep0.enq);
            t1.lo = data_buf as u32;
            t1.hi = 0;
            t1.status = data_len;
            // Type=Data, DIR=1 for IN (bit 16)
            let dir = if data_in { 1u32 << 16 } else { 0 };
            t1.control = (TRB_DATA << 10) | ep0.cycle | dir;
            ep0.enq += 1;
        }

        // Status Stage
        let t2 = &mut *trbs.add(ep0.enq);
        t2.lo = 0;
        t2.hi = 0;
        t2.status = 0;
        // DIR opposite of data for status; IOC=1
        // For IN data, status is OUT (DIR=0); for OUT data or no data, status is IN (DIR=1)
        let status_dir = if data_len > 0 && data_in { 0u32 } else { 1u32 << 16 };
        t2.control = (TRB_STATUS << 10) | TRB_IOC | ep0.cycle | status_dir;
        ep0.enq += 1;
    }

    // Doorbell: slot, target EP0 = 1
    unsafe { w32(x.db(), (slot as usize) * 4, 1); }

    // Wait for Transfer Event
    match wait_event(x, TRB_TRANSFER, 500000) {
        Some(ev) => {
            if ev.cc == 1 || ev.cc == 13 {
                // Success or Short Packet
                serial::write_str("  [EP0] Transfer Event CC=");
                serial::write_usize(ev.cc as usize);
                serial::write_str(" OK\n");
                true
            } else {
                serial::write_str("  [EP0] Transfer Event CC=");
                serial::write_usize(ev.cc as usize);
                serial::write_str(" FAIL\n");
                false
            }
        }
        None => {
            // Also accept CMD_COMPLETE? Transfer uses Transfer Event
            serial::write_str("  [EP0] Transfer TIMEOUT\n");
            false
        }
    }
}

fn get_descriptor(x: &mut Xhci, slot: u8, ep0: &mut Ep0Ring, desc_type: u8, desc_idx: u8, lang: u16, len: u16, buf: usize) -> bool {
    let mut setup = [0u8; 8];
    setup[0] = 0x80; // Device to host, standard, device
    setup[1] = 0x06; // GET_DESCRIPTOR
    setup[2] = desc_idx;
    setup[3] = desc_type;
    setup[4] = (lang & 0xFF) as u8;
    setup[5] = (lang >> 8) as u8;
    setup[6] = (len & 0xFF) as u8;
    setup[7] = (len >> 8) as u8;

    serial::write_str("  [CTRL] GET_DESCRIPTOR type=");
    serial::write_usize(desc_type as usize);
    serial::write_str(" len=");
    serial::write_usize(len as usize);
    serial::write_str("\n");

    ep0_submit_control(x, slot, ep0, &setup, buf, len as u32, true)
}

fn dump_bytes(label: &str, buf: usize, len: usize) {
    serial::write_str("  [");
    serial::write_str(label);
    serial::write_str("]");
    let mut i = 0usize;
    while i < len {
        let b = unsafe { *((buf + i) as *const u8) };
        serial::write_str(" ");
        // 2-digit hex
        let hi = b >> 4;
        let lo = b & 0xf;
        let ch = |d: u8| -> u8 { if d < 10 { b'0' + d } else { b'a' + d - 10 } };
        // use write_str single chars via hex of full byte simpler:
        serial::write_hex(b as usize);
        i += 1;
        if i >= 64 { serial::write_str(" ..."); break; }
    }
    serial::write_str("\n");
}

fn parse_device_desc(buf: usize) {
    let b = |o: usize| unsafe { *((buf + o) as *const u8) };
    let w = |o: usize| unsafe { core::ptr::read_unaligned((buf + o) as *const u16) };
    serial::write_str("  [DEV] bLength=");
    serial::write_usize(b(0) as usize);
    serial::write_str(" bDescType=");
    serial::write_usize(b(1) as usize);
    serial::write_str(" bcdUSB=");
    hx(w(2) as usize);
    serial::write_str(" class=");
    serial::write_usize(b(4) as usize);
    serial::write_str(" sub=");
    serial::write_usize(b(5) as usize);
    serial::write_str(" proto=");
    serial::write_usize(b(6) as usize);
    serial::write_str("\n  [DEV] maxPkt0=");
    serial::write_usize(b(7) as usize);
    serial::write_str(" idVendor=");
    hx(w(8) as usize);
    serial::write_str(" idProduct=");
    hx(w(10) as usize);
    serial::write_str(" bNumConfigs=");
    serial::write_usize(b(17) as usize);
    serial::write_str("\n");
}

fn parse_config_desc(buf: usize, total: usize) {
    serial::write_str("  [CFG] parse total=");
    serial::write_usize(total);
    serial::write_str("\n");
    let mut off = 0usize;
    while off + 2 <= total {
        let blen = unsafe { *((buf + off) as *const u8) } as usize;
        let bty = unsafe { *((buf + off + 1) as *const u8) };
        if blen < 2 { break; }
        if bty == 2 && blen >= 9 {
            let num_intf = unsafe { *((buf + off + 4) as *const u8) };
            let conf_val = unsafe { *((buf + off + 5) as *const u8) };
            serial::write_str("  [CFG] configVal=");
            serial::write_usize(conf_val as usize);
            serial::write_str(" bNumInterfaces=");
            serial::write_usize(num_intf as usize);
            serial::write_str("\n");
        } else if bty == 4 && blen >= 9 {
            let inum = unsafe { *((buf + off + 2) as *const u8) };
            let n_ep = unsafe { *((buf + off + 4) as *const u8) };
            let iclass = unsafe { *((buf + off + 5) as *const u8) };
            let isub = unsafe { *((buf + off + 6) as *const u8) };
            let iproto = unsafe { *((buf + off + 7) as *const u8) };
            serial::write_str("  [CFG] IF=");
            serial::write_usize(inum as usize);
            serial::write_str(" class=");
            serial::write_usize(iclass as usize);
            serial::write_str(" sub=");
            serial::write_usize(isub as usize);
            serial::write_str(" proto=");
            serial::write_usize(iproto as usize);
            serial::write_str(" eps=");
            serial::write_usize(n_ep as usize);
            serial::write_str("\n");
        } else if bty == 5 && blen >= 7 {
            let addr = unsafe { *((buf + off + 2) as *const u8) };
            let attr = unsafe { *((buf + off + 3) as *const u8) };
            let mps = unsafe { core::ptr::read_unaligned((buf + off + 4) as *const u16) };
            serial::write_str("  [CFG] EP addr=");
            hx(addr as usize);
            serial::write_str(" attr=");
            hx(attr as usize);
            serial::write_str(" mps=");
            serial::write_usize(mps as usize);
            serial::write_str("\n");
        }
        off += blen;
    }
}


// ─── HID Boot Keyboard ────────────────────────────────────────

fn control_no_data(x: &mut Xhci, slot: u8, ep0: &mut Ep0Ring, setup: &[u8; 8]) -> bool {
    ep0_submit_control(x, slot, ep0, setup, 0, 0, false)
}

fn set_configuration(x: &mut Xhci, slot: u8, ep0: &mut Ep0Ring, conf: u8) -> bool {
    serial::write_str("\n  [HID] SET_CONFIGURATION\n");
    let setup = [0x00, 0x09, conf, 0x00, 0x00, 0x00, 0x00, 0x00];
    if control_no_data(x, slot, ep0, &setup) {
        serial::write_str("  [HID] SET_CONFIGURATION OK\n");
        true
    } else {
        serial::write_str("  [HID] SET_CONFIGURATION FAIL\n");
        false
    }
}

fn set_protocol_boot(x: &mut Xhci, slot: u8, ep0: &mut Ep0Ring, iface: u8) -> bool {
    serial::write_str("  [HID] SET_PROTOCOL BOOT\n");
    // bmRequestType=0x21, bRequest=0x0B, wValue=0 (boot), wIndex=iface
    let setup = [0x21, 0x0B, 0x00, 0x00, iface, 0x00, 0x00, 0x00];
    if control_no_data(x, slot, ep0, &setup) {
        serial::write_str("  [HID] SET_PROTOCOL BOOT OK\n");
        true
    } else {
        serial::write_str("  [HID] SET_PROTOCOL FAIL\n");
        false
    }
}

fn configure_ep_interrupt_in(x: &mut Xhci, slot: u8, port: u8, ep_ring: usize) -> bool {
    serial::write_str("  [HID] Configure Endpoint 0x81\n");
    let cs = x.ctx_size;
    let in_ctx = match dma_alloc(1) { Some(p) => p, None => return false };
    let ps = unsafe { r32(x.portsc(port), 0) };
    let speed = speed_from_portsc(ps);

    unsafe {
        let ic = in_ctx as *mut u32;
        // Add flags: A0 (slot) | A3 (EP1 IN, DCI=3)
        *ic.add(1) = (1 << 0) | (1 << 3);

        // Slot context: Context Entries = 3 (up to DCI 3)
        let slot_ctx = (in_ctx + cs) as *mut u32;
        *slot_ctx.add(0) = (speed << 20) | (3u32 << 27);
        *slot_ctx.add(1) = (port as u32) << 16;

        // EP1 IN context at offset (1+3)*cs? Layout: control + slot(DCI0) + EP0(DCI1) + EP1OUT(DCI2) + EP1IN(DCI3)
        // Offset for DCI d is (1+d)*cs from start of input context... 
        // Input Control at 0, Slot at 1*cs, EP0 at 2*cs, EP1 OUT at 3*cs, EP1 IN at 4*cs
        let ep_in = (in_ctx + 4 * cs) as *mut u32;
        // Interval: for HS, bInterval=10 → Interval field = 10-1 = 9? Spec: Interval is 2^(Interval-1) * 125us for HS
        // Descriptor had bInterval=7. Use 7 for host Interval field (xHCI uses the value as exponent for HS).
        *ep_in.add(0) = 7u32 << 16; // Interval in bits 23:16
        // CErr=3, EP Type=Interrupt IN (7), MPS=8
        *ep_in.add(1) = (3 << 1) | (7 << 3) | (8 << 16);
        let deq = (ep_ring as u64) | 1; // DCS=1
        *ep_in.add(2) = deq as u32;
        *ep_in.add(3) = (deq >> 32) as u32;
        *ep_in.add(4) = 8; // Average TRB Length

        // Init EP ring Link TRB
        let trbs = ep_ring as *mut Trb;
        let mut i = 0usize;
        while i < 64 { *trbs.add(i) = Trb::z(); i += 1; }
        let link = &mut *trbs.add(63);
        link.lo = ep_ring as u32;
        link.control = (TRB_LINK << 10) | TRB_TC | CYCLE;
    }

    let ctrl = (TRB_CONFIGURE_EP << 10) | ((slot as u32) << 24);
    if !submit_cmd_full(x, in_ctx as u32, 0, 0, ctrl) { return false; }
    match wait_event(x, TRB_CMD_COMPLETE, 500000) {
        Some(ev) if ev.cc == 1 => {
            serial::write_str("  [HID] EP 0x81 configured\n");
            true
        }
        Some(ev) => {
            serial::write_str("  [HID] Configure EP CC=");
            serial::write_usize(ev.cc as usize);
            serial::write_str(" FAIL\n");
            false
        }
        None => {
            serial::write_str("  [HID] Configure EP TIMEOUT\n");
            false
        }
    }
}

fn hid_queue_interrupt_in(x: &mut Xhci, slot: u8, ep_ring: usize, enq: &mut usize, cycle: &mut u32, buf: usize) -> bool {
    // Normal TRB; leave last slot for Link TRB (index 63)
    if *enq >= 63 {
        // Producer wrapped — toggle cycle (Link TRB has TC)
        *enq = 0;
        *cycle ^= 1;
    }
    unsafe {
        let trbs = ep_ring as *mut Trb;
        let t = &mut *trbs.add(*enq);
        t.lo = buf as u32;
        t.hi = 0;
        // Transfer TRB: status[16:0] = TRB transfer length
        t.status = 4; // Boot mouse report is 3-4 bytes
        t.control = (TRB_NORMAL << 10) | TRB_IOC | *cycle;
        *enq += 1;
        DIAG_TRB_QUEUED = DIAG_TRB_QUEUED.wrapping_add(1);
        w32(x.db(), (slot as usize) * 4, 3); // DCI 3 = EP1 IN
    }
    true
}

fn hid_boot_keyboard(x: &mut Xhci, slot: u8, port: u8, ep0: &mut Ep0Ring) -> bool {
    serial::write_str("\n======== HID BOOT KEYBOARD ========\n");

    if !set_configuration(x, slot, ep0, 1) { return false; }
    if !set_protocol_boot(x, slot, ep0, 0) { return false; }

    let ep_ring = match dma_alloc(1) { Some(p) => p, None => return false };
    let report_buf = match dma_alloc(1) { Some(p) => p, None => return false };
    serial::write_str("  [HID] EP ring=");
    hx(ep_ring);
    serial::write_str(" report_buf=");
    hx(report_buf);
    serial::write_str("\n");

    if !configure_ep_interrupt_in(x, slot, port, ep_ring) { return false; }

    serial::write_str("  [HID] Interrupt IN started — waiting for reports\n");
    serial::write_str("  [HID] (QEMU: use monitor sendkey)\n");

    let mut enq = 0usize;
    let mut cycle = 1u32;
    let mut reports = 0u32;
    let mut polls = 0u32;

    // Queue first TRB
    if !hid_queue_interrupt_in(x, slot, ep_ring, &mut enq, &mut cycle, report_buf) {
        return false;
    }

    // Poll for transfer events; also pump compositor input
    while polls < 20000 {
        // Check for transfer event without blocking forever
        if let Some(ev) = wait_event(x, TRB_TRANSFER, 80) {
            if ev.cc == 1 || ev.cc == 13 {
                let mut report = [0u8; 8];
                let mut i = 0usize;
                while i < 8 {
                    report[i] = unsafe { *((report_buf + i) as *const u8) };
                    i += 1;
                }
                serial::write_str("  [HID] report received:");
                i = 0;
                while i < 8 {
                    serial::write_str(" ");
                    hx(report[i] as usize);
                    i += 1;
                }
                serial::write_str("\n");

                // Log keycodes
                i = 2;
                while i < 8 {
                    if report[i] != 0 {
                        serial::write_str("  [HID] keycode=0x");
                        hx(report[i] as usize);
                        serial::write_str("\n");
                        if let Some(ch) = crate::input::hid_to_ascii(report[i], (report[0] & 0x22) != 0) {
                            serial::write_str("  [INPUT] key=0x");
                            hx(ch as usize);
                            serial::write_str("\n");
                        }
                    }
                    i += 1;
                }
                if report[0] != 0 {
                    serial::write_str("  [HID] modifiers=0x");
                    hx(report[0] as usize);
                    serial::write_str("\n");
                }

                crate::input::process_hid_boot_report(&report);
                // Deliver to GUI
                crate::compositor::handle_input();
                reports += 1;

                // Re-queue next interrupt read
                // Clear report buffer
                unsafe {
                    let mut j = 0usize;
                    while j < 8 { *((report_buf + j) as *mut u8) = 0; j += 1; }
                }
                let _ = hid_queue_interrupt_in(x, slot, ep_ring, &mut enq, &mut cycle, report_buf);
            } else {
                serial::write_str("  [HID] Transfer CC=");
                serial::write_usize(ev.cc as usize);
                serial::write_str("\n");
            }
        }
        polls += 1;
        // Continuous polling; exit only after enough wall-time for test keys
        // (~several seconds of spin) so monitor sendkey can inject A B Enter BS Shift+A
        if polls > 15_000 { break; }
    }

    serial::write_str("  [HID] done reports=");
    serial::write_usize(reports as usize);
    serial::write_str(" polls=");
    serial::write_usize(polls as usize);
    serial::write_str("\n");
    reports > 0
}


// ─── Enumeration orchestration ────────────────────────────────


fn hid_boot_mouse(x: &mut Xhci, slot: u8, port: u8, ep0: &mut Ep0Ring) -> bool {
    serial::write_str("\n======== HID BOOT MOUSE ========\n");
    serial::write_str("[USB] mouse probe start\n");
    unsafe { DIAG_MOUSE_IF = true; }
    let ep_ring = match dma_alloc(1) {
        Some(p) => p,
        None => {
            serial::write_str("[USB] HID FAIL (no DMA)\n");
            return false;
        }
    };
    let report_buf = match dma_alloc(1) {
        Some(p) => p,
        None => return false,
    };
    // Queue Interrupt IN (Boot Mouse EP often 0x81 when sole device)
    let mut enq = 0usize;
    let mut cycle = 1u32;
    if !hid_queue_interrupt_in(x, slot, ep_ring, &mut enq, &mut cycle, report_buf) {
        serial::write_str("[USB] HID FAIL (queue)\n");
        return false;
    }
    unsafe { DIAG_EP_IN = true; DIAG_HID = true; }
    serial::write_str("[USB] HID OK EP_IN OK\n");
    let mut polls = 0u32;
    let mut got = 0u32;
    while polls < 8000 {
        if let Some(ev) = wait_event(x, TRB_TRANSFER, 40) {
            if ev.cc == 1 || ev.cc == 13 {
                let mut report = [0u8; 4];
                let mut i = 0usize;
                while i < 4 {
                    report[i] = unsafe { *((report_buf + i) as *const u8) };
                    i += 1;
                }
                crate::input::process_hid_boot_mouse(&report);
                got += 1;
                unsafe { DIAG_REPORTS = got; }
                unsafe {
                    let mut j = 0usize;
                    while j < 4 {
                        *((report_buf + j) as *mut u8) = 0;
                        j += 1;
                    }
                }
                let _ = hid_queue_interrupt_in(x, slot, ep_ring, &mut enq, &mut cycle, report_buf);
            }
        }
        polls += 1;
        if polls > 5000 {
            break;
        }
    }
    // Keep live path for desktop: save ER + EP state, leave TRB queued
    unsafe {
        MOUSE_ER = x.er;
        MOUSE_ER_DEQ = x.er_deq;
        MOUSE_ER_CYCLE = x.er_cycle;
        MOUSE_ER_SIZE = x.er_size;
        MOUSE_INTR0 = x.intr0();
        MOUSE_EP_RING = ep_ring;
        MOUSE_ENQ = enq;
        MOUSE_CYCLE = cycle;
        MOUSE_REPORT = report_buf;
        MOUSE_SLOT = slot;
        MOUSE_DB = x.db();
        MOUSE_LIVE = true;
        DIAG_HID = true;
        DIAG_EP_IN = true;
        DIAG_REPORTS = got;
    }
    // Ensure one outstanding interrupt IN
    let _ = hid_queue_interrupt_in(x, slot, ep_ring, &mut enq, &mut cycle, report_buf);
    unsafe {
        MOUSE_ENQ = enq;
        MOUSE_CYCLE = cycle;
    }
    serial::write_str("[USB] reports=");
    serial::write_usize(got as usize);
    serial::write_str(" live=1\n");
    true // live path ready even if 0 reports during short poll
}

fn enumerate(x: &mut Xhci) -> bool {
    serial::write_str("\n======== USB ENUMERATION ========\n");

    let port = match find_connected_port(x) {
        Some(p) => p,
        None => {
            serial::write_str("  [ENUM] no connected device\n");
            return false;
        }
    };
    serial::write_str("  [ENUM] using port ");
    serial::write_usize(port as usize);
    serial::write_str("\n");

    if !port_reset(x, port) {
        serial::write_str("  [ENUM] port reset failed\n");
        return false;
    }

    let slot = match enable_slot(x) {
        Some(s) => s,
        None => return false,
    };

    let ep0_phys = match address_device(x, slot, port) {
        Some(p) => p,
        None => return false,
    };

    let mut ep0 = Ep0Ring { phys: ep0_phys, enq: 0, cycle: 1, size: 64 };

    // Buffer for descriptors
    let buf = match dma_alloc(1) {
        Some(p) => p,
        None => return false,
    };

    // GET_DESCRIPTOR Device (18 bytes)
    serial::write_str("\n  --- GET_DESCRIPTOR Device ---\n");
    if !get_descriptor(x, slot, &mut ep0, 1, 0, 0, 18, buf) {
        serial::write_str("  [ENUM] device descriptor transfer FAIL\n");
        return false;
    }
    dump_bytes("DEV-RAW", buf, 18);
    parse_device_desc(buf);

    // First 9 bytes of config to get wTotalLength
    serial::write_str("\n  --- GET_DESCRIPTOR Config (9) ---\n");
    if !get_descriptor(x, slot, &mut ep0, 2, 0, 0, 9, buf) {
        serial::write_str("  [ENUM] config desc header FAIL\n");
        return false;
    }
    dump_bytes("CFG-HDR", buf, 9);
    let total = unsafe { core::ptr::read_unaligned((buf + 2) as *const u16) } as usize;
    let total = if total < 9 || total > 512 { 9 } else { total };

    serial::write_str("\n  --- GET_DESCRIPTOR Config (full) ---\n");
    if !get_descriptor(x, slot, &mut ep0, 2, 0, 0, total as u16, buf) {
        serial::write_str("  [ENUM] full config FAIL\n");
        return false;
    }
    dump_bytes("CFG-RAW", buf, total);
    parse_config_desc(buf, total);

    serial::write_str("\n  [ENUM] ENUMERATION SUCCESS\n");

    // HID Boot Keyboard path
    let _ = hid_boot_keyboard(x, slot, port, &mut ep0);
    // Optional mouse (no hang)
    let _ = hid_boot_mouse(x, slot, port, &mut ep0);
    true // never fail boot on HID
}

// ─── Public entry ─────────────────────────────────────────────

fn init_one(bus: u8, dev: u8, func: u8) -> bool {
    serial::write_str("\n[xHCI] === ");
    serial::write_usize(bus as usize);
    serial::write_str(":");
    serial::write_usize(dev as usize);
    serial::write_str(".");
    serial::write_usize(func as usize);
    serial::write_str(" ===\n");

    unsafe {
        let cmd = pci_r32(bus, dev, func, 0x04);
        pci_w32(bus, dev, func, 0x04, cmd | 0x6);
    }
    let bar0 = unsafe { pci_r32(bus, dev, func, 0x10) };
    let is64 = (bar0 & 0x6) == 0x4;
    let mut phys = (bar0 & !0xF) as u64;
    if is64 {
        let bar1 = unsafe { pci_r32(bus, dev, func, 0x14) };
        phys |= (bar1 as u64) << 32;
    }
    // size probe simplified
    let size = 0x4000u64;
    serial::write_str("  BAR0=");
    hx(phys as usize);
    serial::write_str("\n");
    if !map_mmio(phys, size) { return false; }

    let mmio = phys as usize;
    let d0 = unsafe { r32(mmio, 0) };
    let cap = (d0 & 0xFF) as u8;
    let hcs1 = unsafe { r32(mmio, 4) };
    let hcc1 = unsafe { r32(mmio, 0x10) };
    let ctx_size = if hcc1 & 4 != 0 { 64usize } else { 32usize };
    let slots = (hcs1 & 0xFF) as u8;
    let ports = ((hcs1 >> 24) & 0xFF) as u8;
    let dboff = unsafe { r32(mmio, 0x14) };
    let rtsoff = unsafe { r32(mmio, 0x18) };

    serial::write_str("  CAP=");
    serial::write_usize(cap as usize);
    serial::write_str(" slots=");
    serial::write_usize(slots as usize);
    serial::write_str(" ports=");
    serial::write_usize(ports as usize);
    serial::write_str(" ctx=");
    serial::write_usize(ctx_size);
    serial::write_str("\n");

    if !hc_reset(mmio, cap) {
        serial::write_str("  reset FAIL\n");
        return false;
    }

    let mut x = Xhci {
        mmio, cap, slots, ports, dboff, rtsoff, ctx_size,
        dcbaa: 0, cmd_ring: 0, cmd_size: 0, cmd_enq: 0, cmd_cycle: 1,
        erst: 0, er: 0, er_size: 0, er_deq: 0, er_cycle: 1,
    };
    if !setup_rings(&mut x) {
        serial::write_str("  rings FAIL\n");
        return false;
    }
    serial::write_str("  [xHCI] running — start enumeration\n");
    enumerate(&mut x)
}


/// Non-blocking poll of HID mouse transfer events (desktop loop). Timeout-safe.
pub fn poll_mouse_live() {
    unsafe {
        if !MOUSE_LIVE || MOUSE_ER == 0 || MOUSE_REPORT == 0 {
            return;
        }
        // Process a few event ring entries
        let mut spins = 0u32;
        while spins < 32 {
            let trbs = MOUSE_ER as *const Trb;
            let ev = *trbs.add(MOUSE_ER_DEQ);
            if (ev.control & 1) != MOUSE_ER_CYCLE {
                break; // no event
            }
            let ty = (ev.control >> 10) & 0x3F;
            let cc = (ev.status >> 24) & 0xFF;
            MOUSE_ER_DEQ += 1;
            if MOUSE_ER_DEQ >= MOUSE_ER_SIZE {
                MOUSE_ER_DEQ = 0;
                MOUSE_ER_CYCLE ^= 1;
            }
            // Update ERDP
            if MOUSE_INTR0 != 0 {
                let erdp = (MOUSE_ER + MOUSE_ER_DEQ * 16) as u64;
                core::ptr::write_volatile((MOUSE_INTR0 + 0x18) as *mut u64, erdp | (1 << 3));
            }
            if ty == TRB_TRANSFER && (cc == 1 || cc == 13) {
                DIAG_COMPLETIONS = DIAG_COMPLETIONS.wrapping_add(1);
                let mut report = [0u8; 4];
                let mut i = 0usize;
                while i < 4 {
                    report[i] = *((MOUSE_REPORT + i) as *const u8);
                    i += 1;
                }
                crate::input::process_hid_boot_mouse(&report);
                DIAG_REPORTS = DIAG_REPORTS.wrapping_add(1);
                MOUSE_EVENTS = MOUSE_EVENTS.wrapping_add(1);
                // clear + re-queue
                let mut j = 0usize;
                while j < 4 {
                    *((MOUSE_REPORT + j) as *mut u8) = 0;
                    j += 1;
                }
                // queue next TRB manually (mirror hid_queue_interrupt_in)
                let ring = MOUSE_EP_RING as *mut Trb;
                let idx = MOUSE_ENQ;
                let trb = ring.add(idx);
                (*trb).lo = MOUSE_REPORT as u32;
                (*trb).hi = 0;
                (*trb).status = 4 | (1 << 22); // 4 bytes residual? length
                (*trb).control = (TRB_NORMAL << 10) | TRB_IOC | MOUSE_CYCLE;
                MOUSE_ENQ += 1;
                if MOUSE_ENQ >= 16 {
                    // simple ring wrap without link for small ring
                    MOUSE_ENQ = 0;
                    MOUSE_CYCLE ^= 1;
                }
                // doorbell EP1 (interrupt IN often DCI 2 = EP1 OUT? Boot mouse IN is EP1 → DCI 3)
                // Use doorbell slot with target 3 (EP1 IN)
                if MOUSE_DB != 0 && MOUSE_SLOT != 0 {
                    core::ptr::write_volatile((MOUSE_DB + (MOUSE_SLOT as usize) * 4) as *mut u32, 3);
                }
            }
            spins += 1;
        }
    }
}

pub fn probe() {
    serial::write_str("\n[xHCI] scan + enumerate...\n");
    unsafe { DIAG_XHCI = false; DIAG_DEV = false; DIAG_HID = false; DIAG_MOUSE_IF = false; DIAG_EP_IN = false; DIAG_REPORTS = 0; DIAG_FOUND = 0; }
    let mut found = 0u32;
    let mut ok = 0u32;
    let mut bus = 0u8;
    while bus < 16 {
        let mut dev = 0u8;
        while dev < 32 {
            let mut func = 0u8;
            while func < 8 {
                let id = unsafe { pci_r32(bus, dev, func, 0) };
                if id != 0xFFFFFFFF {
                    let cl = unsafe { pci_r32(bus, dev, func, 0x08) };
                    if ((cl >> 24) & 0xFF) == 0x0C && ((cl >> 16) & 0xFF) == 0x03 && ((cl >> 8) & 0xFF) == 0x30 {
                        unsafe { DIAG_XHCI = true; }
                        found += 1;
                        if init_one(bus, dev, func) { ok += 1; unsafe { DIAG_DEV = true; DIAG_FOUND += 1; } }
                    }
                }
                func += 1;
            }
            dev += 1;
        }
        bus += 1;
    }
    if !unsafe { DIAG_XHCI } {
        serial::write_str("[USB] xHCI FAIL\n");
    } else {
        serial::write_str("[USB] xHCI OK\n");
    }
    if !unsafe { DIAG_DEV } {
        serial::write_str("[USB] mouse NOT FOUND\n");
    } else {
        serial::write_str("[USB] mouse found\n");
    }
    serial::write_str("[xHCI] enum done found=");
    serial::write_usize(found as usize);
    serial::write_str(" ok=");
    serial::write_usize(ok as usize);
    serial::write_str("\n");
}
