//! Aether RTL8168/8111 Ethernet driver.
//! Target: Realtek 10EC:8168, RTL8168EVL/8111EVL (MAC version 34).
//! First data-path stage: PCI -> MMIO -> reset -> DMA rings -> link.
//! Protocol traffic is deliberately not enabled until this hardware stage is proven.

use crate::serial;

const VID: u16 = 0x10EC;
const DID: u16 = 0x8168;
const XID_EVL: u32 = 0x2C8;

const CR: usize = 0x37;
const TPPOLL: usize = 0x38;
const IMR: usize = 0x3C;
const ISR: usize = 0x3E;
const TCR: usize = 0x40;
const RCR: usize = 0x44;
const PHYSTATUS: usize = 0x6C;
const TNPDS: usize = 0x20;
const RDSAR: usize = 0xE4;
const RMS: usize = 0xDA;
const CPLUSc: usize = 0xE0;
const CMD_RESET: u8 = 0x10;
const CMD_TE: u8 = 0x04;
const CMD_RE: u8 = 0x08;

const DESC_OWN: u32 = 1 << 31;
const DESC_EOR: u32 = 1 << 30;
const DESC_LS: u32 = 1 << 28;
const DESC_FS: u32 = 1 << 29;

const RX_COUNT: usize = 32;
const TX_COUNT: usize = 8;
const BUF_SIZE: usize = 2048;

#[repr(C)]
#[derive(Copy, Clone)]
struct Desc { opts1: u32, opts2: u32, addr_lo: u32, addr_hi: u32 }

#[repr(align(256))]
struct TxRing([Desc; TX_COUNT]);
#[repr(align(256))]
struct RxRing([Desc; RX_COUNT]);
#[repr(align(16))]
struct TxBuf([u8; BUF_SIZE]);
#[repr(align(16))]
struct RxBuf([u8; BUF_SIZE]);

const EMPTY_DESC: Desc = Desc { opts1: 0, opts2: 0, addr_lo: 0, addr_hi: 0 };
static mut TX_RING: TxRing = TxRing([EMPTY_DESC; TX_COUNT]);
static mut RX_RING: RxRing = RxRing([EMPTY_DESC; RX_COUNT]);
static mut TX_BUFS: [TxBuf; TX_COUNT] = unsafe { core::mem::transmute([[0u8; BUF_SIZE]; TX_COUNT]) };
static mut RX_BUFS: [RxBuf; RX_COUNT] = unsafe { core::mem::transmute([[0u8; BUF_SIZE]; RX_COUNT]) };

static mut MMIO: usize = 0;
static mut FOUND: bool = false;
static mut READY: bool = false;
static mut LINK: bool = false;
static mut MAC: [u8; 6] = [0; 6];

#[inline] unsafe fn r8(off: usize) -> u8 { core::ptr::read_volatile((MMIO + off) as *const u8) }
#[inline] unsafe fn r16(off: usize) -> u16 { core::ptr::read_volatile((MMIO + off) as *const u16) }
#[inline] unsafe fn r32(off: usize) -> u32 { core::ptr::read_volatile((MMIO + off) as *const u32) }
#[inline] unsafe fn w8(off: usize, v: u8) { core::ptr::write_volatile((MMIO + off) as *mut u8, v) }
#[inline] unsafe fn w16(off: usize, v: u16) { core::ptr::write_volatile((MMIO + off) as *mut u16, v) }
#[inline] unsafe fn w32(off: usize, v: u32) { core::ptr::write_volatile((MMIO + off) as *mut u32, v) }

unsafe fn pci_r32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let a = 0x8000_0000u32 | ((bus as u32) << 16) | ((dev as u32) << 11) |
        ((func as u32) << 8) | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") 0xCFCu16, out("eax") v, options(nostack, preserves_flags));
    v
}

unsafe fn pci_w32(bus: u8, dev: u8, func: u8, off: u8, v: u32) {
    let a = 0x8000_0000u32 | ((bus as u32) << 16) | ((dev as u32) << 11) |
        ((func as u32) << 8) | ((off as u32) & 0xFC);
    core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") a, options(nostack, preserves_flags));
    core::arch::asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") v, options(nostack, preserves_flags));
}

unsafe fn map_mmio(phys: usize) -> bool {
    let cr3 = crate::mm::paging::kernel_cr3();
    if !crate::mm::paging::map_page(
        cr3, phys & !0xFFF, phys & !0xFFF,
        crate::mm::paging::PAGE_PRESENT | crate::mm::paging::PAGE_PCD,
    ) { return false; }
    crate::mm::paging::load_cr3(cr3);
    true
}

