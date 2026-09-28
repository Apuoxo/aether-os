//! Physical Memory Manager + paging re-export

use core::sync::atomic::{AtomicUsize, Ordering};

pub mod paging;

const PAGE_SIZE: usize = 4096;
const MAX_PAGES: usize = 32768;

static mut BITMAP: [u64; MAX_PAGES / 64] = [0; MAX_PAGES / 64];
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE: AtomicUsize = AtomicUsize::new(0);
static START_PAGE: AtomicUsize = AtomicUsize::new(0);

const MAX_MEMORY_RANGES: usize = 64;
static mut MEMORY_RANGES: [(u64, u64, u32); MAX_MEMORY_RANGES] = [(0, 0, 0); MAX_MEMORY_RANGES];
static MEMORY_RANGE_COUNT: AtomicUsize = AtomicUsize::new(0);
static USABLE_RAM_BYTES: AtomicUsize = AtomicUsize::new(0);
static HIGHEST_PHYS_ADDR: AtomicUsize = AtomicUsize::new(0);
static MBI_ADDR: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn discover_multiboot(mbi: usize) {
    MBI_ADDR.store(mbi, Ordering::SeqCst);
    MEMORY_RANGE_COUNT.store(0, Ordering::SeqCst);
    USABLE_RAM_BYTES.store(0, Ordering::SeqCst);
    HIGHEST_PHYS_ADDR.store(0, Ordering::SeqCst);
    if mbi == 0 { return; }
    let total_size = core::ptr::read_unaligned(mbi as *const u32) as usize;
    if total_size < 16 { return; }
    let mut off = 8usize;
    while off + 8 <= total_size {
        let tag = (mbi + off) as *const u32;
        let tag_type = core::ptr::read_unaligned(tag);
        let tag_size = core::ptr::read_unaligned(tag.add(1)) as usize;
        if tag_size < 8 || off + tag_size > total_size { break; }
        if tag_type == 6 && tag_size >= 16 {
            let entry_size = core::ptr::read_unaligned((mbi + off + 8) as *const u32) as usize;
            if entry_size >= 24 {
                let mut p = off + 16;
                while p + entry_size <= off + tag_size {
                    let base = core::ptr::read_unaligned((mbi + p) as *const u64);
                    let len = core::ptr::read_unaligned((mbi + p + 8) as *const u64);
                    let kind = core::ptr::read_unaligned((mbi + p + 16) as *const u32);
                    let end = base.saturating_add(len);
                    if end > HIGHEST_PHYS_ADDR.load(Ordering::SeqCst) as u64 { HIGHEST_PHYS_ADDR.store(end.min(usize::MAX as u64) as usize, Ordering::SeqCst); }
                    let idx = MEMORY_RANGE_COUNT.load(Ordering::SeqCst);
                    if idx < MAX_MEMORY_RANGES { MEMORY_RANGES[idx] = (base, len, kind); MEMORY_RANGE_COUNT.store(idx + 1, Ordering::SeqCst); }
                    if kind == 1 { USABLE_RAM_BYTES.store(USABLE_RAM_BYTES.load(Ordering::SeqCst).saturating_add(len.min(usize::MAX as u64) as usize), Ordering::SeqCst); }
                    p += entry_size;
                }
            }
        }
        off = (off + tag_size + 7) & !7;
    }
}
pub fn memory_map_count() -> usize { MEMORY_RANGE_COUNT.load(Ordering::SeqCst) }
pub fn usable_ram_bytes() -> usize { USABLE_RAM_BYTES.load(Ordering::SeqCst) }
pub fn highest_phys_addr() -> usize { HIGHEST_PHYS_ADDR.load(Ordering::SeqCst) }
pub fn multiboot_addr() -> usize { MBI_ADDR.load(Ordering::SeqCst) }
pub fn memory_range(index: usize) -> Option<(u64, u64, u32)> { if index >= MEMORY_RANGE_COUNT.load(Ordering::SeqCst) || index >= MAX_MEMORY_RANGES { return None; } unsafe { Some(MEMORY_RANGES[index]) } }

pub fn init(start: usize, size: usize) {
    let start_page = start / PAGE_SIZE;
    let count = size / PAGE_SIZE;
    let usable = if count > MAX_PAGES.saturating_sub(start_page) { MAX_PAGES - start_page } else { count };
    unsafe {
        let mut i = 0;
        while i < usable {
            let page = start_page + i;
            let idx = page / 64;
            let bit = page % 64;
            if idx < BITMAP.len() {
                BITMAP[idx] |= 1u64 << bit;
            }
            i += 1;
        }
    }
    START_PAGE.store(start_page, Ordering::SeqCst);
    TOTAL.store(usable, Ordering::SeqCst);
    FREE.store(usable, Ordering::SeqCst);
}

pub fn alloc_page() -> Option<usize> {
    if FREE.load(Ordering::SeqCst) == 0 { return None; }
    let total = TOTAL.load(Ordering::SeqCst);
    unsafe {
        let start_page = START_PAGE.load(Ordering::SeqCst);
        let first_idx = start_page / 64;
        let end_idx = (start_page + total + 63) / 64;
        let mut idx = first_idx;
        while idx < end_idx && idx < BITMAP.len() {
            let word = BITMAP[idx];
            if word != 0 {
                let bit = word.trailing_zeros() as usize;
                BITMAP[idx] &= !(1u64 << bit);
                FREE.fetch_sub(1, Ordering::SeqCst);
                return Some((START_PAGE.load(Ordering::SeqCst) + idx * 64 + bit) * PAGE_SIZE);
            }
            idx += 1;
        }
    }
    None
}

pub fn free_page(addr: usize) {
    let page = addr / PAGE_SIZE;
    let idx = page / 64;
    let bit = page % 64;
    unsafe {
        if idx < BITMAP.len() && (BITMAP[idx] & (1u64 << bit)) == 0 {
            BITMAP[idx] |= 1u64 << bit;
            FREE.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub fn free_count() -> usize { FREE.load(Ordering::SeqCst) }
pub fn total_count() -> usize { TOTAL.load(Ordering::SeqCst) }

/// Allocate `count` contiguous physical pages. Returns physical address of first page.
pub fn alloc_pages(count: usize) -> Option<usize> {
    if count == 0 { return None; }
    if count == 1 { return alloc_page(); }
    let total = TOTAL.load(Ordering::SeqCst);
    unsafe {
        let mut start = 0usize;
        while start + count <= total && start + count <= MAX_PAGES.saturating_sub(START_PAGE.load(Ordering::SeqCst)) {
            let mut ok = true;
            let mut i = 0usize;
            while i < count {
                let page = START_PAGE.load(Ordering::SeqCst) + start + i;
                let idx = page / 64;
                let bit = page % 64;
                if idx >= BITMAP.len() || (BITMAP[idx] & (1u64 << bit)) == 0 {
                    ok = false;
                    break;
                }
                i += 1;
            }
            if ok {
                i = 0;
                while i < count {
                    let page = START_PAGE.load(Ordering::SeqCst) + start + i;
                    let idx = page / 64;
                    let bit = page % 64;
                    BITMAP[idx] &= !(1u64 << bit);
                    i += 1;
                }
                FREE.fetch_sub(count, Ordering::SeqCst);
                return Some((START_PAGE.load(Ordering::SeqCst) + start) * PAGE_SIZE);
            }
            start += 1;
        }
    }
    None
}

pub fn zero_pages(phys: usize, count: usize) {
    unsafe {
        let p = phys as *mut u8;
        let mut i = 0usize;
        while i < count * PAGE_SIZE {
            *p.add(i) = 0;
            i += 1;
        }
    }
}

