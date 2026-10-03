//! Physical Memory Manager + paging re-export
// Stage 1: Multiboot2 memory map, full physical-range bitmap, reservations.

use core::sync::atomic::{AtomicUsize, Ordering};

pub mod paging;

const PAGE_SIZE: usize = 4096;
const MAX_ALLOC_PHYS: u64 = 1u64 << 30;
const LEGACY_ALLOC_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RESERVED_RANGES: usize = 256;

#[derive(Copy, Clone)]
struct PhysRange {
    start: u64,
    end: u64,
}

impl PhysRange {
    const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }
}

static mut BITMAP: *mut u64 = core::ptr::null_mut();
static mut BITMAP_WORDS: usize = 0;
static mut MAX_PAGES: usize = 0;
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE: AtomicUsize = AtomicUsize::new(0);
static START_PAGE: AtomicUsize = AtomicUsize::new(0);

fn align_up_page(v: u64) -> Option<u64> {
    v.checked_add((PAGE_SIZE as u64) - 1).map(|x| x & !((PAGE_SIZE as u64) - 1))
}

fn align_down_page(v: u64) -> u64 {
    v & !((PAGE_SIZE as u64) - 1)
}

unsafe fn read_u32(p: u64) -> u32 {
    core::ptr::read_unaligned(p as *const u32)
}

unsafe fn read_u64(p: u64) -> u64 {
    core::ptr::read_unaligned(p as *const u64)
}

unsafe fn add_reserved(
    ranges: &mut [PhysRange; MAX_RESERVED_RANGES],
    count: &mut usize,
    start: u64,
    end: u64,
) -> bool {
    let start = align_down_page(start);
    let end = align_up_page(end).unwrap_or(u64::MAX & !((PAGE_SIZE as u64) - 1));
    if start >= end {
        return true;
    }
    if *count >= MAX_RESERVED_RANGES {
        return false;
    }
    ranges[*count] = PhysRange { start, end };
    *count += 1;
    true
}

unsafe fn parse_multiboot(
    mbi: u64,
    reserved: &mut [PhysRange; MAX_RESERVED_RANGES],
    reserved_count: &mut usize,
) -> Option<u64> {
    if mbi == 0 {
        return None;
    }
    let total_size = read_u32(mbi) as u64;
    if total_size < 16 {
        return None;
    }

    if !add_reserved(reserved, reserved_count, mbi, mbi.checked_add(total_size)?) {
        return None;
    }

    let mut max_usable_end = 0u64;
    let mut off = 8u64;
    while off + 8 <= total_size {
        let tag = mbi.checked_add(off)?;
        let tag_type = read_u32(tag);
        let tag_size = read_u32(tag + 4) as u64;
        if tag_size < 8 || off.checked_add(tag_size)? > total_size {
            return None;
        }
        if tag_type == 0 {
            break;
        }

        match tag_type {
            6 => {
                if tag_size < 16 {
                    return None;
                }
                let entry_size = read_u32(tag + 8) as u64;
                if entry_size < 24 {
                    return None;
                }
                let mut p = 16u64;
                while p.checked_add(entry_size)? <= tag_size {
                    let e = tag + p;
                    let base = read_u64(e);
                    let len = read_u64(e + 8);
                    let typ = read_u32(e + 16);
                    if typ == 1 && len != 0 {
                        let end = base.checked_add(len)?;
                        let start = align_up_page(base)?;
                        let end = align_down_page(end);
                        if start < end && end > max_usable_end {
                            max_usable_end = end;
                        }
                    }
                    p = p.checked_add(entry_size)?;
                }
            }
            3 => {
                if tag_size >= 16 {
                    if !add_reserved(
                        reserved,
                        reserved_count,
                        read_u32(tag + 8) as u64,
                        read_u32(tag + 12) as u64,
                    ) {
                        return None;
                    }
                }
            }
            8 => {
                if tag_size >= 32 {
                    let addr = read_u64(tag + 8);
                    let pitch = read_u32(tag + 16) as u64;
                    let height = read_u32(tag + 24) as u64;
                    let bytes = pitch.checked_mul(height).unwrap_or(0);
                    if addr != 0 && bytes != 0 {
                        if !add_reserved(reserved, reserved_count, addr, addr.checked_add(bytes)?) {
                            return None;
                        }
                    }
                }
            }
            _ => {}
        }

        off = (off.checked_add(tag_size)?.checked_add(7)?) & !7;
    }

    Some(max_usable_end)
}

