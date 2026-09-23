; /bin/init — permanent userspace path, Aether ABI only
bits 64
global _start
_start:
    ; write banner
    mov rax, 1
    mov rdi, 1
    mov rsi, msg_init
    mov rdx, msg_init_end - msg_init
    int 0x80
    ; read /test.txt
    mov rax, 2
    mov rdi, path_test
    mov rsi, buf
    mov rdx, 64
    int 0x80
    ; write result
    mov rdx, rax
    cmp rdx, 0
    jle .do_exit
    cmp rdx, 64
    jbe .oklen
    mov rdx, 64
.oklen:
    mov rax, 1
    mov rdi, 1
    mov rsi, buf
    int 0x80
    mov rax, 1
    mov rdi, 1
    mov rsi, msg_nl
    mov rdx, 1
    int 0x80
.do_exit:
    mov rax, 60
    xor rdi, rdi
    int 0x80
.hang:
    hlt
    jmp .hang

align 16
msg_init:
    db "[INIT] userspace PID path OK", 10
msg_init_end:
msg_nl:
    db 10
path_test:
    db "/test.txt", 0
align 16
buf:
    times 64 db 0
