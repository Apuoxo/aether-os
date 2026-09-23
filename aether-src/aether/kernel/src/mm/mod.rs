//! Physical Memory Manager + paging re-export

use core::sync::atomic::{AtomicUsize, Ordering};

pub mod paging;

const PAGE_SIZE: usize = 4096;
const MAX_PAGES: usize = 32768;

static mut BITMAP: [u64; MAX_PAGES / 64] = [0; MAX_PAGES / 64];
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE: AtomicUsize = AtomicUsize::new(0);

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
    TOTAL.store(usable, Ordering::SeqCst);
    FREE.store(usable, Ordering::SeqCst);
}

pub fn alloc_page() -> Option<usize> {
    if FREE.load(Ordering::SeqCst) == 0 { return None; }
    let total = TOTAL.load(Ordering::SeqCst);
    unsafe {
        let mut idx = 0;
        while idx < (total + 63) / 64 && idx < BITMAP.len() {
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
        while start + count <= total && start + count <= MAX_PAGES {
            let mut ok = true;
            let mut i = 0usize;
            while i < count {
                let page = start + i;
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

