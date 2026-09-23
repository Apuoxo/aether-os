//! GDT + TSS for Ring 0 / Ring 3

#[repr(C, packed)]
pub struct GdtEntry {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_mid: u8,
    pub access: u8,
    pub granularity: u8,
    pub base_high: u8,
}

#[repr(C, packed)]
pub struct Tss {
    pub reserved0: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    pub reserved1: u64,
    pub ist: [u64; 7],
    pub reserved2: u64,
    pub reserved3: u16,
    pub iomap_base: u16,
}

#[repr(C, packed)]
pub struct GdtPointer {
    pub limit: u16,
    pub base: u64,
}

static mut GDT: [GdtEntry; 7] = [
    // 0: null
    GdtEntry { limit_low:0, base_low:0, base_mid:0, access:0, granularity:0, base_high:0 },
    // 1: kernel code 0x08
    GdtEntry { limit_low:0xFFFF, base_low:0, base_mid:0, access:0x9A, granularity:0xAF, base_high:0 },
    // 2: kernel data 0x10
    GdtEntry { limit_low:0xFFFF, base_low:0, base_mid:0, access:0x92, granularity:0xCF, base_high:0 },
    // 3: user code 0x18 (DPL=3) 64-bit
    GdtEntry { limit_low:0xFFFF, base_low:0, base_mid:0, access:0xFA, granularity:0xAF, base_high:0 },
    // 4: user data 0x20 (DPL=3)
    GdtEntry { limit_low:0xFFFF, base_low:0, base_mid:0, access:0xF2, granularity:0xCF, base_high:0 },
    // 5-6: TSS (will be filled)
    GdtEntry { limit_low:0, base_low:0, base_mid:0, access:0, granularity:0, base_high:0 },
    GdtEntry { limit_low:0, base_low:0, base_mid:0, access:0, granularity:0, base_high:0 },
];

static mut TSS: Tss = Tss {
    reserved0: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    reserved1: 0,
    ist: [0; 7],
    reserved2: 0,
    reserved3: 0,
    iomap_base: 0,
};

static mut GDTR: GdtPointer = GdtPointer { limit: 0, base: 0 };

pub fn init(kernel_stack_top: u64) {
    unsafe {
        TSS.rsp0 = kernel_stack_top;

        let tss_ptr = &TSS as *const Tss as u64;
        let tss_limit = (core::mem::size_of::<Tss>() - 1) as u16;

        // TSS descriptor (system segment, type 0x9 available TSS)
        GDT[5].limit_low = tss_limit;
        GDT[5].base_low = (tss_ptr & 0xFFFF) as u16;
        GDT[5].base_mid = ((tss_ptr >> 16) & 0xFF) as u8;
        GDT[5].access = 0x89; // present, type=9 (available 64-bit TSS)
        GDT[5].granularity = ((tss_ptr >> 24) & 0x0F) as u8;
        GDT[5].base_high = ((tss_ptr >> 32) & 0xFF) as u8;
        // high dword of base for 64-bit
        GDT[6].limit_low = ((tss_ptr >> 40) & 0xFFFF) as u16;
        GDT[6].base_low = ((tss_ptr >> 48) & 0xFFFF) as u16;

        GDTR.limit = (core::mem::size_of_val(&GDT) - 1) as u16;
        GDTR.base = &GDT as *const _ as u64;

        // Load GDT
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &GDTR,
            options(readonly, nostack, preserves_flags)
        );

        // Load TSS
        core::arch::asm!(
            "mov ax, 0x28", // TSS selector (5 * 8)
            "ltr ax",
            options(nostack, preserves_flags)
        );

        // Reload data segments
        core::arch::asm!(
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            options(nostack, preserves_flags)
        );
    }
}

pub const KERNEL_CS: u16 = 0x08;
pub const KERNEL_DS: u16 = 0x10;
pub const USER_CS: u16 = 0x18 | 3; // RPL=3
pub const USER_DS: u16 = 0x20 | 3;
