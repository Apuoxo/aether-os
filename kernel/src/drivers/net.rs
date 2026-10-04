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


/* QEMU validation path: the stock QEMU RTL8139 is used only as a transport
 * backend. The physical AH532 path above remains RTL8168EVL-only. */
const RTL8139: u16 = 0x8139;
const R8139_CR: u16 = 0x37;
const R8139_CAPR: u16 = 0x38;
const R8139_IMR: u16 = 0x3C;
const R8139_ISR: u16 = 0x3E;
const R8139_TCR: u16 = 0x40;
const R8139_RCR: u16 = 0x44;
const R8139_RBSTART: u16 = 0x30;
const R8139_CONFIG1: u16 = 0x52;
const R8139_TSAD0: u16 = 0x20;
const R8139_TSD0: u16 = 0x10;
const R8139_CMD_RESET: u8 = 0x10;
const R8139_CMD_RE: u8 = 0x08;
const R8139_CMD_TE: u8 = 0x04;
const R8139_TSD_TOK: u32 = 1 << 15;
const R8139_RX: usize = 8192 + 16 + 1500;
static mut R8139_IO: u16 = 0;
#[repr(align(16))]
struct R8139Rx([u8; R8139_RX]);
#[repr(align(16))]
struct R8139Tx([u8; 2048]);
static mut R8139_RXBUF: R8139Rx = R8139Rx([0; R8139_RX]);
static mut R8139_TXBUFS: [R8139Tx; 4] = [R8139Tx([0;2048]), R8139Tx([0;2048]), R8139Tx([0;2048]), R8139Tx([0;2048])];
static mut R8139_TXIDX: usize = 0;
static mut R8139_MAC: [u8; 6] = [0; 6];
static mut R8139_READY: bool = false;

#[inline] unsafe fn in8(p: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") p, out("al") v, options(nostack, preserves_flags));
    v
}
#[inline] unsafe fn in16(p: u16) -> u16 {
    let v: u16;
    core::arch::asm!("in ax, dx", in("dx") p, out("ax") v, options(nostack, preserves_flags));
    v
}
#[inline] unsafe fn in32(p: u16) -> u32 {
    let v: u32;
    core::arch::asm!("in eax, dx", in("dx") p, out("eax") v, options(nostack, preserves_flags));
    v
}
#[inline] unsafe fn out8(p: u16, v: u8) {
    core::arch::asm!("out dx, al", in("dx") p, in("al") v, options(nostack, preserves_flags));
}
#[inline] unsafe fn out16(p: u16, v: u16) {
    core::arch::asm!("out dx, ax", in("dx") p, in("ax") v, options(nostack, preserves_flags));
}
#[inline] unsafe fn out32(p: u16, v: u32) {
    core::arch::asm!("out dx, eax", in("dx") p, in("eax") v, options(nostack, preserves_flags));
}

unsafe fn find_rtl8139() -> Option<(u8,u8,u8,u16)> {
    for bus in 0..=31u8 {
        for dev in 0..32u8 {
            for func in 0..8u8 {
                let id = pci_r32(bus, dev, func, 0);
                if (id & 0xFFFF) as u16 != VID || (id >> 16) as u16 != RTL8139 { continue; }
                let bar = pci_r32(bus, dev, func, 0x10);
                if (bar & 1) != 0 && (bar & !3) != 0 {
                    return Some((bus,dev,func,(bar & !3) as u16));
                }
            }
        }
    }
    None
}

unsafe fn r8139_reset() -> bool {
    out8(R8139_IO + R8139_CR, R8139_CMD_RESET);
    for _ in 0..200_000 {
        if in8(R8139_IO + R8139_CR) & R8139_CMD_RESET == 0 { return true; }
        core::hint::spin_loop();
    }
    false
}

unsafe fn r8139_tx(frame: &[u8]) -> bool {
    if frame.len() > 2048 { return false; }
    core::ptr::copy_nonoverlapping(frame.as_ptr(), R8139_TXBUF.0.as_mut_ptr(), frame.len());
    let p = &R8139_TXBUF.0 as *const u8 as usize;
    out32(R8139_IO + R8139_TSAD0, p as u32);
    out32(R8139_IO + R8139_TSD0, frame.len() as u32);
    for _ in 0..500_000 {
        let s = in32(R8139_IO + R8139_TSD0);
        if (s & R8139_TSD_TOK) != 0 { return true; }
        core::hint::spin_loop();
    }
    false
}

unsafe fn csum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0usize;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i],data[i+1]]) as u32;
        i += 2;
    }
    if i < data.len() { sum += (data[i] as u32) << 8; }
    while (sum >> 16) != 0 { sum = (sum & 0xFFFF) + (sum >> 16); }
    !(sum as u16)
}

unsafe fn r8139_poll_frame(out: &mut [u8], timeout: usize) -> usize {
    let mut off = (in16(R8139_IO + R8139_CAPR) as usize + 16) % 8192;
    for _ in 0..timeout {
        let base = R8139_RXBUF.0.as_ptr().add(off);
        let status = u16::from_le_bytes([*base, *base.add(1)]);
        let len = u16::from_le_bytes([*base.add(2), *base.add(3)]) as usize;
        if (status & 1) != 0 && len >= 4 && len <= 2048 {
            let n = core::cmp::min(len - 4, out.len());
            core::ptr::copy_nonoverlapping(base.add(4), out.as_mut_ptr(), n);
            let next = (off + len + 4 + 3) & !3;
            out16(R8139_IO + R8139_CAPR, (next as u16).wrapping_sub(16));
            return n;
        }
        core::hint::spin_loop();
    }
    0
}

