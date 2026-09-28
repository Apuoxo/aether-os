//! FAT12/16/32 READ-ONLY browser
use crate::block;
use crate::part;
use crate::serial;

#[derive(Clone, Copy)]
pub struct FatVol {
    pub part_idx: usize,
    pub disk: u8,
    pub part_lba: u32,
    pub fat_type: u8, // 12, 16, 32
    pub bytes_per_sec: u16,
    pub sec_per_cl: u8,
    pub reserved: u16,
    pub fats: u8,
    pub root_ents: u16,
    pub fat_secs: u32,
    pub total_secs: u32,
    pub root_cl: u32, // FAT32
    pub data_lba: u32,
    pub root_lba: u32, // FAT16 root dir
    pub root_secs: u32,
    pub ok: bool,
}

#[derive(Clone, Copy)]
pub struct FatEntry {
    pub name: [u8; 13], // 8.3 + nul
    pub name_len: usize,
    pub size: u32,
    pub cluster: u32,
    pub is_dir: bool,
    pub attr: u8,
}

static mut VOL: FatVol = FatVol {
    part_idx: 0, disk: 0, part_lba: 0, fat_type: 0,
    bytes_per_sec: 512, sec_per_cl: 0, reserved: 0, fats: 0,
    root_ents: 0, fat_secs: 0, total_secs: 0, root_cl: 0,
    data_lba: 0, root_lba: 0, root_secs: 0, ok: false,
};
static mut CWD_CL: u32 = 0; // 0 = root for FAT32
static mut ENTRIES: [FatEntry; 32] = [FatEntry {
    name: [0; 13], name_len: 0, size: 0, cluster: 0, is_dir: false, attr: 0,
}; 32];
static mut NENT: usize = 0;

fn read_sec(disk: u8, abs_lba: u32, buf: &mut [u8; 512]) -> bool {
    block::read(disk, abs_lba, 1, buf)
}

pub fn mount_partition(pi: usize) -> bool {
    let p = match part::get(pi) {
        Some(p) => p,
        None => return false,
    };
    let mut bpb = [0u8; 512];
    if !block::read(p.disk, p.lba_start, 1, &mut bpb) {
        serial::write_str("[FAT] BPB read fail\n");
        return false;
    }
    let bps = u16::from_le_bytes([bpb[11], bpb[12]]);
    if bps != 512 {
        serial::write_str("[FAT] unsupported sector size\n");
        return false;
    }
    let spc = bpb[13];
    let reserved = u16::from_le_bytes([bpb[14], bpb[15]]);
    let fats = bpb[16];
    let root_ents = u16::from_le_bytes([bpb[17], bpb[18]]);
    let mut total16 = u16::from_le_bytes([bpb[19], bpb[20]]) as u32;
    let mut fat_secs = u16::from_le_bytes([bpb[22], bpb[23]]) as u32;
    if fat_secs == 0 {
        fat_secs = u32::from_le_bytes([bpb[36], bpb[37], bpb[38], bpb[39]]);
    }
    let total32 = u32::from_le_bytes([bpb[32], bpb[33], bpb[34], bpb[35]]);
    if total16 == 0 {
        total16 = total32;
    }
    let root_secs = ((root_ents as u32 * 32) + 511) / 512;
    let data_lba = p.lba_start + reserved as u32 + fats as u32 * fat_secs + root_secs;
    let root_lba = p.lba_start + reserved as u32 + fats as u32 * fat_secs;
    let data_secs = total16.saturating_sub(
        reserved as u32 + fats as u32 * fat_secs + root_secs,
    );
    let clusters = if spc == 0 { 0 } else { data_secs / spc as u32 };
    let fat_type = if clusters < 4085 {
        12
    } else if clusters < 65525 {
        16
    } else {
        32
    };
    let root_cl = if fat_type == 32 {
        u32::from_le_bytes([bpb[44], bpb[45], bpb[46], bpb[47]])
    } else {
        0
    };
    // check fat signature-ish
    if bpb[0] != 0xEB && bpb[0] != 0xE9 {
        serial::write_str("[FAT] bad jump\n");
        // still try
    }
    unsafe {
        VOL = FatVol {
            part_idx: pi,
            disk: p.disk,
            part_lba: p.lba_start,
            fat_type,
            bytes_per_sec: bps,
            sec_per_cl: spc,
            reserved,
            fats,
            root_ents,
            fat_secs,
            total_secs: total16,
            root_cl,
            data_lba,
            root_lba,
            root_secs,
            ok: true,
        };
        CWD_CL = if fat_type == 32 { root_cl } else { 0 };
    }
    serial::write_str("[FAT] mounted type=");
    serial::write_usize(fat_type as usize);
    serial::write_str(" spc=");
    serial::write_usize(spc as usize);
    serial::write_str(" clusters~");
    serial::write_usize(clusters as usize);
    serial::write_str("\n");
    list_cwd()
}

pub fn is_mounted() -> bool {
    unsafe { VOL.ok }
}

fn cluster_to_lba(cl: u32) -> u32 {
    unsafe {
        if cl < 2 {
            return VOL.root_lba;
        }
        VOL.data_lba + (cl - 2) * VOL.sec_per_cl as u32
    }
}

