//! Physical Memory Manager + paging re-export

use core::sync::atomic::{AtomicUsize, Ordering};

pub mod paging;

const PAGE_SIZE: usize = 4096;
const MAX_FREE_RANGES: usize = 256;
const MAX_MEMORY_RANGES: usize = 64;
const MAX_BOOT_RESERVATIONS: usize = 32;

static mut FREE_RANGES: [(u64, u64); MAX_FREE_RANGES] = [(0, 0); MAX_FREE_RANGES];
static FREE_RANGE_COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE: AtomicUsize = AtomicUsize::new(0);

static mut MEMORY_RANGES: [(u64, u64, u32); MAX_MEMORY_RANGES] = [(0, 0, 0); MAX_MEMORY_RANGES];
static MEMORY_RANGE_COUNT: AtomicUsize = AtomicUsize::new(0);
static USABLE_RAM_BYTES: AtomicUsize = AtomicUsize::new(0);
static HIGHEST_PHYS_ADDR: AtomicUsize = AtomicUsize::new(0);
static MBI_ADDR: AtomicUsize = AtomicUsize::new(0);
static MBI_SIZE: AtomicUsize = AtomicUsize::new(0);
static mut BOOT_RESERVATIONS: [(u64, u64); MAX_BOOT_RESERVATIONS] = [(0, 0); MAX_BOOT_RESERVATIONS];
static BOOT_RESERVATION_COUNT: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn discover_multiboot(mbi: usize) {
    MBI_ADDR.store(mbi, Ordering::SeqCst);
    MBI_SIZE.store(0, Ordering::SeqCst);
    MEMORY_RANGE_COUNT.store(0, Ordering::SeqCst);
    BOOT_RESERVATION_COUNT.store(0, Ordering::SeqCst);
    USABLE_RAM_BYTES.store(0, Ordering::SeqCst);
    HIGHEST_PHYS_ADDR.store(0, Ordering::SeqCst);
    if mbi == 0 { return; }

    let total_size = core::ptr::read_unaligned(mbi as *const u32) as usize;
    if total_size < 16 { return; }
    MBI_SIZE.store(total_size, Ordering::SeqCst);
    reserve_boot_range(mbi as u64, total_size as u64);

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
                    if end > HIGHEST_PHYS_ADDR.load(Ordering::SeqCst) as u64 {
                        HIGHEST_PHYS_ADDR.store(end.min(usize::MAX as u64) as usize, Ordering::SeqCst);
                    }
                    let idx = MEMORY_RANGE_COUNT.load(Ordering::SeqCst);
                    if idx < MAX_MEMORY_RANGES {
                        MEMORY_RANGES[idx] = (base, len, kind);
                        MEMORY_RANGE_COUNT.store(idx + 1, Ordering::SeqCst);
                    }
                    if kind == 1 {
                        USABLE_RAM_BYTES.store(
                            USABLE_RAM_BYTES.load(Ordering::SeqCst).saturating_add(len.min(usize::MAX as u64) as usize),
                            Ordering::SeqCst
                        );
                    }
                    p += entry_size;
                }
            }
        } else if tag_type == 3 && tag_size >= 16 {
            let module_start = core::ptr::read_unaligned((mbi + off + 8) as *const u32) as u64;
            let module_end = core::ptr::read_unaligned((mbi + off + 12) as *const u32) as u64;
            if module_end > module_start {
                reserve_boot_range(module_start, module_end - module_start);
            }
        }

        off = (off + tag_size + 7) & !7;
    }
}

unsafe fn reserve_boot_range(base: u64, len: u64) {
    if len == 0 { return; }
    let idx = BOOT_RESERVATION_COUNT.load(Ordering::SeqCst);
    if idx < MAX_BOOT_RESERVATIONS {
        BOOT_RESERVATIONS[idx] = (base, len);
        BOOT_RESERVATION_COUNT.store(idx + 1, Ordering::SeqCst);
    }
}

pub fn memory_map_count() -> usize { MEMORY_RANGE_COUNT.load(Ordering::SeqCst) }
pub fn usable_ram_bytes() -> usize { USABLE_RAM_BYTES.load(Ordering::SeqCst) }
pub fn highest_phys_addr() -> usize { HIGHEST_PHYS_ADDR.load(Ordering::SeqCst) }
pub fn multiboot_addr() -> usize { MBI_ADDR.load(Ordering::SeqCst) }

pub fn memory_range(index: usize) -> Option<(u64, u64, u32)> {
    if index >= MEMORY_RANGE_COUNT.load(Ordering::SeqCst) || index >= MAX_MEMORY_RANGES { return None; }
    unsafe { Some(MEMORY_RANGES[index]) }
}