unsafe fn find_bitmap_area(
    mbi: u64,
    reserved: &[PhysRange; MAX_RESERVED_RANGES],
    reserved_count: usize,
    bytes: u64,
) -> Option<u64> {
    let need = align_up_page(bytes)?;
    let total_size = read_u32(mbi) as u64;
    let mut off = 8u64;

    while off + 8 <= total_size {
        let tag = mbi.checked_add(off)?;
        let tag_type = read_u32(tag);
        let tag_size = read_u32(tag + 4) as u64;
        if tag_size < 8 || off.checked_add(tag_size)? > total_size {
            return None;
        }
        if tag_type == 0 {
            break;
        }

        if tag_type == 6 && tag_size >= 16 {
            let entry_size = read_u32(tag + 8) as u64;
            if entry_size < 24 {
                return None;
            }
            let mut p = 16u64;
            while p.checked_add(entry_size)? <= tag_size {
                let e = tag + p;
                let base = read_u64(e);
                let len = read_u64(e + 8);
                let typ = read_u32(e + 16);

                if typ == 1 && len != 0 {
                    let mut candidate = align_up_page(base)?;
                    let end = core::cmp::min(
                        align_down_page(base.checked_add(len)?),
                        MAX_ALLOC_PHYS,
                    );

                    while candidate < end {
                        let candidate_end = candidate.checked_add(need)?;
                        if candidate_end > end {
                            break;
                        }

                        let mut conflict = false;
                        let mut r = 0usize;
                        while r < reserved_count {
                            let rr = reserved[r];
                            if candidate < rr.end && candidate_end > rr.start {
                                conflict = true;
                                break;
                            }
                            r += 1;
                        }

                        if !conflict && candidate >= 0x0010_0000 {
                            return Some(candidate);
                        }
                        candidate = candidate.checked_add(PAGE_SIZE as u64)?;
                    }
                }

                p = p.checked_add(entry_size)?;
            }
            break;
        }

        off = (off.checked_add(tag_size)?.checked_add(7)?) & !7;
    }

    None
}

unsafe fn set_free_bit(page: usize) {
    if page >= MAX_PAGES || BITMAP.is_null() {
        return;
    }
    *BITMAP.add(page / 64) |= 1u64 << (page % 64);
}

unsafe fn clear_free_bit(page: usize) {
    if page >= MAX_PAGES || BITMAP.is_null() {
        return;
    }
    *BITMAP.add(page / 64) &= !(1u64 << (page % 64));
}

unsafe fn is_free_bit(page: usize) -> bool {
    if page >= MAX_PAGES || BITMAP.is_null() {
        return false;
    }
    (*BITMAP.add(page / 64) & (1u64 << (page % 64))) != 0
}

unsafe fn mark_usable_range(start: u64, end: u64) {
    let start = match align_up_page(start) {
        Some(v) => v,
        None => return,
    };
    let end = align_down_page(end);
    if start >= end {
        return;
    }

    let mut pa = start;
    while pa < end {
        set_free_bit((pa / PAGE_SIZE as u64) as usize);
        pa = match pa.checked_add(PAGE_SIZE as u64) {
            Some(v) => v,
            None => break,
        };
    }
}

unsafe fn reserve_range(start: u64, end: u64) {
    let start = align_down_page(start);
    let end = align_up_page(end).unwrap_or(u64::MAX & !((PAGE_SIZE as u64) - 1));
    if start >= end {
        return;
    }

    let mut pa = start;
    while pa < end {
        clear_free_bit((pa / PAGE_SIZE as u64) as usize);
        pa = match pa.checked_add(PAGE_SIZE as u64) {
            Some(v) => v,
            None => break,
        };
    }
}

unsafe fn count_free_pages() -> usize {
    let mut total = 0usize;
    let mut page = 0usize;
    while page < MAX_PAGES {
        if is_free_bit(page) {
            total += 1;
        }
        page += 1;
    }
    total
}

