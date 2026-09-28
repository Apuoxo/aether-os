; Aether DIAG — minimal Multiboot2 → Long Mode → VGA message → halt
; Stage markers on VGA text (0xB8000):
;   '0' = _start entered
;   '1' = long mode CPUID OK
;   '2' = paging+CR0 set
;   '3' = long mode entry
;   'OK' = full success

MULTIBOOT2_MAGIC equ 0xE85250D6
MULTIBOOT2_ARCH  equ 0

section .multiboot
align 8
mb_header:
    dd MULTIBOOT2_MAGIC
    dd MULTIBOOT2_ARCH
    dd mb_header_end - mb_header
    dd -(MULTIBOOT2_MAGIC + MULTIBOOT2_ARCH + (mb_header_end - mb_header))
    ; End tag
    dw 0
    dw 0
    dd 8
mb_header_end:

section .bss
align 16
stack_bottom:
    resb 16384
stack_top:

section .data
align 4096
p4:
    dq p3 + 0x03
    times 511 dq 0
align 4096
p3:
    dq p2 + 0x03
    times 511 dq 0
align 4096
p2:
%assign i 0
%rep 512
    dq (i << 21) + 0x83   ; 2MB pages, Present|RW|PS
%assign i i+1
%endrep

align 16
gdt:
    dq 0
    dq 0x00AF9A000000FFFF  ; code 64
    dq 0x00AF92000000FFFF  ; data 64
gdt_end:

; 32-bit LGDT format: limit (2) + base (4) = 6 bytes
align 4
gdt_ptr32:
    dw gdt_end - gdt - 1
    dd gdt

section .text
bits 32
global _start
_start:
    cli
    cld
    ; Stage 0
    mov dword [0xB8000], 0x0F300F41   ; 'A''0' white

    mov esp, stack_top
    and esp, 0xFFFFFFF0

    ; Multiboot2 magic check in eax should be 0x36d76289
    cmp eax, 0x36d76289
    jne .bad_magic

    ; Stage 0b magic OK — write 'M'
    mov word [0xB8004], 0x0F4D

    ; CPUID long mode
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .fail
    mov eax, 0x80000001
    cpuid
    test edx, (1 << 29)
    jz .fail

    ; Stage 1
    mov word [0xB8006], 0x0F31        ; '1'

    ; PAE
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    mov eax, p4
    mov cr3, eax

    ; EFER.LME
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    ; PG|PE
    mov eax, cr0
    or eax, (1 << 31) | 1
    mov cr0, eax

    ; Stage 2
    mov word [0xB8008], 0x0F32        ; '2'

    lgdt [gdt_ptr32]
    jmp 0x08:long64

.bad_magic:
    mov dword [0xB8000], 0x4F214F42   ; 'B!' red
.fail:
    mov word [0xB800A], 0x4F46        ; 'F' red
.hang32:
    hlt
    jmp .hang32

bits 64
long64:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov rsp, stack_top
    and rsp, -16

    ; Stage 3
    mov word [0xB800A], 0x0F33        ; '3'

    ; FPU/SSE safe
    mov rax, cr0
    and rax, ~(1 << 2)
    or rax, (1 << 1) | (1 << 5)
    mov cr0, rax
    mov rax, cr4
    or rax, (1 << 9) | (1 << 10)
    mov cr4, rax
    fninit

    ; SUCCESS message on VGA line 1
    mov rdi, 0xB8000
    ; clear first line already has markers; write line 2
    mov rdi, 0xB80A0
    mov rax, 0x0F410F45              ; 'E''A'
    mov [rdi], eax
    mov rax, 0x0F480F54              ; 'T''H'
    mov [rdi+4], eax
    mov rax, 0x0F520F45              ; 'E''R'
    mov [rdi+8], eax
    mov rax, 0x0F200F20
    mov [rdi+12], eax
    mov rax, 0x0F4B0F4F              ; 'O''K'
    mov [rdi+16], eax
    mov rax, 0x0F4E0F20              ; ' ''N'
    mov [rdi+20], eax
    mov rax, 0x0F4C0F45              ; 'E''L'
    mov [rdi+24], eax

    ; also serial COM1 if present
    mov dx, 0x3F8
    mov al, 'A'
    out dx, al
    mov al, 'O'
    out dx, al
    mov al, 'K'
    out dx, al
    mov al, 10
    out dx, al

.halt:
    hlt
    jmp .halt
