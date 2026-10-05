//! Physical Memory Manager + paging re-export

use core::sync::atomic::{AtomicUsize, Ordering};

pub mod paging;

const PAGE_SIZE: usize = 4096;
// Track physical page numbers up to 4 GiB.  The bitmap is sparse-aware: only
// pages explicitly reported as Multiboot type-1 usable are made allocatable.
const MAX_PAGES: usize = 1_048_576;

// BITMAP: 1 = free managed page, 0 = allocated/unavailable.
// MANAGED: 1 = page belongs to a usable region accepted by the PMM.
static mut BITMAP: [u64; MAX_PAGES / 64] = [0; MAX_PAGES / 64];
static mut MANAGED: [u64; MAX_PAGES / 64] = [0; MAX_PAGES / 64];
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn page_limit() -> usize { MAX_PAGES }

unsafe fn mark_free_range(start: usize, end: usize) {
    let first = (start + PAGE_SIZE - 1) / PAGE_SIZE;
    let last = end / PAGE_SIZE;
    let limit = page_limit();
    let mut page = first;
    while page < last && page < limit {
        let idx = page / 64;
        let bit = page % 64;
        let mask = 1u64 << bit;
        if (MANAGED[idx] & mask) == 0 {
            MANAGED[idx] |= mask;
            BITMAP[idx] |= mask;
            TOTAL.fetch_add(1, Ordering::SeqCst);
            FREE.fetch_add(1, Ordering::SeqCst);
        }
        page += 1;
    }
}

unsafe fn reserve_range(start: usize, end: usize) {
    let first = start / PAGE_SIZE;
    let last = (end + PAGE_SIZE - 1) / PAGE_SIZE;
    let limit = page_limit();
    let mut page = first;
    while page < last && page < limit {
        let idx = page / 64;
        let bit = page % 64;
        let mask = 1u64 << bit;
        if (MANAGED[idx] & mask) != 0 && (BITMAP[idx] & mask) != 0 {
            BITMAP[idx] &= !mask;
            FREE.fetch_sub(1, Ordering::SeqCst);
        }
        page += 1;
    }
}

pub fn init(start: usize, size: usize) {
    // Compatibility entry point for callers that have a known safe contiguous
    // region. It no longer invents extra RAM when the caller supplies less.
    unsafe {
        let mut i = 0usize;
        while i < BITMAP.len() {
            BITMAP[i] = 0;
            MANAGED[i] = 0;
            i += 1;
        }
    }
    TOTAL.store(0, Ordering::SeqCst);
    FREE.store(0, Ordering::SeqCst);
    unsafe { mark_free_range(start, start.saturating_add(size)); }
}

/// Initialize from the Multiboot2 memory map. Only type-1 usable regions are
/// managed, and the Multiboot information block itself is reserved afterwards.
/// This removes the old artificial contiguous-RAM assumption.
pub fn init_from_multiboot(mbi: usize, kernel_end: usize) -> usize {
    unsafe {
        let mut i = 0usize;
        while i < BITMAP.len() {
            BITMAP[i] = 0;
            MANAGED[i] = 0;
            i += 1;
        }
    }
    TOTAL.store(0, Ordering::SeqCst);
    FREE.store(0, Ordering::SeqCst);
    if mbi == 0 { return 0; }

    unsafe {
        let total_size = core::ptr::read_unaligned(mbi as *const u32) as usize;
        if total_size < 16 || total_size > 0x100000 { return 0; }

        let mut off = 8usize;
        while off + 8 <= total_size {
            let tag_type = core::ptr::read_unaligned((mbi + off) as *const u32);
            let tag_size = core::ptr::read_unaligned((mbi + off + 4) as *const u32) as usize;
            if tag_size < 8 || off + tag_size > total_size { break; }
            if tag_type == 0 { break; }

            if tag_type == 6 && tag_size >= 16 {
                let entry_size = core::ptr::read_unaligned((mbi + off + 8) as *const u32) as usize;
                if entry_size < 24 { break; }
                let mut p = off + 16;
                while p + entry_size <= off + tag_size {
                    let addr = core::ptr::read_unaligned((mbi + p) as *const u64) as usize;
                    let len = core::ptr::read_unaligned((mbi + p + 8) as *const u64) as usize;
                    let typ = core::ptr::read_unaligned((mbi + p + 16) as *const u32);
                    if typ == 1 && len != 0 {
                        let mut begin = addr;
                        let end = addr.saturating_add(len);
                        if begin < kernel_end { begin = kernel_end; }
                        if begin < end { mark_free_range(begin, end); }
                    }
                    p += entry_size;
                }
                break;
            }
            off = (off + tag_size + 7) & !7;
        }

        // The Multiboot information block is itself stored in RAM. Reserve it
        // even when the firmware's memory map labels that physical range usable.
        reserve_range(mbi, mbi.saturating_add(total_size));
    }

    FREE.load(Ordering::SeqCst)
}

pub fn alloc_page() -> Option<usize> {
    if FREE.load(Ordering::SeqCst) == 0 { return None; }
    unsafe {
        let mut idx = 0usize;
        while idx < BITMAP.len() {
            let word = BITMAP[idx];
            if word != 0 {
                let bit = word.trailing_zeros() as usize;
                BITMAP[idx] &= !(1u64 << bit);
                FREE.fetch_sub(1, Ordering::SeqCst);
                return Some((idx * 64 + bit) * PAGE_SIZE);
            }
            idx += 1;
        }
    }
    None
}

pub fn free_page(addr: usize) {
    let page = addr / PAGE_SIZE;
    if page >= MAX_PAGES { return; }
    let idx = page / 64;
    let bit = page % 64;
    unsafe {
        let mask = 1u64 << bit;
        if (MANAGED[idx] & mask) != 0 && (BITMAP[idx] & mask) == 0 {
            BITMAP[idx] |= mask;
            FREE.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub fn free_count() -> usize { FREE.load(Ordering::SeqCst) }
pub fn total_count() -> usize { TOTAL.load(Ordering::SeqCst) }

/// Allocate `count` contiguous physical pages from managed usable memory.
pub fn alloc_pages(count: usize) -> Option<usize> {
    if count == 0 { return None; }
    if count == 1 { return alloc_page(); }
    unsafe {
        let mut start = 0usize;
        while start + count <= MAX_PAGES {
            let mut ok = true;
            let mut i = 0usize;
            while i < count {
                let page = start + i;
                let idx = page / 64;
                let bit = page % 64;
                let mask = 1u64 << bit;
                if (MANAGED[idx] & mask) == 0 || (BITMAP[idx] & mask) == 0 {
                    ok = false;
                    break;
                }
                i += 1;
            }
            if ok {
                i = 0;
                while i < count {
                    let page = start + i;
                    let idx = page / 64;
                    let bit = page % 64;
                    BITMAP[idx] &= !(1u64 << bit);
                    i += 1;
                }
                FREE.fetch_sub(count, Ordering::SeqCst);
                return Some(start * PAGE_SIZE);
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