pub fn total_bytes() -> usize { TOTAL.load(Ordering::SeqCst).saturating_mul(PAGE_SIZE) }
pub fn free_bytes() -> usize { FREE.load(Ordering::SeqCst).saturating_mul(PAGE_SIZE) }

unsafe fn add_free_range(base: u64, len: u64, floor: u64) {
    let start = base.max(floor);
    let end = base.saturating_add(len);
    if end <= start { return; }
    let aligned_start = (start + (PAGE_SIZE as u64 - 1)) & !(PAGE_SIZE as u64 - 1);
    let aligned_end = end & !(PAGE_SIZE as u64 - 1);
    if aligned_end <= aligned_start { return; }

    let mut pos = FREE_RANGE_COUNT.load(Ordering::SeqCst);
    if pos >= MAX_FREE_RANGES { return; }
    FREE_RANGES[pos] = (aligned_start, aligned_end - aligned_start);
    FREE_RANGE_COUNT.store(pos + 1, Ordering::SeqCst);
}

unsafe fn subtract_range(base: u64, len: u64) {
    let cut_start = base;
    let cut_end = base.saturating_add(len);
    if cut_end <= cut_start { return; }

    let count = FREE_RANGE_COUNT.load(Ordering::SeqCst);
    let mut i = 0usize;
    while i < count {
        let (start, length) = FREE_RANGES[i];
        let end = start.saturating_add(length);
        if cut_end <= start || cut_start >= end {
            i += 1;
            continue;
        }

        let left_len = cut_start.saturating_sub(start);
        let right_start = cut_end.max(start);
        let right_len = end.saturating_sub(right_start);

        if left_len != 0 && right_len != 0 && count < MAX_FREE_RANGES {
            FREE_RANGES[i] = (start, left_len);
            let mut j = count;
            while j > i + 1 {
                FREE_RANGES[j] = FREE_RANGES[j - 1];
                j -= 1;
            }
            FREE_RANGES[i + 1] = (right_start, right_len);
            FREE_RANGE_COUNT.store(count + 1, Ordering::SeqCst);
            return;
        } else if left_len != 0 {
            FREE_RANGES[i] = (start, left_len);
            i += 1;
        } else if right_len != 0 {
            FREE_RANGES[i] = (right_start, right_len);
            i += 1;
        } else {
            let mut j = i;
            while j + 1 < count {
                FREE_RANGES[j] = FREE_RANGES[j + 1];
                j += 1;
            }
            FREE_RANGES[count - 1] = (0, 0);
            FREE_RANGE_COUNT.store(count - 1, Ordering::SeqCst);
        }
    }
}

unsafe fn merge_free_ranges() {
    let mut count = FREE_RANGE_COUNT.load(Ordering::SeqCst);
    let mut i = 0usize;
    while i < count {
        let mut j = i + 1;
        while j < count {
            if FREE_RANGES[j].0 < FREE_RANGES[i].0 {
                let tmp = FREE_RANGES[i];
                FREE_RANGES[i] = FREE_RANGES[j];
                FREE_RANGES[j] = tmp;
            }
            j += 1;
        }
        i += 1;
    }

    i = 0;
    while i + 1 < count {
        let end = FREE_RANGES[i].0.saturating_add(FREE_RANGES[i].1);
        if end >= FREE_RANGES[i + 1].0 {
            let next_end = FREE_RANGES[i + 1].0.saturating_add(FREE_RANGES[i + 1].1);
            FREE_RANGES[i].1 = next_end.max(end) - FREE_RANGES[i].0;
            let mut j = i + 1;
            while j + 1 < count {
                FREE_RANGES[j] = FREE_RANGES[j + 1];
                j += 1;
            }
            count -= 1;
            FREE_RANGES[count] = (0, 0);
        } else {
            i += 1;
        }
    }
    FREE_RANGE_COUNT.store(count, Ordering::SeqCst);
}

