//! Minimal NTFS READ-ONLY — root directory list + resident file read
//! NO writes. Incomplete vs full NTFS; enough for browse proof.

use crate::block;
use crate::part;
use crate::serial;

#[derive(Clone, Copy)]
pub struct NtfsEntry {
    pub name: [u8; 48],
    pub name_len: usize,
    pub size: u64,
    pub is_dir: bool,
    pub mft_ref: u32,
}

static mut MOUNTED: bool = false;
static mut DISK: u8 = 0;
static mut PART_LBA: u32 = 0;
static mut BPS: u16 = 512;
static mut SPC: u8 = 8;
static mut MFT_LCN: u64 = 0;
static mut MFT_REC_SIZE: u32 = 1024;
// Parsed $MFT:$DATA runlist. The old implementation assumed the whole MFT was contiguous.
// Real NTFS volumes may fragment $MFT, so record lookup must follow these runs.
static mut MFT_RUNS: [(u64, u64); 32] = [(0, 0); 32]; // (LCN, cluster_count)
static mut MFT_RUN_COUNT: usize = 0;
static mut INDEX_RUNS: [(u64, u64); 32] = [(0, 0); 32];
static mut INDEX_RUN_COUNT: usize = 0;
static mut INDEX_BLOCK_SIZE: u32 = 4096;
static mut INDEX_REAL_SIZE: u64 = 0;
static mut INDEX_BUFFERS_READ: usize = 0;
static mut ENTRIES: [NtfsEntry; 32] = [NtfsEntry {
    name: [0; 48], name_len: 0, size: 0, is_dir: false, mft_ref: 0,
}; 32];
static mut NENT: usize = 0;

fn read_lba(disk: u8, lba: u32, buf: &mut [u8; 512]) -> bool {
    block::read(disk, lba, 1, buf)
}

fn cluster_to_lba(lcn: u64) -> u32 {
    unsafe { PART_LBA.wrapping_add((lcn as u32).wrapping_mul(SPC as u32)) }
}

/// Mount a specific NTFS partition by partition-table index (READ-ONLY).
pub fn mount_partition(pi: usize) -> bool {
    unsafe {
        MOUNTED = false;
        NENT = 0;
    }
    if let Some(p) = part::get(pi) {
        if p.ptype == 0x07 {
            return try_mount(p.disk, p.lba_start);
        }
    }
    false
}