/// Initialize the PMM from the Multiboot2 memory map.
pub fn init(mbi: usize, kernel_end: usize) -> bool {
    unsafe {
        let mut reserved = [PhysRange::empty(); MAX_RESERVED_RANGES];
        let mut reserved_count = 0usize;

        // First MiB is never issued; keep one page below 1 MiB explicitly
        // reserved for the future SMP trampoline.
        if !add_reserved(&mut reserved, &mut reserved_count, 0, 0x0010_0000) {
            return false;
        }
        if !add_reserved(&mut reserved, &mut reserved_count, 0x000F_F000, 0x0010_0000) {
            return false;
        }

        // Linker places the kernel at 1 MiB and __kernel_end includes .bss.
        if !add_reserved(
            &mut reserved,
            &mut reserved_count,
            0x0010_0000,
            kernel_end as u64,
        ) {
            return false;
        }

        let max_usable_end = match parse_multiboot(mbi as u64, &mut reserved, &mut reserved_count) {
            Some(v) if v != 0 => v,
            _ => {
                crate::serial::write_str("[PMM] no usable Multiboot memory map\n");
                return false;
            }
        };

        let max_pages_u64 = (max_usable_end / PAGE_SIZE as u64).max(1);
        if max_pages_u64 > usize::MAX as u64 {
            return false;
        }

        let max_pages = max_pages_u64 as usize;
        let bitmap_words = (max_pages + 63) / 64;
        let bitmap_bytes = match (bitmap_words as u64).checked_mul(8) {
            Some(v) => v,
            None => return false,
        };

        let bitmap_phys = match find_bitmap_area(
            mbi as u64,
            &reserved,
            reserved_count,
            bitmap_bytes,
        ) {
            Some(v) => v,
            None => {
                crate::serial::write_str("[PMM] no mapped usable area for bitmap\n");
                return false;
            }
        };

        BITMAP = bitmap_phys as *mut u64;
        BITMAP_WORDS = bitmap_words;
        MAX_PAGES = max_pages;
        // Keep the physical allocation window stable while Stage 1 expands the
        // PMM's tracked/usable RAM map. This preserves the pre-Stage-1 source
        // range for DMA/page-table consumers; higher RAM remains tracked but is
        // not issued until the allocator/mapping policy is deliberately widened.
        let alloc_start = match align_up_page(kernel_end as u64) {
            Some(v) => v,
            None => return false,
        };
        let alloc_start_page = (alloc_start / PAGE_SIZE as u64) as usize;
        START_PAGE.store(alloc_start_page, Ordering::SeqCst);

        if !add_reserved(
            &mut reserved,
            &mut reserved_count,
            bitmap_phys,
            bitmap_phys + bitmap_bytes,
        ) {
            return false;
        }

        let mut i = 0usize;
        while i < BITMAP_WORDS {
            *BITMAP.add(i) = 0;
            i += 1;
        }

        // Mark every aligned type-1 record first. Records may be unsorted or
        // overlapping; applying reservations afterwards makes the result stable.
        let total_size = read_u32(mbi as u64) as u64;
        let mut off = 8u64;
        while off + 8 <= total_size {
            let tag = match (mbi as u64).checked_add(off) {
                Some(v) => v,
                None => return false,
            };
            let tag_type = read_u32(tag);
            let tag_size = read_u32(tag + 4) as u64;
            if tag_size < 8 || off.checked_add(tag_size).map_or(true, |v| v > total_size) {
                return false;
            }
            if tag_type == 0 {
                break;
            }

            if tag_type == 6 && tag_size >= 16 {
                let entry_size = read_u32(tag + 8) as u64;
                if entry_size < 24 {
                    return false;
                }
                let mut p = 16u64;
                while p.checked_add(entry_size).map_or(false, |v| v <= tag_size) {
                    let e = tag + p;
                    let base = read_u64(e);
                    let len = read_u64(e + 8);
                    let typ = read_u32(e + 16);
                    if typ == 1 && len != 0 {
                        if let Some(end) = base.checked_add(len) {
                            mark_usable_range(base, end);
                        }
                    }
                    p = match p.checked_add(entry_size) {
                        Some(v) => v,
                        None => return false,
                    };
                }
                break;
            }

            off = match off.checked_add(tag_size).and_then(|v| v.checked_add(7)) {
                Some(v) => v & !7,
                None => return false,
            };
        }

        i = 0;
        while i < reserved_count {
            reserve_range(reserved[i].start, reserved[i].end);
            i += 1;
        }

        if MAX_PAGES % 64 != 0 {
            let last = MAX_PAGES / 64;
            let valid = MAX_PAGES % 64;
            *BITMAP.add(last) &= (1u64 << valid) - 1;
        }

        let free = count_free_pages();
        TOTAL.store(free, Ordering::SeqCst);
        FREE.store(free, Ordering::SeqCst);

        crate::serial::write_str("[PMM] total=");
        crate::serial::write_usize((max_usable_end / (1024 * 1024)) as usize);
        crate::serial::write_str(" MiB usable=");
        crate::serial::write_usize((free * PAGE_SIZE) / (1024 * 1024));
        crate::serial::write_str(" MiB bitmap=");
        crate::serial::write_usize((bitmap_bytes / 1024) as usize);
        crate::serial::write_str(" KiB\n");
        crate::serial::write_str("[PMM] alloc_window=");
        crate::serial::write_hex(alloc_start as usize);
        crate::serial::write_str("..");
        let alloc_end = core::cmp::min(
            alloc_start.saturating_add(LEGACY_ALLOC_BYTES),
            MAX_ALLOC_PHYS,
        );
        crate::serial::write_hex(alloc_end as usize);
        crate::serial::write_str(" bitmap_phys=");
        crate::serial::write_hex(bitmap_phys as usize);
        crate::serial::write_str(" pages=");
        crate::serial::write_usize(MAX_PAGES);
        crate::serial::write_str(" alloc_limit=1GiB\n");
        crate::serial::write_str("[PMM] reservations=kernel+MBI+modules+LFB+ACPI+first1MiB+bitmap\n");
        crate::serial::write_str("[PMM] kernel image reservation includes linked ELF blobs\n");

        true
    }
}