fn fat_get(cl: u32) -> u32 {
    unsafe {
        if !VOL.ok {
            return 0xFFFFFFFF;
        }
        let mut sec = [0u8; 512];
        if VOL.fat_type == 32 {
            let fat_lba = VOL.part_lba + VOL.reserved as u32;
            let off = cl * 4;
            let sec_i = off / 512;
            let so = (off % 512) as usize;
            if !read_sec(VOL.disk, fat_lba + sec_i, &mut sec) {
                return 0xFFFFFFFF;
            }
            u32::from_le_bytes([sec[so], sec[so + 1], sec[so + 2], sec[so + 3]]) & 0x0FFF_FFFF
        } else if VOL.fat_type == 16 {
            let fat_lba = VOL.part_lba + VOL.reserved as u32;
            let off = cl * 2;
            let sec_i = off / 512;
            let so = (off % 512) as usize;
            if !read_sec(VOL.disk, fat_lba + sec_i, &mut sec) {
                return 0xFFFFFFFF;
            }
            u16::from_le_bytes([sec[so], sec[so + 1]]) as u32
        } else {
            0xFFFFFFFF
        }
    }
}

fn parse_dir_sector(sec: &[u8; 512], start: usize) {
    let mut i = 0usize;
    while i < 512 {
        let off = i;
        if sec[off] == 0 {
            break;
        }
        if sec[off] == 0xE5 {
            i += 32;
            continue;
        }
        let attr = sec[off + 11];
        if attr == 0x0F {
            // LFN skip
            i += 32;
            continue;
        }
        if attr & 0x08 != 0 {
            // volume label
            i += 32;
            continue;
        }
        unsafe {
            if NENT >= 32 {
                break;
            }
            let mut name = [0u8; 13];
            let mut nl = 0usize;
            let mut j = 0usize;
            while j < 8 && sec[off + j] != b' ' {
                name[nl] = sec[off + j];
                nl += 1;
                j += 1;
            }
            if sec[off + 8] != b' ' {
                name[nl] = b'.';
                nl += 1;
                j = 8;
                while j < 11 && sec[off + j] != b' ' {
                    name[nl] = sec[off + j];
                    nl += 1;
                    j += 1;
                }
            }
            let cl_hi = u16::from_le_bytes([sec[off + 20], sec[off + 21]]) as u32;
            let cl_lo = u16::from_le_bytes([sec[off + 26], sec[off + 27]]) as u32;
            let cluster = (cl_hi << 16) | cl_lo;
            let size = u32::from_le_bytes([sec[off + 28], sec[off + 29], sec[off + 30], sec[off + 31]]);
            ENTRIES[NENT] = FatEntry {
                name,
                name_len: nl,
                size,
                cluster,
                is_dir: attr & 0x10 != 0,
                attr,
            };
            NENT += 1;
        }
        i += 32;
        let _ = start;
    }
}

pub fn list_cwd() -> bool {
    unsafe {
        if !VOL.ok {
            return false;
        }
        NENT = 0;
        let mut sec = [0u8; 512];
        if VOL.fat_type != 32 {
            // FAT16 root is fixed region
            let mut s = 0u32;
            while s < VOL.root_secs && NENT < 32 {
                if !read_sec(VOL.disk, VOL.root_lba + s, &mut sec) {
                    return false;
                }
                parse_dir_sector(&sec, 0);
                s += 1;
            }
        } else {
            let mut cl = if CWD_CL == 0 { VOL.root_cl } else { CWD_CL };
            let mut guard = 0usize;
            while cl >= 2 && cl < 0x0FFF_FFF8 && NENT < 32 && guard < 64 {
                let lba = cluster_to_lba(cl);
                let mut s = 0u8;
                while s < VOL.sec_per_cl && NENT < 32 {
                    if !read_sec(VOL.disk, lba + s as u32, &mut sec) {
                        return false;
                    }
                    parse_dir_sector(&sec, 0);
                    s += 1;
                }
                cl = fat_get(cl);
                guard += 1;
            }
        }
        serial::write_str("[FAT] entries=");
        serial::write_usize(NENT);
        serial::write_str("\n");
        true
    }
}

pub fn entry_count() -> usize {
    unsafe { NENT }
}

pub fn entry(i: usize) -> Option<FatEntry> {
    unsafe {
        if i < NENT {
            Some(ENTRIES[i])
        } else {
            None
        }
    }
}

pub fn enter_dir(i: usize) -> bool {
    unsafe {
        if i >= NENT || !ENTRIES[i].is_dir {
            return false;
        }
        let cl = ENTRIES[i].cluster;
        CWD_CL = if cl == 0 { VOL.root_cl } else { cl };
        list_cwd()
    }
}

pub fn go_root() -> bool {
    unsafe {
        if !VOL.ok {
            return false;
        }
        CWD_CL = if VOL.fat_type == 32 { VOL.root_cl } else { 0 };
        list_cwd()
    }
}

pub fn vol_info() -> Option<FatVol> {
    unsafe {
        if VOL.ok {
            Some(VOL)
        } else {
            None
        }
    }
}
