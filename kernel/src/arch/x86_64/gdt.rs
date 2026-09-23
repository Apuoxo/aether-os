//! GDT + TSS for Ring 0 / Ring 3

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

static mut GDT: [u64; 7] = [
    0x0000000000000000,
    0x00AF9A000000FFFF,
    0x00CF92000000FFFF,
    0x00AFFA000000FFFF,
    0x00CFF2000000FFFF,
    0,
    0,
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
    iomap_base: core::mem::size_of::<Tss>() as u16,
};

static mut GDTR: GdtPointer = GdtPointer { limit: 0, base: 0 };

pub fn init(kernel_stack_top: u64) {
    unsafe {
        TSS.rsp0 = kernel_stack_top;

        let base = &TSS as *const Tss as u64;
        let limit = (core::mem::size_of::<Tss>() - 1) as u64;

        // Intel 64: a TSS descriptor is 16 bytes. GDT[5] is the
        // low 8-byte descriptor; GDT[6] contains Base[63:32].
        let low =
            (limit & 0xFFFF)
            | ((base & 0xFFFFFF) << 16)
            | (0x89u64 << 40)
            | (((limit >> 16) & 0xF) << 48)
            | (((base >> 24) & 0xFF) << 56);
        let high = (base >> 32) & 0xFFFF_FFFF;

        GDT[5] = low;
        GDT[6] = high;

        GDTR.limit = (core::mem::size_of_val(&GDT) - 1) as u16;
        GDTR.base = &GDT as *const _ as u64;

        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &GDTR,
            options(readonly, nostack, preserves_flags)
        );

        core::arch::asm!(
            "mov ax, 0x28",
            "ltr ax",
            options(nostack, preserves_flags)
        );

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
pub const USER_CS: u16 = 0x18 | 3;
pub const USER_DS: u16 = 0x20 | 3;
