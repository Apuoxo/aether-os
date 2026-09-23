bits 64
global _start
_start:
    ; Try to read kernel-only identity page (2MiB map without USER)
    mov rax, [0x200000]
    ; if we get here, isolation failed
    mov rax, 60
    int 0x80
.hang:
    hlt
    jmp .hang
