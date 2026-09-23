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

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