unsafe fn find_device() -> Option<(u8, u8, u8, usize)> {
    for bus in 0..=31u8 {
        for dev in 0..32u8 {
            for func in 0..8u8 {
                let id = pci_r32(bus, dev, func, 0);
                if id == 0 || id == 0xFFFF_FFFF { continue; }
                if (id & 0xFFFF) as u16 != VID || (id >> 16) as u16 != DID { continue; }
                let class = pci_r32(bus, dev, func, 0x08);
                if ((class >> 24) & 0xFF) != 0x02 || ((class >> 16) & 0xFF) != 0x00 { continue; }
                let lo = pci_r32(bus, dev, func, 0x18);
                let hi = pci_r32(bus, dev, func, 0x1C);
                let base = (lo & !0x0F) as usize | ((hi as u64) << 32) as usize;
                if base != 0 { return Some((bus, dev, func, base)); }
            }
        }
    }
    None
}

unsafe fn reset() -> bool {
    w8(CR, CMD_RESET);
    let mut i = 0usize;
    while i < 200_000 {
        if r8(CR) & CMD_RESET == 0 {
            w16(IMR, 0);
            w16(ISR, 0xFFFF);
            return true;
        }
        core::hint::spin_loop();
        i += 1;
    }
    false
}

unsafe fn init_rings() {
    TX_RING.0 = [EMPTY_DESC; TX_COUNT];
    RX_RING.0 = [EMPTY_DESC; RX_COUNT];

    for i in 0..TX_COUNT {
        let p = &TX_BUFS[i].0 as *const u8 as usize;
        TX_RING.0[i].addr_lo = p as u32;
        TX_RING.0[i].addr_hi = (p >> 32) as u32;
        TX_RING.0[i].opts1 = if i + 1 == TX_COUNT { DESC_EOR } else { 0 };
    }
    for i in 0..RX_COUNT {
        let p = &RX_BUFS[i].0 as *const u8 as usize;
        RX_RING.0[i].addr_lo = p as u32;
        RX_RING.0[i].addr_hi = (p >> 32) as u32;
        RX_RING.0[i].opts1 = DESC_OWN | (BUF_SIZE as u32) |
            if i + 1 == RX_COUNT { DESC_EOR } else { 0 };
    }

    let tx = &TX_RING.0 as *const Desc as usize;
    let rx = &RX_RING.0 as *const Desc as usize;
    w32(TNPDS, tx as u32);
    w32(TNPDS + 4, (tx >> 32) as u32);
    w32(RDSAR, rx as u32);
    w32(RDSAR + 4, (rx >> 32) as u32);

    w16(CPLUSc, r16(CPLUSc) | 0x0008);
    w16(RMS, BUF_SIZE as u16);
    w32(TCR, (r32(TCR) & !0x0000_0700) | 0x0000_0700);
    w32(RCR, 0x0000_E00A);
    w16(IMR, 0);
    w16(ISR, 0xFFFF);
    w8(CR, CMD_TE | CMD_RE);
}

pub fn init() {
    unsafe {
        FOUND = false; READY = false; LINK = false; MMIO = 0;
        let Some((bus, dev, func, base)) = find_device() else {
            serial::write_str("[NET] RTL8168 10EC:8168 not present (QEMU-safe)\n");
            return;
        };
        FOUND = true;
        MMIO = base;
        if !map_mmio(base) {
            serial::write_str("[NET] RTL8168 MMIO map failed\n");
            return;
        }

        let txcfg = r32(TCR);
        let xid = (txcfg >> 20) & 0xFCF;
        serial::write_str("[NET] RTL8168 MMIO="); serial::write_hex(base);
        serial::write_str(" TXCONFIG="); serial::write_hex(txcfg as usize);
        serial::write_str(" XID="); serial::write_hex(xid as usize); serial::write_str("\n");
        if xid != XID_EVL {
            serial::write_str("[NET] XID is not RTL8168EVL; refusing generic data path\n");
            return;
        }

        let cmd = pci_r32(bus, dev, func, 0x04);
        pci_w32(bus, dev, func, 0x04, cmd | 0x0000_0006);

        if !reset() {
            serial::write_str("[NET] RTL8168 reset timeout\n");
            return;
        }
        for i in 0..6 { MAC[i] = r8(i); }
        serial::write_str("[NET] RTL8168EVL MAC=");
        for i in 0..6 { serial::write_hex(MAC[i] as usize); if i != 5 { serial::write_str(":"); } }
        serial::write_str("\n");

        init_rings();
        READY = true;
        let ps = r8(PHYSTATUS);
        LINK = (ps & 0x02) != 0;
        serial::write_str("[NET] RTL8168EVL DATA_PATH=READY LINK=");
        serial::write_str(if LINK { "UP" } else { "DOWN" });
        serial::write_str(" PHYSTATUS="); serial::write_hex(ps as usize); serial::write_str("\n");
    }
}

pub fn found() -> bool { unsafe { FOUND } }
pub fn ready() -> bool { unsafe { READY } }
pub fn link_up() -> bool { unsafe { LINK } }
pub fn mac(out: &mut [u8; 6]) { unsafe { *out = MAC; } }
