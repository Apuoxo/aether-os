bits 64
global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, 0x40000040
    mov rdx, 17
    int 0x80
    mov rax, 2
    xor rdi, rdi
    xor rsi, rsi
    xor rdx, rdx
    int 0x80
    ; Isolation test: touch kernel-only phys map
    mov rax, [0x200000]
    mov rax, 60
    int 0x80
.hang:
    hlt
    jmp .hang
align 64
msg:
    db "Hello from Ring3", 10
