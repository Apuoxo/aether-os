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

pub fn list_directory(mft_ref: u32) -> bool {
    unsafe { NENT = 0; }
    let mut rec = [0u8; 1024];
    let rec_size = unsafe { MFT_REC_SIZE as usize };
    if rec_size > 1024 {
        return false;
    }
    if !read_mft_record(mft_ref, &mut rec[..rec_size]) {
        serial::write_str("[NTFS] directory MFT read fail\n");
        return false;
    }
    // Walk attributes for INDEX_ROOT (0x90) resident
    let mut attr_off = u16::from_le_bytes([rec[20], rec[21]]) as usize;
    while attr_off + 8 < rec_size {
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
            // INDEX_ROOT resident
            let val_off = u16::from_le_bytes([rec[attr_off + 20], rec[attr_off + 21]]) as usize;
            let val_len = u32::from_le_bytes([
                rec[attr_off + 16], rec[attr_off + 17], rec[attr_off + 18], rec[attr_off + 19],
            ]) as usize;
            let base = attr_off + val_off;
            // INDEX_ROOT header 16 bytes + INDEX_HEADER 16
            if val_len > 32 {
                parse_index_entries(&rec, base + 16 + 16, base + val_len);
            }
        }
        // Also INDEX_ALLOCATION non-resident — skip for MVP if root fits in INDEX_ROOT
        attr_off += alen;
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

pub fn remount_list() -> bool {
    if !is_mounted() {
        return mount_first();
    }
    list_root()
}
