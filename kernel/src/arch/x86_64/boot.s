; Aether bootstrap — Multiboot2 + optional framebuffer → Long Mode → kernel_main

MULTIBOOT2_MAGIC equ 0xE85250D6
MULTIBOOT2_ARCH  equ 0

section .multiboot
align 8
multiboot_header:
    dd MULTIBOOT2_MAGIC
    dd MULTIBOOT2_ARCH
    dd multiboot_header_end - multiboot_header
    dd -(MULTIBOOT2_MAGIC + MULTIBOOT2_ARCH + (multiboot_header_end - multiboot_header))
    ; Framebuffer request tag (type 5) — GRUB may provide linear FB
    align 8
    dw 5
    dw 0
    dd 20
    dd 800          ; width preference
    dd 600          ; height preference
    dd 32           ; depth preference
    ; End tag
    align 8
    dw 0
    dw 0
    dd 8
multiboot_header_end:

section .bss
align 16
stack_bottom:
    resb 65536
stack_top:

; Saved Multiboot2 info pointer (physical)
align 8
mbi_ptr:
    resq 1

section .data
align 4096
p4_table:
    dq p3_table + 0x03
    times 511 dq 0

align 4096
p3_table:
    dq p2_table + 0x03
    times 511 dq 0

align 4096
p2_table:
%assign i 0
%rep 512
    dq (i << 21) + 0x83
%assign i i+1
%endrep

align 16
gdt64:
    dq 0
    dq 0x00AF9A000000FFFF
    dq 0x00AF92000000FFFF
gdt64_end:

align 4
gdt64_pointer:
    dw gdt64_end - gdt64 - 1
    dd gdt64

section .text
bits 32
global _start

_start:
    cli
    cld
    ; Preserve Multiboot2 info (ebx) and magic (eax)
    mov [mbi_ptr], ebx

    mov esp, stack_top
    and esp, 0xFFFFFFF0

    ; VGA: "A0"
    mov dword [0xB8000], 0x0F300F41

    ; CPUID long mode
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_long_mode
    mov eax, 0x80000001
    cpuid
    test edx, (1 << 29)
    jz .no_long_mode

    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    mov eax, p4_table
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    mov eax, cr0
    or eax, (1 << 31) | 1
    mov cr0, eax

    lgdt [gdt64_pointer]
    jmp 0x08:long_mode_entry

.no_long_mode:
    mov word [0xB8000], 0x4F45
.halt32:
    hlt
    jmp .halt32

bits 64
long_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    mov rsp, stack_top
    and rsp, -16

    ; FPU/SSE
    mov rax, cr0
    and rax, ~(1 << 2)
    or rax, (1 << 1) | (1 << 5)
    mov cr0, rax
    mov rax, cr4
    or rax, (1 << 9) | (1 << 10)
    mov cr4, rax
    fninit

    ; VGA OK
    mov rdi, 0xB8000
    mov rax, 0x0F4B0F4F
    mov [rdi], eax

    ; kernel_main(mbi_ptr)
    mov rdi, [mbi_ptr]
    extern kernel_main
    call kernel_main

.hang:
    hlt
    jmp .hang