pub fn alloc_page_phys() -> Option<u64> {
    if FREE.load(Ordering::SeqCst) == 0 {
        return None;
    }

    unsafe {
        let alloc_start_page = START_PAGE.load(Ordering::SeqCst);
        let legacy_pages = (LEGACY_ALLOC_BYTES / PAGE_SIZE as u64) as usize;
        let alloc_end_page = core::cmp::min(
            MAX_PAGES,
            core::cmp::min(
                alloc_start_page.saturating_add(legacy_pages),
                (MAX_ALLOC_PHYS / PAGE_SIZE as u64) as usize,
            ),
        );
        let mut page = alloc_start_page;

        while page < alloc_end_page {
            if is_free_bit(page) {
                clear_free_bit(page);
                FREE.fetch_sub(1, Ordering::SeqCst);
                return Some((page as u64) * PAGE_SIZE as u64);
            }
            page += 1;
        }
    }
    None
}

/// Compatibility wrapper for current x86_64 callers.
pub fn alloc_page() -> Option<usize> {
    alloc_page_phys().map(|p| p as usize)
}

pub fn free_page(addr: usize) {
    let page = ((addr as u64) / PAGE_SIZE as u64) as usize;
    unsafe {
        if page < MAX_PAGES && !is_free_bit(page) {
            set_free_bit(page);
            FREE.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub fn free_count() -> usize {
    FREE.load(Ordering::SeqCst)
}

pub fn total_count() -> usize {
    TOTAL.load(Ordering::SeqCst)
}

pub fn alloc_pages(count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    if count == 1 {
        return alloc_page();
    }

    unsafe {
        let alloc_start_page = START_PAGE.load(Ordering::SeqCst);
        let legacy_pages = (LEGACY_ALLOC_BYTES / PAGE_SIZE as u64) as usize;
        let alloc_end_page = core::cmp::min(
            MAX_PAGES,
            core::cmp::min(
                alloc_start_page.saturating_add(legacy_pages),
                (MAX_ALLOC_PHYS / PAGE_SIZE as u64) as usize,
            ),
        );
        let mut start = alloc_start_page;

        while start.checked_add(count).map_or(false, |v| v <= alloc_end_page) {
            let mut ok = true;
            let mut i = 0usize;
            while i < count {
                if !is_free_bit(start + i) {
                    ok = false;
                    break;
                }
                i += 1;
            }

            if ok {
                i = 0;
                while i < count {
                    clear_free_bit(start + i);
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