/// Mount first NTFS partition found
pub fn mount_first() -> bool {
    unsafe {
        MOUNTED = false;
        NENT = 0;
    }
    let np = part::count();
    let mut i = 0usize;
    while i < np {
        if let Some(p) = part::get(i) {
            if p.ptype == 0x07 {
                if try_mount(p.disk, p.lba_start) {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

fn try_mount(disk: u8, part_lba: u32) -> bool {
    let mut boot = [0u8; 512];
    if !block::read(disk, part_lba, 1, &mut boot) {
        return false;
    }
    // OEM "NTFS    "
    if boot[3]!=b'N' || boot[4]!=b'T' || boot[5]!=b'F' || boot[6]!=b'S' {
        return false;
    }
    let bps = u16::from_le_bytes([boot[11], boot[12]]);
    if bps != 512 {
        return false;
    }
    let spc = boot[13];
    if spc == 0 {
        return false;
    }
    let mft_lcn = u64::from_le_bytes([
        boot[48], boot[49], boot[50], boot[51], boot[52], boot[53], boot[54], boot[55],
    ]);
    let clusters_per_mft = boot[64] as i8;
    let rec_size = if clusters_per_mft > 0 {
        (clusters_per_mft as u32) * (spc as u32) * 512
    } else {
        1u32 << ((-clusters_per_mft) as u32)
    };
    if rec_size < 512 || rec_size > 4096 {
        return false;
    }
    unsafe {
        DISK = disk;
        PART_LBA = part_lba;
        BPS = bps;
        SPC = spc;
        MFT_LCN = mft_lcn;
        MFT_REC_SIZE = rec_size;
        MFT_RUN_COUNT = 0;
        MOUNTED = true;
    }
    if !init_mft_runs() {
        unsafe { MOUNTED = false; }
        serial::write_str("[NTFS] MFT runlist init fail\\n");
        return false;
    }
    serial::write_str("[NTFS] mounted LBA=");
    serial::write_usize(part_lba as usize);
    serial::write_str(" MFT LCN=");
    serial::write_usize(mft_lcn as usize);
    serial::write_str("\n");
    list_root()
}

pub fn is_mounted() -> bool {
    unsafe { MOUNTED }
}

fn read_mft_record_contiguous(ref_num: u32, out: &mut [u8]) -> bool {
    let rec_size = unsafe { MFT_REC_SIZE as usize };
    if out.len() < rec_size {
        return false;
    }
    // Simplified: assume $MFT is contiguous from MFT_LCN (often true for small systems)
    let bytes_per_cluster = unsafe { SPC as u32 * 512 };
    let offset = (ref_num as u64) * (rec_size as u64);
    let lcn = unsafe { MFT_LCN } + offset / bytes_per_cluster as u64;
    let mut skip = (offset % bytes_per_cluster as u64) as usize;
    let mut got = 0usize;
    let mut cur = lcn;
    while got < rec_size {
        let lba = cluster_to_lba(cur);
        let mut sec = [0u8; 512];
        let disk = unsafe { DISK };
        // read full cluster
        let spc = unsafe { SPC as u32 };
        let mut s = 0u32;
        while s < spc && got < rec_size {
            if !read_lba(disk, lba + s, &mut sec) {
                return false;
            }
            let mut i = if s == 0 { skip } else { 0 };
            if s > 0 {
                skip = 0;
            }
            while i < 512 && got < rec_size {
                out[got] = sec[i];
                got += 1;
                i += 1;
            }
            s += 1;
        }
        cur += 1;
        skip = 0;
    }
    // Fixup
    if out[0]!=b'F' || out[1]!=b'I' || out[2]!=b'L' || out[3]!=b'E' {
        return false;
    }
    let usa_off = u16::from_le_bytes([out[4], out[5]]) as usize;
    let usa_count = u16::from_le_bytes([out[6], out[7]]) as usize;
    if usa_off + usa_count * 2 > rec_size {
        return false;
    }
    let mut i = 1usize;
    while i < usa_count {
        let sector_end = i * 512 - 2;
        if sector_end + 1 < rec_size {
            out[sector_end] = out[usa_off + i * 2];
            out[sector_end + 1] = out[usa_off + i * 2 + 1];
        }
        i += 1;
    }
    true
}

fn init_mft_runs() -> bool {
    unsafe {
        MFT_RUN_COUNT = 0;
    }
    // Record 0 is the $MFT file itself. Its first record is at MFT_LCN and is
    // normally contiguous, so use the old/simple reader only to obtain record 0.
    let rec_size = unsafe { MFT_REC_SIZE as usize };
    if rec_size > 1024 {
        return false;
    }
    let mut rec = [0u8; 1024];
    if !read_mft_record_contiguous(0, &mut rec[..rec_size]) {
        serial::write_str("[NTFS] MFT record 0 read fail\\n");
        return false;
    }

    let mut attr_off = u16::from_le_bytes([rec[20], rec[21]]) as usize;
    while attr_off + 8 <= rec_size {
        let atype = u32::from_le_bytes([
            rec[attr_off], rec[attr_off + 1], rec[attr_off + 2], rec[attr_off + 3],
        ]);
        if atype == 0xFFFF_FFFF {
            break;
        }
        let alen = u32::from_le_bytes([
            rec[attr_off + 4], rec[attr_off + 5], rec[attr_off + 6], rec[attr_off + 7],
        ]) as usize;
        if alen < 16 || attr_off + alen > rec_size {
            break;
        }

        if atype == 0x80 && rec[attr_off + 8] != 0 {
            // $DATA non-resident. Runlist offset is at +0x20.
            let run_off = u16::from_le_bytes([
                rec[attr_off + 32], rec[attr_off + 33],
            ]) as usize;
            if run_off < alen {
                let mut p = attr_off + run_off;
                let end = attr_off + alen;
                let mut current_lcn: i64 = 0;
                while p < end && unsafe { MFT_RUN_COUNT } < 32 {
                    let head = rec[p];
                    p += 1;
                    if head == 0 {
                        break;
                    }
                    let len_bytes = (head & 0x0F) as usize;
                    let off_bytes = ((head >> 4) & 0x0F) as usize;
                    if len_bytes == 0 || len_bytes > 8 || off_bytes > 8 || p + len_bytes + off_bytes > end {
                        break;
                    }

                    let mut run_len = 0u64;
                    let mut i = 0usize;
                    while i < len_bytes {
                        run_len |= (rec[p + i] as u64) << (i * 8);
                        i += 1;
                    }
                    p += len_bytes;

                    let mut delta = 0i64;
                    if off_bytes > 0 {
                        let mut raw = 0u64;
                        i = 0;
                        while i < off_bytes {
                            raw |= (rec[p + i] as u64) << (i * 8);
                            i += 1;
                        }
                        // Sign-extend the little-endian signed LCN delta.
                        if (rec[p + off_bytes - 1] & 0x80) != 0 && off_bytes < 8 {
                            raw |= (!0u64) << (off_bytes * 8);
                        }
                        delta = raw as i64;
                    }
                    p += off_bytes;

                    if run_len == 0 {
                        break;
                    }
                    current_lcn = current_lcn.wrapping_add(delta);
                    unsafe {
                        MFT_RUNS[MFT_RUN_COUNT] = (current_lcn as u64, run_len);
                        MFT_RUN_COUNT += 1;
                    }
                }
            }
            break;
        }
        attr_off += alen;
    }

    unsafe {
        if MFT_RUN_COUNT == 0 {
            // Keep compatibility with simple/contiguous volumes.
            MFT_RUNS[0] = (MFT_LCN, u64::MAX);
            MFT_RUN_COUNT = 1;
        }
    }
    true
}

fn mft_cluster_lcn(cluster_index: u64) -> Option<u64> {
    let mut base = 0u64;
    unsafe {
        let mut i = 0usize;
        while i < MFT_RUN_COUNT {
            let (lcn, count) = MFT_RUNS[i];
            if cluster_index < base + count {
                return Some(lcn + (cluster_index - base));
            }
            base = base.wrapping_add(count);
            i += 1;
        }
    }
    None
}

fn read_mft_record(ref_num: u32, out: &mut [u8]) -> bool {
    let rec_size = unsafe { MFT_REC_SIZE as usize };
    if out.len() < rec_size {
        return false;
    }
    let bytes_per_cluster = unsafe { SPC as u64 * 512 };
    let offset = (ref_num as u64) * (rec_size as u64);
    let mut got = 0usize;

    while got < rec_size {
        let cur_offset = offset + got as u64;
        let cluster_index = cur_offset / bytes_per_cluster;
        let intra = (cur_offset % bytes_per_cluster) as usize;
        let lcn = match mft_cluster_lcn(cluster_index) {
            Some(v) => v,
            None => return false,
        };
        let mut s = (intra / 512) as u32;
        let mut sec_off = intra % 512;
        let spc = unsafe { SPC as u32 };
        while s < spc && got < rec_size {
            let mut sec = [0u8; 512];
            let disk = unsafe { DISK };
            if !read_lba(disk, cluster_to_lba(lcn) + s, &mut sec) {
                return false;
            }
            let mut i = sec_off;
            sec_off = 0;
            while i < 512 && got < rec_size {
                out[got] = sec[i];
                got += 1;
                i += 1;
            }
            s += 1;
        }
    }

    // FILE record signature.
    if out[0] != b'F' || out[1] != b'I' || out[2] != b'L' || out[3] != b'E' {
        return false;
    }
    let usa_off = u16::from_le_bytes([out[4], out[5]]) as usize;
    let usa_count = u16::from_le_bytes([out[6], out[7]]) as usize;
    if usa_off + usa_count * 2 > rec_size {
        return false;
    }
    let mut i = 1usize;
    while i < usa_count {
        let sector_end = i * 512 - 2;
        if sector_end + 1 < rec_size {
            out[sector_end] = out[usa_off + i * 2];
            out[sector_end + 1] = out[usa_off + i * 2 + 1];
        }
        i += 1;
    }
    true
}

fn parse_runlist(rec: &[u8], attr_off: usize, alen: usize, runs: &mut [(u64, u64); 32], count: &mut usize) {
    *count = 0;
    if attr_off + 8 > rec.len() || attr_off + alen > rec.len() || alen < 64 || rec[attr_off + 8] == 0 {
        return;
    }
    let run_off = u16::from_le_bytes([rec[attr_off + 32], rec[attr_off + 33]]) as usize;
    if run_off >= alen {
        return;
    }
    let mut p = attr_off + run_off;
    let end = attr_off + alen;
    let mut current_lcn: i64 = 0;
    while p < end && *count < 32 {
        let head = rec[p];
        p += 1;
        if head == 0 {
            break;
        }
        let len_bytes = (head & 0x0F) as usize;
        let off_bytes = ((head >> 4) & 0x0F) as usize;
        if len_bytes == 0 || len_bytes > 8 || off_bytes > 8 || p + len_bytes + off_bytes > end {
            break;
        }
        let mut run_len = 0u64;
        let mut i = 0usize;
        while i < len_bytes {
            run_len |= (rec[p + i] as u64) << (i * 8);
            i += 1;
        }
        p += len_bytes;
        let mut delta = 0i64;
        if off_bytes > 0 {
            let mut raw = 0u64;
            i = 0;
            while i < off_bytes {
                raw |= (rec[p + i] as u64) << (i * 8);
                i += 1;
            }
            if (rec[p + off_bytes - 1] & 0x80) != 0 && off_bytes < 8 {
                raw |= (!0u64) << (off_bytes * 8);
            }
            delta = raw as i64;
        }
        p += off_bytes;
        if run_len == 0 {
            break;
        }
        current_lcn = current_lcn.wrapping_add(delta);
        runs[*count] = (current_lcn as u64, run_len);
        *count += 1;
    }
}

fn index_cluster_lcn(cluster_index: u64) -> Option<u64> {
    let mut base = 0u64;
    unsafe {
        let mut i = 0usize;
        while i < INDEX_RUN_COUNT {
            let (lcn, count) = INDEX_RUNS[i];
            if cluster_index < base + count {
                return Some(lcn + (cluster_index - base));
            }
            base = base.wrapping_add(count);
            i += 1;
        }
    }
    None
}

fn read_index_stream(offset: u64, out: &mut [u8]) -> bool {
    let bytes_per_cluster = unsafe { SPC as u64 * 512 };
    let mut got = 0usize;
    while got < out.len() {
        let cur = offset + got as u64;
        let cluster_index = cur / bytes_per_cluster;
        let intra = (cur % bytes_per_cluster) as usize;
        let lcn = match index_cluster_lcn(cluster_index) {
            Some(v) => v,
            None => return false,
        };
        let mut s = (intra / 512) as u32;
        let mut sec_off = intra % 512;
        let spc = unsafe { SPC as u32 };
        while s < spc && got < out.len() {
            let mut sec = [0u8; 512];
            let disk = unsafe { DISK };
            if !read_lba(disk, cluster_to_lba(lcn) + s, &mut sec) {
                return false;
            }
            let mut i = sec_off;
            sec_off = 0;
            while i < 512 && got < out.len() {
                out[got] = sec[i];
                got += 1;
                i += 1;
            }
            s += 1;
        }
    }
    true
}

fn apply_index_fixup(buf: &mut [u8], size: usize) -> bool {
    if size < 8 || &buf[0..4] != b"INDX" {
        return false;
    }
    let usa_off = u16::from_le_bytes([buf[4], buf[5]]) as usize;
    let usa_count = u16::from_le_bytes([buf[6], buf[7]]) as usize;
    if usa_count < 1 || usa_off + usa_count * 2 > size {
        return false;
    }
    let mut i = 1usize;
    while i < usa_count {
        let sector_end = i * 512 - 2;
        if sector_end + 1 >= size {
            return false;
        }
        buf[sector_end] = buf[usa_off + i * 2];
        buf[sector_end + 1] = buf[usa_off + i * 2 + 1];
        i += 1;
    }
    true
}

fn parse_index_buffer(buf: &[u8], size: usize) -> usize {
    if size < 0x38 || &buf[0..4] != b"INDX" {
        return 0;
    }
    let hdr = 0x18usize;
    let entries_off = u32::from_le_bytes([buf[hdr], buf[hdr+1], buf[hdr+2], buf[hdr+3]]) as usize;
    let total_size = u32::from_le_bytes([buf[hdr+4], buf[hdr+5], buf[hdr+6], buf[hdr+7]]) as usize;
    let start = hdr + entries_off;
    let end = core::cmp::min(hdr + total_size, size);
    if start >= end {
        return 0;
    }
    let before = unsafe { NENT };
    parse_index_entries(buf, start, end);
    unsafe { NENT.saturating_sub(before) }
}

pub fn list_directory(mft_ref: u32) -> bool {
    unsafe {
        NENT = 0;
        INDEX_RUN_COUNT = 0;
        INDEX_REAL_SIZE = 0;
        INDEX_BLOCK_SIZE = 4096;
        INDEX_BUFFERS_READ = 0;
    }
    let mut rec = [0u8; 1024];
    let rec_size = unsafe { MFT_REC_SIZE as usize };
    if rec_size > 1024 {
        return false;
    }
    if !read_mft_record(mft_ref, &mut rec[..rec_size]) {
        serial::write_str("[NTFS] directory MFT read fail\n");
        return false;
    }

    let mut attr_off = u16::from_le_bytes([rec[20], rec[21]]) as usize;
    let mut index_alloc_attr = 0usize;
    let mut index_alloc_len = 0usize;
    while attr_off + 8 <= rec_size {
        let atype = u32::from_le_bytes([
            rec[attr_off], rec[attr_off + 1], rec[attr_off + 2], rec[attr_off + 3],
        ]);
        if atype == 0xFFFF_FFFF {
            break;
        }
        let alen = u32::from_le_bytes([
            rec[attr_off + 4], rec[attr_off + 5], rec[attr_off + 6], rec[attr_off + 7],
        ]) as usize;
        if alen < 16 || attr_off + alen > rec_size {
            break;
        }
        let nonres = rec[attr_off + 8];
        if atype == 0x90 && nonres == 0 {
            let val_off = u16::from_le_bytes([rec[attr_off + 20], rec[attr_off + 21]]) as usize;
            let val_len = u32::from_le_bytes([
                rec[attr_off + 16], rec[attr_off + 17], rec[attr_off + 18], rec[attr_off + 19],
            ]) as usize;
            let base = attr_off + val_off;
            if val_len >= 16 && base + 16 <= rec_size {
                // INDEX_ROOT: +8 = index block size.
                let ibs = u32::from_le_bytes([rec[base+8], rec[base+9], rec[base+10], rec[base+11]]);
                unsafe {
                    if ibs >= 512 && ibs <= 8192 {
                        INDEX_BLOCK_SIZE = ibs;
                    }
                }
                if val_len > 32 {
                    parse_index_entries(&rec, base + 32, base + val_len);
                }
            }
        } else if atype == 0xA0 && nonres != 0 {
            index_alloc_attr = attr_off;
            index_alloc_len = alen;
            let real_size = u64::from_le_bytes([
                rec[attr_off+48], rec[attr_off+49], rec[attr_off+50], rec[attr_off+51],
                rec[attr_off+52], rec[attr_off+53], rec[attr_off+54], rec[attr_off+55],
            ]);
            unsafe { INDEX_REAL_SIZE = real_size; }
        }
        attr_off += alen;
    }

    if index_alloc_attr != 0 {
        unsafe {
            parse_runlist(&rec, index_alloc_attr, index_alloc_len, &mut INDEX_RUNS, &mut INDEX_RUN_COUNT);
        }
        let block_size = unsafe { INDEX_BLOCK_SIZE as usize };
        let real_size = unsafe { INDEX_REAL_SIZE as usize };
        if block_size >= 512 && block_size <= 8192 && real_size >= block_size {
            let mut off = 0usize;
            let mut buf = [0u8; 8192];
            while off + block_size <= real_size && unsafe { INDEX_BUFFERS_READ } < 256 {
                if !read_index_stream(off as u64, &mut buf[..block_size]) {
                    break;
                }
                if apply_index_fixup(&mut buf[..block_size], block_size) {
                    unsafe { INDEX_BUFFERS_READ += 1; }
                    let _ = parse_index_buffer(&buf[..block_size], block_size);
                }
                off += block_size;
            }
        }
    }

    serial::write_str("[NTFS] directory entries=");
    serial::write_usize(unsafe { NENT });
    serial::write_str("\n");
    true
}

fn list_root() -> bool {
    list_directory(5)
}

fn parse_index_entries(rec: &[u8], mut off: usize, end: usize) {
    while off + 16 <= end && unsafe { NENT } < 32 {
        let mft_lo = u32::from_le_bytes([rec[off], rec[off + 1], rec[off + 2], rec[off + 3]]);
        let entry_size = u16::from_le_bytes([rec[off + 8], rec[off + 9]]) as usize;
        let flags = u16::from_le_bytes([rec[off + 12], rec[off + 13]]);
        if entry_size < 16 {
            break;
        }
        if (flags & 2) != 0 {
            // last entry
            break;
        }
        // filename in entry: at off+16 is FILE_NAME attr style for index
        // Index entry layout: MFT ref 8, size 2, flags 2, then filename structure
        let name_len = rec[off + 0x50] as usize; // approximate for standard $I30 — varies
        // Standard INDEX entry for $I30:
        // +0x00 MFT 8
        // +0x08 entry size 2
        // +0x0A stream size 2
        // +0x0C flags 2
        // +0x10 starts FILE_NAME attribute body: parent 8, times 32, sizes 16, flags 4, name_len 1, ns 1, name UTF16
        let fn_off = off + 0x10;
        if fn_off + 0x42 <= off + entry_size && off + entry_size <= end {
            let nlen = rec[fn_off + 0x40] as usize;
            let flags_fn = u32::from_le_bytes([
                rec[fn_off + 0x38], rec[fn_off + 0x39], rec[fn_off + 0x3A], rec[fn_off + 0x3B],
            ]);
            let real_size = u64::from_le_bytes([
                rec[fn_off + 0x30], rec[fn_off + 0x31], rec[fn_off + 0x32], rec[fn_off + 0x33],
                rec[fn_off + 0x34], rec[fn_off + 0x35], rec[fn_off + 0x36], rec[fn_off + 0x37],
            ]);
            let name_bytes = fn_off + 0x42;
            let mut name = [0u8; 48];
            let mut nl = 0usize;
            let mut c = 0usize;
            while c < nlen && nl < 47 && name_bytes + c * 2 + 1 < off + entry_size {
                let lo = rec[name_bytes + c * 2];
                let hi = rec[name_bytes + c * 2 + 1];
                if hi == 0 && lo >= 32 && lo < 127 {
                    name[nl] = lo;
                    nl += 1;
                } else if hi == 0 && lo == 0 {
                    break;
                } else {
                    name[nl] = b'?';
                    nl += 1;
                }
                c += 1;
            }
            // skip . and dos short names namespace 2
            let ns = rec[fn_off + 0x41];
            if nl > 0 && ns != 2 {
                let is_dir = (flags_fn & 0x1000_0000) != 0;
                unsafe {
                    ENTRIES[NENT] = NtfsEntry {
                        name,
                        name_len: nl,
                        size: real_size,
                        is_dir,
                        mft_ref: mft_lo,
                    };
                    NENT += 1;
                }
            }
        }
        off += entry_size;
    }
}

pub fn entry_count() -> usize {
    unsafe { NENT }
}

pub fn entry(i: usize) -> Option<NtfsEntry> {
    unsafe {
        if i < NENT {
            Some(ENTRIES[i])
        } else {
            None
        }
    }
}


/// NTFS diagnostic for the GUI terminal (read-only).
/// This intentionally exercises the filesystem parser path, not the DSK RAW/MBR probe.
fn diag_str(s: &str) {
    crate::desktop::terminal_write(s);
    serial::write_str(s);
}

fn diag_usize(mut v: usize) {
    let mut buf = [0u8; 20];
    let mut n = 0usize;
    if v == 0 {
        diag_str("0");
        return;
    }
    while v > 0 {
        buf[n] = b'0' + (v % 10) as u8;
        n += 1;
        v /= 10;
    }
    let mut out = [0u8; 20];
    let mut i = 0usize;
    while i < n {
        out[i] = buf[n - 1 - i];
        i += 1;
    }
    let s = unsafe { core::str::from_utf8_unchecked(&out[..n]) };
    diag_str(s);
}

pub fn diagnostic() {
    diag_str("[NTFSDIAG] begin\n");
    let np = part::count();
    diag_str("[NTFSDIAG] partitions=");
    diag_usize(np);
    diag_str("\n");

    let mut i = 0usize;
    let mut found = 0usize;
    while i < np {
        if let Some(p) = part::get(i) {
            if p.ptype == 0x07 {
                found += 1;
                diag_str("[NTFSDIAG] NTFS part index=");
                diag_usize(i);
                diag_str(" disk=");
                diag_usize(p.disk as usize);
                diag_str(" start_lba=");
                diag_usize(p.lba_start as usize);
                diag_str(" sectors=");
                diag_usize(p.sectors as usize);
                diag_str("\n");

                let mut boot = [0u8; 512];
                if !block::read(p.disk, p.lba_start, 1, &mut boot) {
                    diag_str("[NTFSDIAG] BOOT_READ=FAIL\n");
                } else {
                    let ntfs = boot[3]==b'N' && boot[4]==b'T' && boot[5]==b'F' && boot[6]==b'S';
                    diag_str("[NTFSDIAG] BOOT_NTFS=");
                    diag_str(if ntfs { "OK" } else { "FAIL" });
                    diag_str("\n");
                    if ntfs {
                        let bps = u16::from_le_bytes([boot[11],boot[12]]);
                        let spc = boot[13];
                        let mft = u64::from_le_bytes([
                            boot[48],boot[49],boot[50],boot[51],boot[52],boot[53],boot[54],boot[55]
                        ]);
                        let cpm = boot[64] as i8;
                        let rec = if cpm > 0 {
                            (cpm as u32) * (spc as u32) * 512
                        } else if cpm < 0 {
                            1u32 << ((-cpm) as u32)
                        } else { 0 };
                        diag_str("[NTFSDIAG] BPS=");
                        diag_usize(bps as usize);
                        diag_str(" SPC=");
                        diag_usize(spc as usize);
                        diag_str(" MFT_LCN=");
                        diag_usize(mft as usize);
                        diag_str(" REC_SIZE=");
                        diag_usize(rec as usize);
                        diag_str("\n");

                        if bps != 512 || spc == 0 || rec < 512 || rec > 4096 {
                            diag_str("[NTFSDIAG] GEOMETRY=INVALID\n");
                        } else {
                            unsafe {
                                DISK=p.disk; PART_LBA=p.lba_start; BPS=bps; SPC=spc;
                                MFT_LCN=mft; MFT_REC_SIZE=rec; MFT_RUN_COUNT=0; MOUNTED=true;
                            }
                            let mut r0=[0u8;1024];
                            if read_mft_record_contiguous(0,&mut r0[..rec as usize]) {
                                diag_str("[NTFSDIAG] MFT0=OK\n");
                                if init_mft_runs() {
                                    diag_str("[NTFSDIAG] MFT_RUNS=");
                                    unsafe { diag_usize(MFT_RUN_COUNT); }
                                    diag_str("\n");
                                } else {
                                    diag_str("[NTFSDIAG] MFT_RUNS=FAIL\n");
                                }
                                let mut r5=[0u8;1024];
                                if read_mft_record(5,&mut r5[..rec as usize]) {
                                    diag_str("[NTFSDIAG] MFT5=OK\n");
                                    let _ = list_directory(5);
                                    diag_str("[NTFSDIAG] DIRECTORY_ENTRIES=");
                                    diag_usize(entry_count());
                                    diag_str("\n");
                                    // Inspect MFT#5 attributes to determine whether the directory
                                    // needs non-resident $INDEX_ALLOCATION traversal.
                                    let mut aoff = u16::from_le_bytes([r5[20], r5[21]]) as usize;
                                    let mut index_root_seen = false;
                                    let mut index_alloc_seen = false;
                                    while aoff + 8 <= rec as usize {
                                        let atype = u32::from_le_bytes([
                                            r5[aoff], r5[aoff+1], r5[aoff+2], r5[aoff+3]
                                        ]);
                                        if atype == 0xFFFF_FFFF { break; }
                                        let alen = u32::from_le_bytes([
                                            r5[aoff+4], r5[aoff+5], r5[aoff+6], r5[aoff+7]
                                        ]) as usize;
                                        if alen < 16 || aoff + alen > rec as usize { break; }
                                        let nonres = r5[aoff+8];
                                        if atype == 0x90 {
                                            index_root_seen = true;
                                            let vlen = u32::from_le_bytes([
                                                r5[aoff+16], r5[aoff+17], r5[aoff+18], r5[aoff+19]
                                            ]);
                                            diag_str("[NTFSDIAG] INDEX_ROOT=");
                                            diag_str(if nonres == 0 { "RESIDENT" } else { "NONRESIDENT" });
                                            diag_str(" VALUE_SIZE=");
                                            diag_usize(vlen as usize);
                                            diag_str("\n");
                                        } else if atype == 0xA0 {
                                            index_alloc_seen = true;
                                            diag_str("[NTFSDIAG] INDEX_ALLOCATION=NONRESIDENT");
                                            if alen >= 56 {
                                                let run_off = u16::from_le_bytes([
                                                    r5[aoff+32], r5[aoff+33]
                                                ]) as usize;
                                                let alloc_size = u64::from_le_bytes([
                                                    r5[aoff+40], r5[aoff+41], r5[aoff+42], r5[aoff+43],
                                                    r5[aoff+44], r5[aoff+45], r5[aoff+46], r5[aoff+47]
                                                ]);
                                                let real_size = u64::from_le_bytes([
                                                    r5[aoff+48], r5[aoff+49], r5[aoff+50], r5[aoff+51],
                                                    r5[aoff+52], r5[aoff+53], r5[aoff+54], r5[aoff+55]
                                                ]);
                                                diag_str(" RUN_OFF=");
                                                diag_usize(run_off);
                                                diag_str(" ALLOC_SIZE=");
                                                diag_usize(alloc_size as usize);
                                                diag_str(" REAL_SIZE=");
                                                diag_usize(real_size as usize);
                                            }
                                            diag_str("\n");
                                        }
                                        aoff += alen;
                                    }
                                    diag_str("[NTFSDIAG] INDEX_ALLOCATION_RUNS="); unsafe { diag_usize(INDEX_RUN_COUNT); } diag_str(" INDEX_BUFFERS_READ="); unsafe { diag_usize(INDEX_BUFFERS_READ); } diag_str("\n");
                                    diag_str("[NTFSDIAG] INDEX_ROOT_PRESENT=");
                                    diag_str(if index_root_seen { "YES" } else { "NO" });
                                    diag_str(" INDEX_ALLOCATION_PRESENT=");
                                    diag_str(if index_alloc_seen { "YES" } else { "NO" });
                                    diag_str("\n");
                                } else {
                                    diag_str("[NTFSDIAG] MFT5=FAIL\n");
                                }
                            } else {
                                diag_str("[NTFSDIAG] MFT0=FAIL\n");
                            }
                            unsafe { MOUNTED=false; }
                        }
                    }
                }
            }
        }
        i += 1;
    }
    diag_str("[NTFSDIAG] NTFS_PARTITIONS=");
    diag_usize(found);
    diag_str("\n[NTFSDIAG] end\n");
}

pub fn remount_list() -> bool {
    if !is_mounted() {
        return mount_first();
    }
    list_root()
}