pub fn init(start: usize, _size: usize) {
    unsafe {
        FREE_RANGE_COUNT.store(0, Ordering::SeqCst);
        TOTAL.store(0, Ordering::SeqCst);
        FREE.store(0, Ordering::SeqCst);

        let n = MEMORY_RANGE_COUNT.load(Ordering::SeqCst);
        let floor = start as u64;
        let mut i = 0usize;
        while i < n {
            let (base, len, kind) = MEMORY_RANGES[i];
            if kind == 1 {
                add_free_range(base, len, floor);
            }
            i += 1;
        }

        let boot_n = BOOT_RESERVATION_COUNT.load(Ordering::SeqCst);
        let mut r = 0usize;
        while r < boot_n {
            let (base, len) = BOOT_RESERVATIONS[r];
            subtract_range(base, len);
            r += 1;
        }
        merge_free_ranges();

        let mut pages = 0usize;
        let mut k = 0usize;
        let count = FREE_RANGE_COUNT.load(Ordering::SeqCst);
        while k < count {
            pages = pages.saturating_add((FREE_RANGES[k].1 / PAGE_SIZE as u64).min(usize::MAX as u64) as usize);
            k += 1;
        }
        TOTAL.store(pages, Ordering::SeqCst);
        FREE.store(pages, Ordering::SeqCst);
    }
}

pub fn alloc_page() -> Option<usize> {
    if FREE.load(Ordering::SeqCst) == 0 { return None; }
    unsafe {
        let count = FREE_RANGE_COUNT.load(Ordering::SeqCst);
        let mut i = 0usize;
        while i < count {
            if FREE_RANGES[i].1 >= PAGE_SIZE as u64 {
                let addr = FREE_RANGES[i].0 as usize;
                FREE_RANGES[i].0 += PAGE_SIZE as u64;
                FREE_RANGES[i].1 -= PAGE_SIZE as u64;
                if FREE_RANGES[i].1 == 0 {
                    let mut j = i;
                    while j + 1 < count {
                        FREE_RANGES[j] = FREE_RANGES[j + 1];
                        j += 1;
                    }
                    FREE_RANGES[count - 1] = (0, 0);
                    FREE_RANGE_COUNT.store(count - 1, Ordering::SeqCst);
                }
                FREE.fetch_sub(1, Ordering::SeqCst);
                return Some(addr);
            }
            i += 1;
        }
    }
    None
}

pub fn alloc_pages(count: usize) -> Option<usize> {
    if count == 0 { return None; }
    if count == 1 { return alloc_page(); }
    let bytes = (count as u64).checked_mul(PAGE_SIZE as u64)?;
    unsafe {
        let range_count = FREE_RANGE_COUNT.load(Ordering::SeqCst);
        let mut i = 0usize;
        while i < range_count {
            if FREE_RANGES[i].1 >= bytes {
                let addr = FREE_RANGES[i].0 as usize;
                FREE_RANGES[i].0 += bytes;
                FREE_RANGES[i].1 -= bytes;
                if FREE_RANGES[i].1 == 0 {
                    let mut j = i;
                    while j + 1 < range_count {
                        FREE_RANGES[j] = FREE_RANGES[j + 1];
                        j += 1;
                    }
                    FREE_RANGES[range_count - 1] = (0, 0);
                    FREE_RANGE_COUNT.store(range_count - 1, Ordering::SeqCst);
                }
                FREE.fetch_sub(count, Ordering::SeqCst);
                return Some(addr);
            }
            i += 1;
        }
    }
    None
}

pub fn free_page(addr: usize) {
    if addr & (PAGE_SIZE - 1) != 0 { return; }
    let target = addr as u64;
    unsafe {
        let n = MEMORY_RANGE_COUNT.load(Ordering::SeqCst);
        let mut managed = false;
        let mut i = 0usize;
        while i < n {
            let (base, len, kind) = MEMORY_RANGES[i];
            let end = base.saturating_add(len);
            if kind == 1 && target >= base && target.saturating_add(PAGE_SIZE as u64) <= end {
                managed = true;
                break;
            }
            i += 1;
        }
        if !managed { return; }

        let boot_n = BOOT_RESERVATION_COUNT.load(Ordering::SeqCst);
        i = 0;
        while i < boot_n {
            let (base, len) = BOOT_RESERVATIONS[i];
            let end = base.saturating_add(len);
            if target < end && target.saturating_add(PAGE_SIZE as u64) > base { return; }
            i += 1;
        }

        let count = FREE_RANGE_COUNT.load(Ordering::SeqCst);
        i = 0;
        while i < count {
            let end = FREE_RANGES[i].0.saturating_add(FREE_RANGES[i].1);
            if target >= FREE_RANGES[i].0 && target < end { return; }
            i += 1;
        }

        if count >= MAX_FREE_RANGES { return; }
        FREE_RANGES[count] = (target, PAGE_SIZE as u64);
        FREE_RANGE_COUNT.store(count + 1, Ordering::SeqCst);
        merge_free_ranges();
        FREE.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn free_count() -> usize { FREE.load(Ordering::SeqCst) }
pub fn total_count() -> usize { TOTAL.load(Ordering::SeqCst) }


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