unsafe fn r8139_frame_test() -> bool {
    let mut arp = [0u8; 64];
    for i in 0..6 { arp[i] = 0xFF; }
    for i in 0..6 { arp[6+i] = R8139_MAC[i]; }
    arp[12]=0x08; arp[13]=0x06;
    arp[14]=0; arp[15]=1; arp[16]=0x08; arp[17]=0; arp[18]=6; arp[19]=4;
    arp[20]=0; arp[21]=1;
    for i in 0..6 { arp[22+i]=R8139_MAC[i]; }
    arp[28]=10; arp[29]=0; arp[30]=2; arp[31]=15;
    for i in 0..6 { arp[32+i]=0; }
    arp[38]=10; arp[39]=0; arp[40]=2; arp[41]=2;
    if !r8139_tx(&arp[..42]) { serial::write_str("[NET-QEMU] ARP TX FAIL\n"); return false; }
    serial::write_str("[NET-QEMU] ARP TX OK TSD="); serial::write_hex(in32(R8139_IO + R8139_TSD0) as usize); serial::write_str("\n");

    let mut rx = [0u8; 2048];
    let n = r8139_poll_frame(&mut rx, 1_500_000);
    serial::write_str("[NET-QEMU] ARP RX bytes="); serial::write_hex(n); serial::write_str(" ISR="); serial::write_hex(in16(R8139_IO + R8139_ISR) as usize); serial::write_str(" CAPR="); serial::write_hex(in16(R8139_IO + R8139_CAPR) as usize); serial::write_str("\n");
    if n < 42 || rx[12] != 0x08 || rx[13] != 0x06 || rx[20] != 0 || rx[21] != 2 {
        return false;
    }
    let mut gw = [0u8;6];
    for i in 0..6 { gw[i] = rx[22+i]; }
    if rx[28..32] != [10,0,2,2] { return false; }

    let mut ip = [0u8; 98];
    for i in 0..6 { ip[i]=gw[i]; ip[6+i]=R8139_MAC[i]; }
    ip[12]=0x08; ip[13]=0x00;
    ip[14]=0x45; ip[15]=0; ip[16]=0; ip[17]=84;
    ip[18]=0x12; ip[19]=0x34; ip[20]=0x40; ip[21]=0;
    ip[22]=64; ip[23]=1; ip[24]=0; ip[25]=0;
    ip[26]=10; ip[27]=0; ip[28]=2; ip[29]=15;
    ip[30]=10; ip[31]=0; ip[32]=2; ip[33]=2;
    let ipcs = csum(&ip[14..34]); ip[24]=(ipcs>>8) as u8; ip[25]=ipcs as u8;
    ip[34]=8; ip[35]=0; ip[36]=0; ip[37]=0; ip[38]=0x12; ip[39]=0x34; ip[40]=0; ip[41]=1;
    for i in 42..98 { ip[i]=i as u8; }
    let ics = csum(&ip[34..98]); ip[36]=(ics>>8) as u8; ip[37]=ics as u8;
    if !r8139_tx(&ip[..98]) { serial::write_str("[NET-QEMU] ICMP TX FAIL TSD="); serial::write_hex(in32(R8139_IO + R8139_TSD0) as usize); serial::write_str("\n"); return false; }
    serial::write_str("[NET-QEMU] ICMP TX OK\n");

    let n2 = r8139_poll_frame(&mut rx, 1_500_000);
    serial::write_str("[NET-QEMU] ICMP RX bytes="); serial::write_hex(n2); serial::write_str(" ISR="); serial::write_hex(in16(R8139_IO + R8139_ISR) as usize); serial::write_str("\n");
    n2 >= 42 && rx[12] == 0x08 && rx[13] == 0x00 && rx[14] == 0x45 &&
        rx[23] == 1 && rx[34] == 0 && rx[35] == 0 && rx[38] == 0x12 && rx[39] == 0x34
}

pub fn qemu_ping() -> bool {
    unsafe {
        let Some((bus,dev,func,io)) = find_rtl8139() else { return false; };
        R8139_IO=io;
        let cmd=pci_r32(bus,dev,func,0x04);
        pci_w32(bus,dev,func,0x04,cmd|0x0005);
        if !r8139_reset() { return false; }
        for i in 0..6 { R8139_MAC[i]=in8(R8139_IO+i as u16); }
        out32(R8139_IO+R8139_RBSTART,&R8139_RXBUF.0 as *const u8 as usize as u32);
        out16(R8139_IO+R8139_IMR,0);
        out16(R8139_IO+R8139_ISR,0xFFFF);
        out32(R8139_IO+R8139_RCR,0x0000078F);\n        for i in 0..4 { let p=&R8139_TXBUFS[i].0 as *const u8 as usize; out32(R8139_IO+R8139_TSAD0+(i as u16)*4,p as u32); }
        out32(R8139_IO+R8139_TCR,0x03000700);
        out8(R8139_IO+R8139_CR,R8139_CMD_RE|R8139_CMD_TE);
        out8(R8139_IO+R8139_CONFIG1,0);
        R8139_READY=true;
        serial::write_str("[NET-QEMU] RTL8139 READY MAC=");
        for i in 0..6 { serial::write_hex(R8139_MAC[i] as usize); if i!=5 {serial::write_str(":");} }
        serial::write_str("\n[NET-QEMU] ARP+ICMP test -> 10.0.2.2\n");
        let ok=r8139_frame_test();
        serial::write_str(if ok {"[NET-QEMU] PING=PASS\n"} else {"[NET-QEMU] PING=FAIL\n"});
        ok
    }
}
