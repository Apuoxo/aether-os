; /bin/sh — minimal Aether shell (scripted proof + builtins)
bits 64
global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, msg_sh
    mov rdx, msg_sh_end - msg_sh
    int 0x80

    ; > ls
    mov rax, 1
    mov rdi, 1
    mov rsi, prompt_ls
    mov rdx, prompt_ls_end - prompt_ls
    int 0x80
    mov rax, 3
    mov rdi, listbuf
    mov rsi, 256
    int 0x80
    mov rdx, rax
    cmp rdx, 0
    jle .cat
    cmp rdx, 256
    jbe .ls_out
    mov rdx, 256
.ls_out:
    mov rax, 1
    mov rdi, 1
    mov rsi, listbuf
    int 0x80

.cat:
    ; > cat /test.txt
    mov rax, 1
    mov rdi, 1
    mov rsi, prompt_cat
    mov rdx, prompt_cat_end - prompt_cat
    int 0x80
    mov rax, 2
    mov rdi, path_test
    mov rsi, buf
    mov rdx, 64
    int 0x80
    mov rdx, rax
    cmp rdx, 0
    jle .do_exit
    cmp rdx, 64
    jbe .cat_out
    mov rdx, 64
.cat_out:
    mov rax, 1
    mov rdi, 1
    mov rsi, buf
    int 0x80
    mov rax, 1
    mov rdi, 1
    mov rsi, msg_nl
    mov rdx, 1
    int 0x80

    ; > exit
    mov rax, 1
    mov rdi, 1
    mov rsi, prompt_exit
    mov rdx, prompt_exit_end - prompt_exit
    int 0x80
.do_exit:
    mov rax, 60
    xor rdi, rdi
    int 0x80
.hang:
    hlt
    jmp .hang

align 16
msg_sh:
    db "[SH] Aether shell CPL=3", 10
msg_sh_end:
prompt_ls:
    db "> ls", 10
prompt_ls_end:
prompt_cat:
    db "> cat /test.txt", 10
prompt_cat_end:
prompt_exit:
    db "> exit", 10
prompt_exit_end:
msg_nl:
    db 10
path_test:
    db "/test.txt", 0
align 16
buf:
    times 64 db 0
listbuf:
    times 256 db 0
