/* Aether Kernel Linker Script (x86_64) */

ENTRY(_start)

SECTIONS
{
    . = 1M;                         /* Загрузка начинается с 1 МБ */

    .rodata : {
        *(.multiboot)               /* Multiboot2 заголовок должен быть рано */
        *(.rodata .rodata.*)
    }

    .text : {
        *(.text .text.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /* Export the first byte after all kernel runtime storage so PMM can
       never hand the kernel's own image/stack pages back to the allocator. */
    __kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
